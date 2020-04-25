use crate::{
    game::{nn_visual::KEY_ORDER, visual::VisGame, GAME_HEIGHT, GAME_WIDTH},
    neural_network::{ActivationType, NNCreationError, NNReadError, NNReadResult, NN},
    run_game,
};
use ggez::{
    event::{EventHandler, KeyMods},
    input::keyboard::KeyCode,
    Context, GameResult,
};
use itertools::Itertools;
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use tap::TapOps;

fn load_generation<P: AsRef<Path>>(path: &P) -> NNReadResult<Vec<NN>> {
    fs::read_to_string(path)?
        .split("--\n")
        .map(NN::from_string)
        .collect()
}

fn save_generation<P: AsRef<Path>>(path: &P, generation: &[NN]) -> io::Result<()> {
    fs::write(path, generation.iter().map(NN::to_string).join("--\n"))
}

pub struct NNTrainer {
    vis: VisGame,

    dir: PathBuf,
    generation: Vec<NN>,
    training: usize, // index
}

#[derive(From, Debug)]
pub enum NNReadOrCreationError {
    Read(NNReadError),
    Create(NNCreationError),
}

pub type NNReadOrCreateResult<T> = Result<T, NNReadOrCreationError>;

impl NNTrainer {
    #[allow(dead_code)]
    pub fn new(dir: &Path) -> NNReadOrCreateResult<Self> {
        let dir = PathBuf::from(".").tap(|pb| pb.push(dir));
        let generation = match load_generation(&dir) {
            Ok(gen) => gen,
            Err(_) => {
                let gen_size = 10;
                eprintln!(
                    "Warning: failed to load generation, creating a random one of size {}",
                    gen_size
                );
                (0..gen_size)
                    .map(|_| {
                        NN::make(GAME_WIDTH * GAME_HEIGHT)
                            .add_layer(20, ActivationType::Relu)
                            .add_layer(10, ActivationType::Relu)
                            .add_layer(7, ActivationType::Sigmoid)
                            .build()
                    })
                    .collect::<Result<_, _>>()?
            }
        };
        Ok(Self {
            vis: VisGame::new(),

            dir,
            generation,
            training: 0,
        })
    }

    #[allow(dead_code)]
    pub fn run(&mut self) -> GameResult<()> {
        run_game(self)
    }
}

impl EventHandler for NNTrainer {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let input = self.vis.game.get_cells();
        let nn_output = self.generation[self.training].apply(&input);
        // if manual control is on, this depends on the user, otherwise, it depends on the nn
        let manual_output = (0..7)
            .map(|i| self.vis.keys[&KEY_ORDER[i]].state.is_pressed())
            .collect::<Vec<_>>()
            .into_boxed_slice();

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
        } else if self.vis.paused && keycode == KeyCode::LControl {
            save_generation(&self.dir, &self.generation).expect("failed to save generation");
            println!("saved nn in \"{}\"", self.dir.display());
        }
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {
        if keycode == KeyCode::Escape {
            self.vis.key_up_event(ctx, keycode, keymods)
        }
    }
}
