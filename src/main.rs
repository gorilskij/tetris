#![feature(stmt_expr_attributes)] // for fine-grained rustfmt control

#[allow(unused_imports)]
use crate::game::nn_visual::NNVisGame;
#[allow(unused_imports)]
use crate::game::visual::VisGame;

use ggez::conf::{FullscreenType, WindowMode};
use ggez::event::EventHandler;
use ggez::{ContextBuilder, GameResult};

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
    VisGame::new().run().unwrap();
    // NNVisGame::new().run().unwrap();
}
