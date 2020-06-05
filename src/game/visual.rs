use crate::{
    game::{intersects_with, FallingPiece, Game, PieceId, Pixel, GAME_HEIGHT, GAME_WIDTH},
    run_game,
    support::sleep_until,
    HORIZONTAL_WINDOW_DIMS, HORIZONTAL_WINDOW_MODE, VERTICAL_WINDOW_DIMS, VERTICAL_WINDOW_MODE,
};
#[allow(unused_imports)]
use ggez::{
    event::{EventHandler, KeyMods},
    graphics,
    graphics::{
        clear, draw, draw_queued_text, present, queue_text, Color, DrawMode, DrawParam,
        FillOptions, FilterMode, MeshBuilder, Rect, Text, BLACK, WHITE,
    },
    input::keyboard::KeyCode,
    mint::Point2,
    Context, GameResult,
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[allow(unused_imports)]
use std::cmp::min;
#[allow(unused_imports)]
use tuple_map::*;

#[derive(Copy, Clone, Eq, PartialEq)]
enum Orientation {
    Horizontal,
    Vertical,
}

// fresh indicates the key was just pressed (with iterations left to wait)
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PressedState {
    Up,
    Down,
    Fresh(u8),
}

impl PressedState {
    pub fn is_pressed(self) -> bool {
        match self {
            PressedState::Up => false,
            _ => true,
        }
    }
}

enum Repeat {
    // initial_delay is number of 'delay's to wait
    // i.e. if delay is 3 frames and initial_delay is 2, initial delay will be 6 frames
    Repeat { initial_delay: u8, delay: u8 },
    NoRepeat,
}

pub struct KeyInfo {
    pub state: PressedState,
    repeat: Repeat,
}

pub type Keys = HashMap<KeyCode, KeyInfo>;

// trace_macros!(true);

macro_rules! keys {
    (@ins_key $keys:ident, $code:tt * ($initial_delay:expr, $delay:expr)) => {
        $keys.insert(
            KeyCode::$code,
            KeyInfo {
                state: PressedState::Up,
                repeat: Repeat::Repeat { initial_delay: $initial_delay, delay: $delay }
            },
        )
    };
    (@ins_key $keys:ident, $code:tt) => {
        $keys.insert(KeyCode::$code, KeyInfo { state: PressedState::Up, repeat: Repeat::NoRepeat })
    };
    ($( $code:tt $( * ($( $t:tt )*) )? ),* $(,)?) => {{
        let mut keys = Keys::new();
        $( keys!(@ins_key keys, $code $( * ($( $t )*) )?); )*
        keys
    }};
}

pub struct VisGame {
    pub game: Game,
    pub paused: bool,
    orientation: Orientation,
    next_frame: Instant,
    pub keys: Keys,
}

impl VisGame {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let keys = keys! {
            Left * (2, 4),
            Right * (2, 4),
            Down * (0, 3),
            Up, RShift, Space,
            J, Escape, Tab,
        };
        Self {
            game: Game::new(),
            paused: false,
            orientation: Orientation::Horizontal,
            next_frame: Instant::now(),
            keys,
        }
    }

    #[allow(dead_code)]
    pub fn run(&mut self) -> GameResult<()> {
        run_game(self)
    }
}

const LEFT_MARGIN: f32 = 10.;
const TOP_MARGIN: f32 = 10.;
const SPACE_BETWEEN: f32 = 30.; // hspace between graphic elements such as hold and board
const CELL_SIDE: f32 = 30.;

const PLAY_FPS: u64 = 60;
const PLAY_WAIT: Duration = Duration::from_millis(1000 / PLAY_FPS);
const PAUSE_FPS: u64 = 15;
const PAUSE_WAIT: Duration = Duration::from_millis(1000 / PAUSE_FPS);

impl VisGame {
    fn do_key_action(&mut self, code: KeyCode, ctx: &mut Context) {
        use KeyCode::*;
        match code {
            Left => self.game.move_falling_piece(-1, 0),
            Right => self.game.move_falling_piece(1, 0),
            Down => self.game.move_falling_piece(0, 1),
            Up => self.game.rotate_falling_piece(1),
            RShift => self.game.rotate_falling_piece(-1),
            Space => self.game.hard_drop(),
            J => self.game.switch_hold(),
            Tab => self.switch_orientation(ctx),
            Escape => self.paused = !self.paused,
            c => panic!("unexpected KeyCode: {:?}", c),
        }
    }
}

const MARGIN: f32 = 0.1;
const SIDE: f32 = CELL_SIDE - 2. * MARGIN;

// drawing
impl VisGame {
    fn add_piece_at(&self, (vis_x, vis_y): (f32, f32), id: PieceId, builder: &mut MeshBuilder) {
        let mask = self.game.mask_map[&id][0];
        for (rel_y, row) in mask.iter().enumerate() {
            for (rel_x, &val) in row.iter().enumerate() {
                if val {
                    let rect = Rect {
                        x: vis_x + rel_x as f32 * CELL_SIDE,
                        y: vis_y + rel_y as f32 * CELL_SIDE,
                        w: SIDE,
                        h: SIDE,
                    };
                    builder.rectangle(DrawMode::Fill(FillOptions::default()), rect, id.color());
                }
            }
        }
    }

    // return (bottom, right)
    fn add_hold(&mut self, builder: &mut MeshBuilder) -> (f32, f32) {
        // background
        let left = LEFT_MARGIN;
        let top = TOP_MARGIN;
        let width = (4. + 2.) * CELL_SIDE;
        let height = (1. * 3. + 2.) * CELL_SIDE;
        let bg_rect = Rect {
            x: left,
            y: top,
            w: width,
            h: height,
        };
        builder.rectangle(
            DrawMode::Fill(FillOptions::default()),
            bg_rect,
            Color::from_rgb(56, 56, 56),
        );
        // piece
        if let Some(id) = self.game.hold {
            let vis_x = left + CELL_SIDE;
            let vis_y = top + CELL_SIDE;
            // TODO: correct for non-centered pieces
            self.add_piece_at((vis_x, vis_y), id, builder)
        }
        (top + height, left + width)
    }

    // return (bottom, right)
    fn add_grid(
        &mut self,
        (left, top): (f32, f32),
        builder: &mut MeshBuilder,
    ) -> GameResult<(f32, f32)> {
        // not necessary because background is already black
        // let bg = Rect {
        //     x: left,
        //     y: top,
        //     w: GAME_WIDTH as f32 * CELL_SIDE,
        //     h: GAME_HEIGHT as f32 * CELL_SIDE,
        // };
        // builder.rectangle(DrawMode::fill(), bg, BLACK);
        let grid_color = Color::from_rgb(50, 50, 50);
        for rel_x in 0..=GAME_WIDTH {
            let abs_x = left + rel_x as f32 * CELL_SIDE;
            builder.line(
                &[
                    Point2 { x: abs_x, y: top },
                    Point2 {
                        x: abs_x,
                        y: top + GAME_HEIGHT as f32 * CELL_SIDE,
                    },
                ],
                1.,
                grid_color,
            )?;
        }
        for rel_y in 0..=GAME_HEIGHT {
            let abs_y = top + rel_y as f32 * CELL_SIDE;
            builder.line(
                &[
                    Point2 { x: left, y: abs_y },
                    Point2 {
                        x: left + GAME_WIDTH as f32 * CELL_SIDE,
                        y: abs_y,
                    },
                ],
                1.,
                grid_color,
            )?;
        }
        Ok((
            top + GAME_HEIGHT as f32 * CELL_SIDE,
            left + GAME_WIDTH as f32 * CELL_SIDE,
        ))
    }

    fn add_pixels(&mut self, (left, top): (f32, f32), builder: &mut MeshBuilder) {
        for (r, row) in self.game.board.iter().enumerate() {
            for (c, px) in row.iter().enumerate() {
                if let Pixel::Full(id) = px {
                    let left = left + c as f32 * CELL_SIDE + MARGIN;
                    let top = top + r as f32 * CELL_SIDE + MARGIN;
                    let rect = Rect {
                        x: left,
                        y: top,
                        w: SIDE,
                        h: SIDE,
                    };
                    builder.rectangle(DrawMode::Fill(FillOptions::default()), rect, id.color());
                }
            }
        }
    }

    fn add_shadow(
        (left, top): (f32, f32),
        falling: &FallingPiece,
        lowest_y: isize,
        builder: &mut MeshBuilder,
    ) -> GameResult<()> {
        for rel_y in 0..4 {
            for rel_x in 0..4 {
                if falling.mask[rel_y][rel_x] {
                    let abs_y = (rel_y as isize + lowest_y) as usize;
                    let abs_x = (rel_x as isize + falling.pos.0) as usize;
                    let vis_y = top + abs_y as f32 * CELL_SIDE;
                    let vis_x = left + abs_x as f32 * CELL_SIDE;

                    // each pixel outline
                    // let rect = Rect {
                    //     x: vis_x,
                    //     y: vis_y,
                    //     w: SIDE,
                    //     h: SIDE,
                    // };
                    // builder.rectangle(DrawMode::stroke(3.), rect, falling.id.color());

                    // fainter color (looks bad)
                    // let rgb = falling.id.color().to_rgb();
                    // let increase_possible = rgb.map(|x| 255. / x as f32);
                    // let min_increase_possible = increase_possible.tmin();
                    // let (r, g, b) = rgb.map(|x| {
                    //     let dx = (min_increase_possible * x as f32) as u8;
                    //     x + min(dx, 255 - x)
                    // });
                    // builder.rectangle(DrawMode::fill(), rect, Color::from_rgb(r, g, b));

                    // full block outline
                    let color = falling.id.color();
                    if rel_y == 0 || !falling.mask[rel_y - 1][rel_x] {
                        // top line
                        builder.line(
                            &[
                                Point2 { x: vis_x, y: vis_y },
                                Point2 {
                                    x: vis_x + SIDE,
                                    y: vis_y,
                                },
                            ],
                            3.,
                            color,
                        )?;
                    }
                    if rel_y == 3 || !falling.mask[rel_y + 1][rel_x] {
                        // bottom line
                        builder.line(
                            &[
                                Point2 {
                                    x: vis_x,
                                    y: vis_y + SIDE,
                                },
                                Point2 {
                                    x: vis_x + SIDE,
                                    y: vis_y + SIDE,
                                },
                            ],
                            3.,
                            color,
                        )?;
                    }
                    if rel_x == 0 || !falling.mask[rel_y][rel_x - 1] {
                        // left line
                        builder.line(
                            &[
                                Point2 { x: vis_x, y: vis_y },
                                Point2 {
                                    x: vis_x,
                                    y: vis_y + SIDE,
                                },
                            ],
                            3.,
                            color,
                        )?;
                    }
                    if rel_x == 3 || !falling.mask[rel_y][rel_x + 1] {
                        // right line
                        builder.line(
                            &[
                                Point2 {
                                    x: vis_x + SIDE,
                                    y: vis_y,
                                },
                                Point2 {
                                    x: vis_x + SIDE,
                                    y: vis_y + SIDE,
                                },
                            ],
                            3.,
                            color,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    fn add_falling(
        &mut self,
        (left, top): (f32, f32),
        builder: &mut MeshBuilder,
    ) -> GameResult<()> {
        if let Some(falling) = self.game.falling.as_ref() {
            let mask = falling.mask;
            let color;

            if falling.is_touching_ground(&self.game.board) {
                let lock_delay_ratio = self
                    .game
                    .falling
                    .as_ref()
                    .expect("can't draw a falling piece if there isn't one")
                    .lock_delay as f32
                    / FallingPiece::LOCK_DELAY as f32;
                let mut rgb = falling.id.color().to_rgb();
                rgb = rgb.map(|x| (x as f32 * lock_delay_ratio) as u8);
                color = Color::from(rgb);
            } else {
                color = falling.id.color();
                // shadow
                let lowest_y = (falling.pos.1 + 1..GAME_HEIGHT as isize)
                    .take_while(|&i| !intersects_with(&mask, (falling.pos.0, i), &self.game.board))
                    .last()
                    .expect("this should be Some, piece should not be touching ground");
                Self::add_shadow((left, top), falling, lowest_y, builder)?;
            }

            // piece
            for (rel_y, row) in mask.iter().enumerate() {
                for (rel_x, &val) in row.iter().enumerate() {
                    if val {
                        let abs_y = (rel_y as isize + falling.pos.1) as usize;
                        let abs_x = (rel_x as isize + falling.pos.0) as usize;
                        let vis_y = top + abs_y as f32 * CELL_SIDE;
                        let vis_x = left + abs_x as f32 * CELL_SIDE;
                        let rect = Rect {
                            x: vis_x,
                            y: vis_y,
                            w: SIDE,
                            h: SIDE,
                        };
                        builder.rectangle(DrawMode::Fill(FillOptions::default()), rect, color);
                    }
                }
            }
        }

        Ok(())
    }

    // return (bottom, right)
    fn add_queue(&mut self, (left, top): (f32, f32), builder: &mut MeshBuilder) -> (f32, f32) {
        // background
        let (width, height) = match self.orientation {
            // tall and thin / short and wide
            Orientation::Horizontal => ((4. + 2.) * CELL_SIDE, (4. * 3. + 5.) * CELL_SIDE),
            Orientation::Vertical => ((4. * 3. + 5.) * CELL_SIDE, (4. + 2.) * CELL_SIDE),
        };
        let bg_rect = Rect {
            x: left,
            y: top,
            w: width,
            h: height,
        };
        builder.rectangle(DrawMode::fill(), bg_rect, Color::from_rgb(56, 56, 56));
        // pieces
        match self.orientation {
            Orientation::Horizontal => {
                let x = left + CELL_SIDE;
                for (i, id) in self.game.piece_queue.iter().enumerate() {
                    let y = top + (i as f32 * 5. + (i + 1) as f32) * CELL_SIDE;
                    self.add_piece_at((x, y), id, builder);
                }
            }
            Orientation::Vertical => {
                let scale = 0.8;
                let y = top + scale * CELL_SIDE;
                for (i, id) in self.game.piece_queue.iter().enumerate() {
                    let x = left + scale * (i as f32 * 5. + (i + 1) as f32) * CELL_SIDE;
                    self.add_piece_at((x, y), id, builder);
                }
            }
        }

        (top + height, left + width)
    }

    fn add_text_info(
        &self,
        (left, top): (f32, f32),
        builder: &mut MeshBuilder,
        ctx: &mut Context,
    ) -> f32 {
        let (width, height) = match self.orientation {
            // tall-ish / wide-ish
            Orientation::Horizontal => (6. * CELL_SIDE, 10. * CELL_SIDE),
            Orientation::Vertical => (6. * CELL_SIDE, 6.5 * CELL_SIDE),
        };
        let bg_rect = Rect {
            x: left,
            y: top,
            w: width,
            h: height,
        };
        builder.rectangle(DrawMode::fill(), bg_rect, Color::from_rgb(56, 56, 56));
        let text_positions = match self.orientation {
            Orientation::Horizontal => (1..=4)
                .map(|i| Point2 {
                    x: left + CELL_SIDE,
                    y: top + i as f32 * CELL_SIDE,
                })
                .collect::<Vec<_>>(),
            Orientation::Vertical => (0..=3)
                .map(|i| Point2 {
                    x: left + CELL_SIDE,
                    y: top + (i as f32 + 0.5) * CELL_SIDE,
                })
                .collect::<Vec<_>>(),
        };

        macro_rules! queue_text {
            ($pos:expr, $( $fmt:expr ),*) => {
                queue_text(
                    ctx, &Text::new(format!($( $fmt ),*)), text_positions[$pos], Some(WHITE)
                );
            }
        }
        queue_text!(0, "{}", self.game.points);
        queue_text!(1, "Level {}", self.game.level);
        queue_text!(2, "Cleared {}", self.game.cleared);
        queue_text!(3, "fps {}", ggez::timer::fps(ctx) as u32);

        top + height
    }

    // return bottom
    fn add_keys(&self, (left, top): (f32, f32), builder: &mut MeshBuilder) -> f32 {
        let scale = match self.orientation {
            Orientation::Horizontal => 1.,
            Orientation::Vertical => 0.6, // 10 wide in a space of 6
        };
        let width = scale * (6. + 4.) * CELL_SIDE;
        let height = scale * (8. + 5.) * CELL_SIDE;
        let bg_rect = Rect {
            x: left,
            y: top,
            w: width,
            h: height,
        };
        builder.rectangle(DrawMode::fill(), bg_rect, Color::from_rgb(56, 56, 56));
        let mut key_bg = |x, y, rel_width, code| {
            let cells = rel_width * 3 - 1;
            let rect = Rect {
                x,
                y,
                w: scale * cells as f32 * CELL_SIDE,
                h: scale * 2. * CELL_SIDE,
            };
            builder.rectangle(
                DrawMode::fill(),
                rect,
                if self.keys[&code].state.is_pressed() {
                    Color::from_rgb(181, 45, 45)
                } else {
                    Color::from_rgb(102, 25, 25)
                },
            );
        };
        // up key
        key_bg(
            left + scale * 4. * CELL_SIDE,
            top + scale * CELL_SIDE,
            1,
            KeyCode::Up,
        );
        // down key
        key_bg(
            left + scale * 4. * CELL_SIDE,
            top + scale * 4. * CELL_SIDE,
            1,
            KeyCode::Down,
        );
        // left key
        key_bg(
            left + scale * CELL_SIDE,
            top + scale * 4. * CELL_SIDE,
            1,
            KeyCode::Left,
        );
        // right key
        key_bg(
            left + scale * 7. * CELL_SIDE,
            top + scale * 4. * CELL_SIDE,
            1,
            KeyCode::Right,
        );
        // hold key
        key_bg(
            left + scale * CELL_SIDE,
            top + scale * 7. * CELL_SIDE,
            1,
            KeyCode::J,
        );
        // rshift
        key_bg(
            left + scale * 4. * CELL_SIDE,
            top + scale * 7. * CELL_SIDE,
            2,
            KeyCode::RShift,
        );
        // spacebar
        key_bg(
            left + scale * CELL_SIDE,
            top + scale * 10. * CELL_SIDE,
            3,
            KeyCode::Space,
        );

        top + height
    }
}

// other
impl VisGame {
    fn switch_orientation(&mut self, ctx: &mut Context) {
        let dims = match self.orientation {
            Orientation::Horizontal => {
                self.orientation = Orientation::Vertical;
                graphics::set_mode(ctx, VERTICAL_WINDOW_MODE).unwrap();
                VERTICAL_WINDOW_DIMS
            }
            Orientation::Vertical => {
                self.orientation = Orientation::Horizontal;
                graphics::set_mode(ctx, HORIZONTAL_WINDOW_MODE).unwrap();
                HORIZONTAL_WINDOW_DIMS
            }
        };
        graphics::set_screen_coordinates(
            ctx,
            Rect {
                x: 0.,
                y: 0.,
                w: dims.0,
                h: dims.1,
            },
        )
        .unwrap()
    }
}

impl EventHandler for VisGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        sleep_until(self.next_frame);
        let start = Instant::now();

        if self.paused {
            self.next_frame = start + PAUSE_WAIT;
        } else {
            let mut actions = Vec::with_capacity(self.keys.len());
            for (&code, info) in self.keys.iter_mut() {
                if let Repeat::Repeat { delay, .. } = info.repeat {
                    if self.game.tick % delay as usize == 0 {
                        match info.state {
                            ref mut s @ PressedState::Fresh(0) | ref mut s @ PressedState::Down => {
                                actions.push(code);
                                *s = PressedState::Down;
                            }
                            PressedState::Fresh(ref mut x) => *x -= 1,
                            _ => (),
                        }
                    }
                }
            }
            for code in actions {
                self.do_key_action(code, ctx)
            }

            self.game.iterate();

            self.next_frame = start + PLAY_WAIT;
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        if self.paused {
            let (window_width, window_height) = match self.orientation {
                Orientation::Horizontal => HORIZONTAL_WINDOW_DIMS,
                Orientation::Vertical => VERTICAL_WINDOW_DIMS,
            };
            clear(ctx, Color::from_rgb(64, 64, 64));

            let mut builder = MeshBuilder::new();
            let _ = self.add_text_info((window_width / 2., window_height / 2.), &mut builder, ctx);
            draw_queued_text(ctx, DrawParam::default(), None, FilterMode::Linear)?;
        } else {
            clear(ctx, BLACK);

            let mut builder = MeshBuilder::new();
            // left quadrant
            let (hold_bottom, right) = self.add_hold(&mut builder);
            // main quadrant
            let pos = (right + SPACE_BETWEEN, TOP_MARGIN);
            let (bottom, right) = self.add_grid(pos, &mut builder)?;
            self.add_pixels(pos, &mut builder);
            self.add_falling(pos, &mut builder)?;
            // right or bottom quadrant
            match self.orientation {
                Orientation::Horizontal => {
                    let (_, right) =
                        self.add_queue((right + SPACE_BETWEEN, TOP_MARGIN), &mut builder);
                    let bottom =
                        self.add_text_info((right + SPACE_BETWEEN, TOP_MARGIN), &mut builder, ctx);
                    self.add_keys(
                        (right + SPACE_BETWEEN, bottom + SPACE_BETWEEN),
                        &mut builder,
                    );
                }
                Orientation::Vertical => {
                    self.add_queue((LEFT_MARGIN, bottom + SPACE_BETWEEN), &mut builder);
                    let bottom = self.add_keys(
                        (LEFT_MARGIN, hold_bottom + SPACE_BETWEEN / 2.),
                        &mut builder,
                    );
                    self.add_text_info(
                        (LEFT_MARGIN, bottom + SPACE_BETWEEN / 2.),
                        &mut builder,
                        ctx,
                    );
                }
            }
            // build and draw
            let mesh = builder.build(ctx)?;
            draw(ctx, &mesh, DrawParam::default())?;
            draw_queued_text(ctx, DrawParam::default(), None, FilterMode::Linear)?;
        }

        present(ctx)
    }

    fn key_down_event(&mut self, ctx: &mut Context, code: KeyCode, _mods: KeyMods, _: bool) {
        let mut do_action = false;
        self.keys.entry(code).and_modify(|key| {
            if key.state == PressedState::Up {
                key.state = match key.repeat {
                    Repeat::Repeat { initial_delay, .. } => PressedState::Fresh(initial_delay),
                    Repeat::NoRepeat => PressedState::Down,
                };
                do_action = true;
            }
        });
        if do_action {
            self.do_key_action(code, ctx)
        }
    }

    fn key_up_event(&mut self, _ctx: &mut Context, code: KeyCode, _mods: KeyMods) {
        self.keys.entry(code).and_modify(|v| {
            v.state = PressedState::Up;
        });
    }
}
