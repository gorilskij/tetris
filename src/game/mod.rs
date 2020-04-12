use crate::support::sleep_until;
use ggez::graphics::Color;
use itertools::Itertools;
use no_comment::IntoWithoutComments;
use rand::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Duration, Instant};

pub mod visual;

type Rotation = [[bool; 4]; 4];
type Rotations = [Rotation; 4];
type RotationMap = HashMap<PieceId, Rotations>;

struct FlyingPiece {
    id: PieceId,
    pos: (isize, isize), // top-left corner
    rotation_idx: usize,
    rotation: Rotation, // cached
}

fn intersects_with(mask: &Rotation, pos: (isize, isize), board: &Board) -> bool {
    for rel_x in 0..4 {
        for rel_y in 0..4 {
            if mask[rel_y][rel_x] {
                let abs_x = rel_x as isize + pos.0;
                let abs_y = rel_y as isize + pos.1;
                if abs_x < 0
                    || abs_x >= GAME_WIDTH as isize
                    || abs_y < 0
                    || abs_y >= GAME_HEIGHT as isize
                {
                    return true;
                } else {
                    if let Pixel::Full(_) = board[abs_y as usize][abs_x as usize] {
                        return true;
                    }
                }
            }
        }
    }
    false
}

impl FlyingPiece {
    // ground is positive y!
    fn is_touching_ground(&self, board: &Board) -> bool {
        intersects_with(
            &self.rotation,
            (self.pos.0 as isize, self.pos.1 as isize + 1),
            board,
        )
    }

    fn print_onto(&self, board: &mut Board) {
        for rel_x in 0..4 {
            for rel_y in 0..4 {
                if self.rotation[rel_y][rel_x] {
                    let abs_x = (self.pos.0 + rel_x as isize) as usize;
                    let abs_y = (self.pos.1 + rel_y as isize) as usize;
                    // this check might be useless if collision checking is already implemented...
                    match &mut board[abs_y][abs_x] {
                        c @ Pixel::Empty => *c = Pixel::Full(self.id),
                        Pixel::Full(_) => panic!(
                            "intersected with board while printing onto it at abs (x, y) == ({}, {})",
                            abs_x, abs_y,
                        ),
                    }
                }
            }
        }
    }
}

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub enum PieceId {
    IBlock,
    JBlock,
    LBlock,
    OBlock,
    SBlock,
    TBlock,
    ZBlock,
}

impl PieceId {
    const ALL: &'static [Self] = &[
        Self::IBlock,
        Self::JBlock,
        Self::LBlock,
        Self::OBlock,
        Self::SBlock,
        Self::TBlock,
        Self::ZBlock,
    ];

    pub fn color(self) -> Color {
        use PieceId::*;
        match self {
            IBlock => Color::from_rgb(88, 176, 188),
            JBlock => Color::from_rgb(22, 101, 167),
            LBlock => Color::from_rgb(217, 133, 1),
            OBlock => Color::from_rgb(235, 214, 1),
            SBlock => Color::from_rgb(55, 154, 48),
            TBlock => Color::from_rgb(137, 64, 135),
            ZBlock => Color::from_rgb(205, 12, 17),
        }
    }
}

pub fn parse_rotations_file<P: AsRef<Path>>(path: P) -> RotationMap {
    let file = File::open(path).expect("failed to open rotations file");

    let text = BufReader::new(file)
        .lines()
        .map(|l| l.expect("failed to read line"))
        .join("\n");

    let text: String = text
        .chars()
        .without_comments()
        .skip_while(|c| c.is_whitespace())
        .collect();

    let iter = &mut text.chars();

    let mut map = RotationMap::new();
    for _ in 0..7 {
        // also gets and drops '\n'
        let name: String = iter.take_while(|c| c.is_alphabetic()).collect();
        use PieceId::*;
        let name = match name.as_str() {
            "IBlock" => IBlock,
            "JBlock" => JBlock,
            "LBlock" => LBlock,
            "OBlock" => OBlock,
            "SBlock" => SBlock,
            "TBlock" => TBlock,
            "ZBlock" => ZBlock,
            n => panic!("unexpected piece name \"{}\"", n),
        };

        // 4 rotations, 4 lines, 4 values
        let mut rotations = [[[false; 4]; 4]; 4];
        for r in 0..4 {
            for l in 0..4 {
                let line: String = iter.take_while(|&c| c != '\n').collect();
                for (v, c) in line.split("  ").enumerate() {
                    rotations[r][l][v] = match c {
                        "." => false,
                        "0" => true,
                        c => panic!("unexpected '{}'", c),
                    }
                }
            }
            // drop empty line
            iter.take_while(|&c| c != '\n').for_each(|_| {});
        }

        map.insert(name, rotations);
    }

    map
}

#[derive(Copy, Clone)]
enum Pixel {
    Empty,
    Full(PieceId),
}

impl Pixel {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Empty => true,
            Self::Full(_) => false,
        }
    }
}

pub const GAME_WIDTH: usize = 10;
pub const GAME_HEIGHT: usize = 20;

// 20 rows of 10 pixels
type Board = [[Pixel; GAME_WIDTH]; GAME_HEIGHT];

pub struct Game {
    rng: ThreadRng,
    current_bag: Vec<PieceId>,

    flying: Option<FlyingPiece>,
    rotation_map: RotationMap,

    board: Board,

    tick: usize, // frame tick tied to fps (== number of vis frames)
}

impl Game {
    pub fn new() -> Self {
        let mut board = [[Pixel::Empty; 10]; 20];
        Self {
            rng: thread_rng(),
            current_bag: Vec::with_capacity(7),

            flying: None,
            rotation_map: parse_rotations_file("rotations.txt"),

            board,

            tick: 0,
        }
    }

    fn spawn(&mut self) {
        if self.current_bag.is_empty() {
            self.current_bag.extend_from_slice(PieceId::ALL)
        }

        let idx = self.rng.gen_range(0, self.current_bag.len());
        let id = self.current_bag.remove(idx);

        self.flying = Some(FlyingPiece {
            id,
            pos: (GAME_WIDTH as isize / 2 - 2 /* width is 4 */, 0),
            rotation_idx: 0,
            rotation: self.rotation_map[&id][0],
        });
    }

    pub fn iterate(&mut self) {
        if self.tick % 15 == 0 {
            // every 15 frames
            if let Some(ref mut flying) = self.flying {
                if flying.is_touching_ground(&self.board) {
                    flying.print_onto(&mut self.board);
                    self.flying = None;
                } else {
                    flying.pos.1 += 1;
                }
            } else {
                self.spawn()
            }
        }

        // compact board
        let mut shift_up = 0; // shift towards ground (positive-y)
        for y in (0..GAME_HEIGHT).rev() {
            if self.board[y].iter().all(|px| !px.is_empty()) {
                shift_up += 1;
            } else if shift_up > 0 {
                self.board[y + shift_up] = self.board[y]
            }
        }

        self.tick += 1;
    }
}

// control
impl Game {
    pub fn teleport_flying_piece(&mut self, dx: isize, dy: isize) {
        if let Some(ref mut flying) = self.flying {
            let mask = &self.rotation_map[&flying.id][flying.rotation_idx];
            let new_pos = (flying.pos.0 as isize + dx, flying.pos.1 as isize + dy);
            if !intersects_with(
                mask,
                new_pos,
                &self.board,
            ) {
                flying.pos = new_pos;
            }
        }
    }

    pub fn rotate_flying_piece(&mut self, di: isize) { // +1 is 90° clockwise, -1 is 90° counterclockwise
        if let Some(ref mut flying) = self.flying {
            let new_idx = ((flying.rotation_idx as isize + di % 4 + 4) % 4) as usize;
            let new_rotation = self.rotation_map[&flying.id][new_idx];
            if !intersects_with(&new_rotation, (flying.pos.0 as isize, flying.pos.1 as isize), &self.board) {
                flying.rotation_idx = new_idx;
                flying.rotation = new_rotation;
            }
        }
    }
}
