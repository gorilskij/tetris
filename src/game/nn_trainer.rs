use crate::game::nn_visual::KEY_ORDER;
use crate::game::visual::{PressedState, VisGame};
use crate::neural_network::{NNReadResult, NN};
use crate::run_game;
use ggez::event::{EventHandler, KeyMods};
use ggez::input::keyboard::KeyCode;
use ggez::{Context, GameResult};
use std::path::Path;

pub struct NNTrainer<'a> {
    file_path: &'a Path,
    vis: VisGame,
    nn: NN,
    manual_control: bool,
}

impl<'a> NNTrainer<'a> {
    #[allow(dead_code)]
    pub fn new(file_path: &'a Path) -> NNReadResult<Self> {
        let nn = NN::read_in(file_path)?;
        Ok(Self {
            file_path,
            vis: VisGame::new(),
            nn,
            manual_control: true,
        })
    }

    #[allow(dead_code)]
    pub fn run(&mut self) -> GameResult<()> {
        run_game(self)
    }
}

impl EventHandler for NNTrainer<'_> {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let input = self.vis.game.get_cells();
        let nn_output = self.nn.apply(&input);
        // if manual control is on, this depends on the user, otherwise, it depends on the nn
        let manual_output = (0..7)
            .map(|i| self.vis.keys[&KEY_ORDER[i]].state.is_pressed())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        if self.manual_control {
            // learn
        } else {
            for ((&nn_out, &is_pressed), &code) in nn_output
                .iter()
                .zip(manual_output.iter())
                .zip(KEY_ORDER.iter())
            {
                let should_be_pressed = nn_out > 0.5;
                if is_pressed && !should_be_pressed {
                    self.vis.key_up_event(ctx, code, KeyMods::default())
                } else if !is_pressed && should_be_pressed {
                    self.vis
                        .key_down_event(ctx, code, KeyMods::default(), false)
                }
            }
        }

        self.vis.update(ctx)
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.vis.draw(ctx)
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        keymods: KeyMods,
        repeat: bool,
    ) {
        if keycode == KeyCode::Escape {
            self.vis.key_down_event(ctx, keycode, keymods, repeat);
        } else if self.vis.paused {
            if keycode == KeyCode::LShift {
                self.manual_control = !self.manual_control;
                // clear keys
                for (_, k) in self.vis.keys.iter_mut() {
                    k.state = PressedState::Up
                }
                println!(
                    "control: {}",
                    if self.manual_control {
                        "manual"
                    } else {
                        "neural"
                    }
                );
            } else if keycode == KeyCode::LControl {
                self.nn.write_out(self.file_path).unwrap();
                println!("saved nn in \"{}\"", self.file_path.display());
            }
        } else if self.manual_control {
            self.vis.key_down_event(ctx, keycode, keymods, repeat);
        } else {
            println!("warning: key_down ignored because under neural control");
        }
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {
        if keycode == KeyCode::Escape || self.manual_control {
            self.vis.key_up_event(ctx, keycode, keymods)
        } else {
            println!("warning: manual key_up ignored")
        }
    }
}
