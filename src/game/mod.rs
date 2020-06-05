use ggez::graphics::Color;
use itertools::Itertools;
use no_comment::IntoWithoutComments;
use rand::prelude::*;
use std::{
    cmp::{max, min},
    collections::{HashMap, VecDeque},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};
use tap::TapOps;

pub(crate) mod nn_trainer;
pub mod nn_visual;
pub mod visual;

type Mask = [[bool; 4]; 4];
type Masks = [Mask; 4];
type MaskMap = HashMap<PieceId, Masks>;

struct FallingPiece {
    id: PieceId,
    pos: (isize, isize), // top-left corner
    mask_idx: usize,
    mask: Mask, // cached

    lock_delay: u8,
    lock_delay_resets: u8,
}

// check whether the given mask at the given position intersects with any elements of the board
// such as pixels or borders
fn intersects_with(mask: &Mask, (x, y): (isize, isize), board: &Board) -> bool {
    for (rel_y, row) in mask.iter().enumerate() {
        for (rel_x, &val) in row.iter().enumerate() {
            if val {
                let abs_x = rel_x as isize + x;
                let abs_y = rel_y as isize + y;
                if abs_x < 0
                    || abs_x >= GAME_WIDTH as isize
                    || abs_y < 0
                    || abs_y >= GAME_HEIGHT as isize
                {
                    return true;
                } else if let Pixel::Full(_) = board[abs_y as usize][abs_x as usize] {
                    return true;
                }
            }
        }
    }
    false
}

impl FallingPiece {
    // ground is positive y!
    fn is_touching_ground(&self, board: &Board) -> bool {
        intersects_with(
            &self.mask,
            (self.pos.0 as isize, self.pos.1 as isize + 1),
            board,
        )
    }

    fn print_onto(&self, board: &mut Board) {
        for rel_x in 0..4 {
            for rel_y in 0..4 {
                if self.mask[rel_y][rel_x] {
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

    const LOCK_DELAY: u8 = 5;

    // will only reset lock delay if the piece is already counting down
    // and there are resets left
    fn checked_reset_lock_delay(&mut self) {
        if self.lock_delay < Self::LOCK_DELAY && self.lock_delay_resets > 0 {
            self.lock_delay = Self::LOCK_DELAY;
            self.lock_delay_resets -= 1;
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

pub fn load_masks<P: AsRef<Path>>(path: P) -> MaskMap {
    let path = path.as_ref();
    let file = File::open(path).unwrap_or_else(|_| panic!("failed to open \"{}\"", path.display()));

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

    let mut map = MaskMap::new();
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

        // 4 masks, 4 lines, 4 values
        let mut masks = [[[false; 4]; 4]; 4];
        for mask in masks.iter_mut() {
            for line in mask.iter_mut() {
                let l: String = iter.take_while(|&c| c != '\n').collect();
                for (i, c) in l.split("  ").enumerate() {
                    line[i] = match c {
                        "." => false,
                        "0" => true,
                        c => panic!("unexpected '{}'", c),
                    }
                }
            }
            // drop empty line
            iter.take_while(|&c| c != '\n').for_each(|_| {});
        }

        map.insert(name, masks);
    }

    map
}

#[derive(Copy, Clone)]
enum Pixel {
    Empty,
    Full(PieceId),
}

impl Pixel {
    pub fn is_empty(self) -> bool {
        match self {
            Self::Empty => true,
            Self::Full(_) => false,
        }
    }
}

pub struct PieceQueue {
    rng: ThreadRng,
    bag: Vec<PieceId>,
    queue: VecDeque<PieceId>,
}

impl PieceQueue {
    fn pop_from_bag(rng: &mut ThreadRng, bag: &mut Vec<PieceId>) -> PieceId {
        if bag.is_empty() {
            bag.extend_from_slice(PieceId::ALL)
        }
        let idx = rng.gen_range(0, bag.len());
        bag.remove(idx)
    }

    fn new() -> Self {
        let mut rng = thread_rng();
        let mut bag = Vec::with_capacity(7);
        let mut queue = VecDeque::with_capacity(3);
        for _ in 0..3 {
            queue.push_back(Self::pop_from_bag(&mut rng, &mut bag))
        }
        Self { rng, bag, queue }
    }

    fn pop(&mut self) -> PieceId {
        let out = self.queue.pop_front().unwrap();
        self.queue
            .push_back(Self::pop_from_bag(&mut self.rng, &mut self.bag));
        out
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = PieceId> + 'a {
        self.queue.iter().copied()
    }
}

pub const GAME_WIDTH: usize = 10;
pub const GAME_HEIGHT: usize = 20;

// 20 rows of 10 pixels
type Board = [[Pixel; GAME_WIDTH]; GAME_HEIGHT];

pub struct Game {
    mask_map: MaskMap,
    tick: usize, // frame tick tied to fps (== number of vis frames)
    points: usize,
    level: usize,
    cleared: usize, // number of rows cleared so far

    board: Board,
    piece_queue: PieceQueue,
    falling: Option<FallingPiece>,
    hold: Option<PieceId>,
    can_switch: bool, // to prevent double-switching hold
}

impl Game {
    pub fn new() -> Self {
        let board = [[Pixel::Empty; 10]; 20];
        Self {
            mask_map: load_masks("masks.txt"),
            tick: 0,
            points: 0,
            level: 1,
            cleared: 0,

            board,
            piece_queue: PieceQueue::new(),
            falling: None,
            hold: None,
            can_switch: true,
        }
        .tap(Game::spawn)
    }

    // return concatenated rows of cells, includes falling piece
    pub fn get_cells(&self) -> Box<[f64]> {
        // board
        let mut cells = self
            .board
            .iter()
            .flat_map(|row| row.iter().map(|px| if px.is_empty() { 0. } else { 1. }))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        // falling piece
        if let Some(falling) = &self.falling {
            let mask = falling.mask;
            for (rel_y, rel_x) in (0..4).cartesian_product(0..4) {
                if mask[rel_y][rel_x] {
                    let abs_y = rel_y as isize + falling.pos.1;
                    let abs_x = rel_x as isize + falling.pos.0;
                    cells[abs_y as usize * GAME_WIDTH + abs_x as usize] = 1.;
                }
            }
        }

        cells
    }

    fn lose(&self) {
        panic!(
            "Lost {{ points: {}, level: {}, cleared: {} }}",
            self.points, self.level, self.cleared
        )
    }

    fn spawn_with_id(&mut self, id: PieceId) {
        let pos = (GAME_WIDTH as isize / 2 - 2 /* width is 4 */, 0);
        let mask_idx = 0;
        let mask = self.mask_map[&id][mask_idx];

        if intersects_with(&mask, pos, &self.board) {
            self.lose()
        } else {
            self.falling = Some(FallingPiece {
                id,
                pos,
                mask_idx,
                mask,
                lock_delay: FallingPiece::LOCK_DELAY,
                lock_delay_resets: 10,
            })
        }
    }

    fn spawn(&mut self) {
        let id = self.piece_queue.pop();
        self.spawn_with_id(id)
    }

    // print falling piece onto the board and destroy it (will be spawned next iteration)
    fn destroy_falling_and_respawn(&mut self) {
        self.falling.as_mut().unwrap().print_onto(&mut self.board);
        self.falling = None;
        self.can_switch = true;
        self.spawn();
    }

    // might get called twice but that shouldn't matter
    // also does scoring
    fn compact_board(&mut self) {
        let mut shift_up = 0; // shift towards ground (positive-y)
        for y in (0..GAME_HEIGHT).rev() {
            if self.board[y].iter().all(|px| !px.is_empty()) {
                shift_up += 1;
            } else if shift_up > 0 {
                self.board[y + shift_up] = self.board[y];
                self.board[y] = [Pixel::Empty; GAME_WIDTH];
            }
        }
        // at this point shift_up == number of rows cleared
        self.cleared += shift_up;
        // level goes up every ten lines
        self.level = (self.cleared / 10) + 1;
        self.points += self.level
            * match shift_up {
                0 => 0,
                1 => 40,
                2 => 100,
                3 => 300,
                4 => 1200,
                n => panic!("unexpected {} lines cleared", n),
            }
    }

    pub fn iterate(&mut self) {
        self.compact_board();

        // rows to fall per frame, assumes 60 fps (levels 1-15+)
        const ROWS_PER_FRAME: [f32; 15] = #[rustfmt::skip] [
            0.01667,
            0.021_017,
            0.026_977,
            0.035_256,
            0.04693,
            0.06361,
            0.0879,
            0.1236,
            0.1775,
            0.2598,
            0.388,
            0.59,
            0.92,
            1.46,
            2.36,
        ];

        let rows_per_frame = ROWS_PER_FRAME[min(self.level, 15) - 1];
        let frames_per_row = max(1, (1. / rows_per_frame) as _);

        // every 15 frames iterate falling piece
        if self.tick % frames_per_row == 0 {
            if let Some(ref mut falling) = self.falling {
                if falling.is_touching_ground(&self.board) {
                    if falling.lock_delay == 0 {
                        self.destroy_falling_and_respawn();
                    } else {
                        falling.lock_delay -= 1;
                    }
                } else {
                    falling.pos.1 += 1;
                }
            } else {
                panic!("no falling piece")
            }
        }

        self.tick += 1;
    }
}

// control
impl Game {
    pub fn move_falling_piece(&mut self, dx: isize, dy: isize) {
        if let Some(ref mut falling) = self.falling {
            let mask = &self.mask_map[&falling.id][falling.mask_idx];
            let new_pos = (falling.pos.0 as isize + dx, falling.pos.1 as isize + dy);
            if !intersects_with(mask, new_pos, &self.board) {
                falling.pos = new_pos;
                falling.checked_reset_lock_delay();
            }
        } else {
            panic!("tried to move with no falling piece")
        }
    }

    pub fn rotate_falling_piece(&mut self, di: isize) {
        // +1 is 90° clockwise, -1 is 90° counterclockwise
        if let Some(ref mut falling) = self.falling {
            let new_idx = ((falling.mask_idx as isize + di % 4 + 4) % 4) as usize;
            let new_mask = self.mask_map[&falling.id][new_idx];
            // sometimes it's necessary to shift a bit when rotating, this is so
            // that rotation isn't blocked when touching the ground or next to a wall
            let mut success = false;
            for (dx, dy) in #[rustfmt::skip] &[
                    (0, 0),
                    (0, -1), (0, -2), // up
                    (0, 1), (0, 2), // down
                    (-1, 0), (-2, 0), // left
                    (1, 0), (2, 0), // right
                ]
            {
                let pos = (falling.pos.0 + dx, falling.pos.1 + dy);
                if !intersects_with(&new_mask, pos, &self.board) {
                    falling.pos = pos;
                    falling.mask_idx = new_idx;
                    falling.mask = new_mask;
                    success = true;
                    break;
                }
            }
            if success {
                falling.checked_reset_lock_delay();
            }
        } else {
            panic!("tried to rotate with no falling piece")
        }
    }

    // does scoring
    pub fn hard_drop(&mut self) {
        self.compact_board();
        if self.falling.is_none() {
            // self.spawn();
            panic!("attempted to hard drop with no falling piece")
        }
        let falling = self.falling.as_mut().unwrap();
        let mask = &falling.mask;
        let pos = falling.pos;
        let mut delta = 0;
        while !intersects_with(mask, (pos.0, pos.1 + delta as isize + 1), &self.board) {
            delta += 1
        }
        falling.pos = (pos.0, pos.1 + delta as isize);
        self.destroy_falling_and_respawn();
        self.points += delta + 1;
    }

    pub fn switch_hold(&mut self) {
        if self.can_switch {
            self.can_switch = false;
            let old = self.hold.take();
            self.hold = Some(
                self.falling
                    .take()
                    .map(|fp| fp.id)
                    .expect("tried to swap with no falling piece"),
            );
            if let Some(id) = old {
                self.spawn_with_id(id)
            } else {
                self.spawn()
            }
        }
    }
}
