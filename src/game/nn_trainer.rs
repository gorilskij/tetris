use crate::game::nn_visual::KEY_ORDER;
use crate::game::visual::{PressedState, VisGame};
use crate::game::{GAME_HEIGHT, GAME_WIDTH};
use crate::neural_network::{ActivationType, NN};
use crate::run_game;
use ggez::event::{EventHandler, KeyMods};
use ggez::input::keyboard::KeyCode;
use ggez::{Context, GameResult};

pub struct NNTrainer {
    vis: VisGame,
    nn: NN,
    manual_control: bool,
}

impl NNTrainer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            vis: VisGame::new(),
            // all cells as input, 7 keys as output
            nn: NN::make(GAME_WIDTH * GAME_HEIGHT)
                .add_layer(20, ActivationType::Relu)
                .add_layer(10, ActivationType::Relu)
                .add_layer(7, ActivationType::Sigmoid)
                .build()
                .unwrap(),
            manual_control: true,
        }
    }

    #[allow(dead_code)]
    pub fn run(&mut self) -> GameResult<()> {
        run_game(self)
    }
}

impl EventHandler for NNTrainer {
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
        if keycode == KeyCode::LShift {
            self.manual_control = !self.manual_control;
            // clear keys
            for (_, k) in self.vis.keys.iter_mut() {
                k.state = PressedState::Up
            }
        } else if self.manual_control {
            self.vis.key_down_event(ctx, keycode, keymods, repeat)
        } else {
            println!("warning: manual key_down ignored")
        }
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {
        if self.manual_control {
            self.vis.key_up_event(ctx, keycode, keymods)
        } else {
            println!("warning: manual key_up ignored")
        }
    }
}
