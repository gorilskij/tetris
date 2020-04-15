#![feature(stmt_expr_attributes)] // for fine-grained rustfmt control

use crate::game::visual::VisGame;

pub mod game;
mod neural_network;
mod support;

fn main() {
    VisGame::run();
}
