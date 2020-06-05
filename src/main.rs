#![feature(stmt_expr_attributes)] // for fine-grained rustfmt control

#[macro_use]
extern crate derive_more;

#[allow(unused_imports)]
use crate::game::nn_trainer::NNTrainer;
#[allow(unused_imports)]
use crate::game::nn_visual::NNVisGame;
#[allow(unused_imports)]
use crate::game::visual::VisGame;

#[allow(unused_imports)]
use crate::game::{GAME_HEIGHT, GAME_WIDTH};
#[allow(unused_imports)]
use crate::neural_network::{ActivationType, NNReadResult, NN};
use ggez::{
    conf::{FullscreenType, WindowMode},
    event::EventHandler,
    ContextBuilder, GameResult,
};

pub(crate) mod game;
pub(crate) mod neural_network;
mod support;

const HORIZONTAL_WINDOW_DIMS: (f32, f32) = (1150., 750.);
const VERTICAL_WINDOW_DIMS: (f32, f32) = (550., 850.);

const HORIZONTAL_WINDOW_MODE: WindowMode = WindowMode {
    width: HORIZONTAL_WINDOW_DIMS.0,
    height: HORIZONTAL_WINDOW_DIMS.1,
    maximized: false,
    fullscreen_type: FullscreenType::Windowed,
    borderless: false,
    min_width: 0.0,
    max_width: 0.0,
    min_height: 0.0,
    max_height: 0.0,
    resizable: false,
};

const VERTICAL_WINDOW_MODE: WindowMode = WindowMode {
    width: VERTICAL_WINDOW_DIMS.0,
    height: VERTICAL_WINDOW_DIMS.1,
    maximized: false,
    fullscreen_type: FullscreenType::Windowed,
    borderless: false,
    min_width: 0.0,
    max_width: 0.0,
    min_height: 0.0,
    max_height: 0.0,
    resizable: false,
};

// todo try to factor out this function
pub fn run_game(eh: &mut impl EventHandler) -> GameResult<()> {
    let (ref mut ctx, ref mut event_loop) = ContextBuilder::new("my_game", "me")
        .window_mode(HORIZONTAL_WINDOW_MODE)
        .build()
        .expect("failed to create context");

    ggez::event::run(ctx, event_loop, eh)
}

fn main() {
    // playable game
    VisGame::new().run().unwrap();

    // NNVisGame::new().run().unwrap();

    // NNTrainer::new("data/saved_gen.txt".as_ref())
    //     .expect("failed to create nn_trainer")
    //     .run()
    //     .unwrap()
}
