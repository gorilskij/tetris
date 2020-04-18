#![feature(stmt_expr_attributes)] // for fine-grained rustfmt control

use crate::game::visual::VisGame;
use crate::neural_network::NN;
use crate::game::nn_visual::NNVisGame;
use ggez::event::EventHandler;
use ggez::{GameResult, ContextBuilder};
use ggez::conf::{WindowMode, FullscreenType};

pub(crate) mod game;
pub(crate) mod neural_network;
mod support;

const WINDOW_WIDTH: f32 = 1150.;
const WINDOW_HEIGHT: f32 = 650.;

pub fn run_game(eh: &mut impl EventHandler) -> GameResult<()> {
    let window_mode = WindowMode {
        width: WINDOW_WIDTH,
        height: WINDOW_HEIGHT,
        maximized: false,
        fullscreen_type: FullscreenType::Windowed,
        borderless: false,
        min_width: 0.0,
        max_width: 0.0,
        min_height: 0.0,
        max_height: 0.0,
        resizable: false,
    };

    let (ref mut ctx, ref mut event_loop) = ContextBuilder::new("my_game", "me")
        .window_mode(window_mode)
        .build()
        .expect("failed to create context");

    ggez::event::run(ctx, event_loop, eh)
}

fn main() {
    // VisGame::new().run().unwrap();
    NNVisGame::new().run().unwrap();
    // let nn = NN::new(&[4, 10, 6]);
    // let out = nn.apply(&[1., 2., 3., 4.]);
    // println!("{:?}", out);
}
