use crate::game::visual::VisGame;
use crate::game::{parse_rotations_file, PieceId};

mod game;
mod support;

fn main() {
    VisGame::run();
}
