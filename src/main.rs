#![feature(stmt_expr_attributes)] // for fine-grained rustfmt control

use crate::game::visual::VisGame;
use crate::neural_network::NN;

pub mod game;
mod neural_network;
mod support;

fn main() {
    // VisGame::run();
    let nn = NN::new(&[4, 10, 6]);
    let out = nn.apply(&[1., 2., 3., 4.]);
    println!("{:?}", out);
}
