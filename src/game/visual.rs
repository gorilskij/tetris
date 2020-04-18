use crate::game::{intersects_with, Game, PieceId, Pixel, GAME_HEIGHT, GAME_WIDTH};
use crate::support::sleep_until;
use ggez::event::{EventHandler, KeyMods};
use ggez::graphics::{
    clear, draw, draw_queued_text, present, queue_text, Color, DrawMode, DrawParam, FillOptions,
    FilterMode, MeshBuilder, Rect, Text, BLACK, WHITE,
};
use ggez::input::keyboard::KeyCode;
use ggez::mint::Point2;
use ggez::{Context, GameResult};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::{run_game, WINDOW_HEIGHT, WINDOW_WIDTH};
#[allow(unused_imports)]
use std::cmp::min;
#[allow(unused_imports)]
use tuple_map::*;

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

pub struct VisGame {
    pub game: Game,
    paused: bool,
    next_frame: Instant,
    pub keys: Keys,
}

impl VisGame {
    pub fn new() -> Self {
        let mut keys = Keys::new();
        keys.insert(
            KeyCode::Left,
            KeyInfo {
                state: PressedState::Up,
                repeat: Repeat::Repeat {
                    initial_delay: 2,
                    delay: 4,
                },
            },
        );
        keys.insert(
            KeyCode::Right,
            KeyInfo {
                state: PressedState::Up,
                repeat: Repeat::Repeat {
                    initial_delay: 2,
                    delay: 4,
                },
            },
        );
        keys.insert(
            KeyCode::Down,
            KeyInfo {
                state: PressedState::Up,
                repeat: Repeat::Repeat {
                    initial_delay: 0,
                    delay: 4,
                },
            },
        );
        keys.insert(
            KeyCode::Up,
            KeyInfo {
                state: PressedState::Up,
                repeat: Repeat::NoRepeat,
            },
        );
        keys.insert(
            KeyCode::RShift,
            KeyInfo {
                state: PressedState::Up,
                repeat: Repeat::NoRepeat,
            },
        );
        keys.insert(
            KeyCode::Space,
            KeyInfo {
                state: PressedState::Up,
                repeat: Repeat::NoRepeat,
            },
        );
        keys.insert(
            KeyCode::J,
            KeyInfo {
                state: PressedState::Up,
                repeat: Repeat::NoRepeat,
            },
        );
        keys.insert(
            KeyCode::Escape,
            KeyInfo {
                state: PressedState::Up,
                repeat: Repeat::NoRepeat,
            },
        );
        Self {
            game: Game::new(),
            paused: false,
            next_frame: Instant::now(),
            keys,
        }
    }

    pub fn run(&mut self) -> GameResult<()> {
        run_game(self)
    }
}

const LEFT_MARGIN: f32 = 10.;
const TOP_MARGIN: f32 = 10.;
const SPACE_BETWEEN: f32 = 30.; // hspace between graphic elements such as hold and board
const CELL_SIDE: f32 = 30.;

const FPS: u64 = 60;
const WAIT: Duration = Duration::from_millis(1000 / FPS);

impl VisGame {
    // unconditional
    fn do_key_action(&mut self, code: KeyCode) {
        match code {
            KeyCode::Left => self.game.teleport_flying_piece(-1, 0),
            KeyCode::Right => self.game.teleport_flying_piece(1, 0),
            KeyCode::Down => self.game.teleport_flying_piece(0, 1),
            KeyCode::Up => self.game.rotate_flying_piece(1),
            KeyCode::RShift => self.game.rotate_flying_piece(-1),
            KeyCode::Space => self.game.hard_drop(),
            KeyCode::J => self.game.switch_hold(),
            KeyCode::Escape => self.paused = !self.paused,
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

    // return right
    fn add_hold(&mut self, builder: &mut MeshBuilder) -> f32 {
        // background
        let left = LEFT_MARGIN;
        let top = TOP_MARGIN;
        let width = (4. + 2.) * CELL_SIDE;
        let bg_rect = Rect {
            x: left,
            y: top,
            w: width,
            h: (1. * 3. + 2.) * CELL_SIDE,
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
            self.add_piece_at((vis_x, vis_y), id, builder)
        }
        left + width
    }

    // return right
    fn add_grid(&mut self, (left, top): (f32, f32), builder: &mut MeshBuilder) -> GameResult<f32> {
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
                BLACK,
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
                BLACK,
            )?;
        }
        Ok(left + GAME_WIDTH as f32 * CELL_SIDE)
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

    fn add_flying(&mut self, (left, top): (f32, f32), builder: &mut MeshBuilder) -> GameResult<()> {
        if let Some(flying) = self.game.flying.as_ref() {
            let mask = flying.mask;

            // shadow
            if let Some(lowest_y) = (flying.pos.1 + 1..GAME_HEIGHT as isize)
                .take_while(|&i| !intersects_with(&mask, (flying.pos.0, i), &self.game.board))
                .last()
            {
                for rel_y in 0..4 {
                    for rel_x in 0..4 {
                        if mask[rel_y][rel_x] {
                            let abs_y = (rel_y as isize + lowest_y) as usize;
                            let abs_x = (rel_x as isize + flying.pos.0) as usize;
                            let vis_y = top + abs_y as f32 * CELL_SIDE;
                            let vis_x = left + abs_x as f32 * CELL_SIDE;

                            // each pixel outline
                            // let rect = Rect {
                            //     x: vis_x,
                            //     y: vis_y,
                            //     w: SIDE,
                            //     h: SIDE,
                            // };
                            // builder.rectangle(DrawMode::stroke(3.), rect, flying.id.color());

                            // fainter color (looks bad)
                            // let rgb = flying.id.color().to_rgb();
                            // let increase_possible = rgb.map(|x| 255. / x as f32);
                            // let min_increase_possible = increase_possible.tmin();
                            // let (r, g, b) = rgb.map(|x| {
                            //     let dx = (min_increase_possible * x as f32) as u8;
                            //     x + min(dx, 255 - x)
                            // });
                            // builder.rectangle(DrawMode::fill(), rect, Color::from_rgb(r, g, b));

                            // full block outline
                            let color = flying.id.color();
                            if rel_y == 0 || !mask[rel_y - 1][rel_x] {
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
                            if rel_y == 3 || !mask[rel_y + 1][rel_x] {
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
                            if rel_x == 0 || !mask[rel_y][rel_x - 1] {
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
                            if rel_x == 3 || !mask[rel_y][rel_x + 1] {
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
            }

            // piece
            for (rel_y, row) in mask.iter().enumerate() {
                for (rel_x, &val) in row.iter().enumerate() {
                    if val {
                        let abs_y = (rel_y as isize + flying.pos.1) as usize;
                        let abs_x = (rel_x as isize + flying.pos.0) as usize;
                        let vis_y = top + abs_y as f32 * CELL_SIDE;
                        let vis_x = left + abs_x as f32 * CELL_SIDE;
                        let rect = Rect {
                            x: vis_x,
                            y: vis_y,
                            w: SIDE,
                            h: SIDE,
                        };
                        builder.rectangle(
                            DrawMode::Fill(FillOptions::default()),
                            rect,
                            flying.id.color(),
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn add_queue(&mut self, (left, top): (f32, f32), builder: &mut MeshBuilder) -> f32 {
        // background
        let width = (4. + 2.) * CELL_SIDE;
        let bg_rect = Rect {
            x: left,
            y: top,
            w: width,
            h: (4. * 3. + 5.) * CELL_SIDE,
        };
        builder.rectangle(DrawMode::fill(), bg_rect, Color::from_rgb(56, 56, 56));
        // pieces
        let x = left + CELL_SIDE;
        for (i, id) in self.game.piece_queue.iter().enumerate() {
            let y = top + (i as f32 * 5. + (i + 1) as f32) * CELL_SIDE;
            self.add_piece_at((x, y), id, builder);
        }

        left + width
    }

    fn add_points(
        &self,
        (left, top): (f32, f32),
        builder: &mut MeshBuilder,
        ctx: &mut Context,
    ) -> f32 {
        let height = 4. * CELL_SIDE;
        let bg_rect = Rect {
            x: left,
            y: top,
            w: 10. * CELL_SIDE,
            h: height,
        };
        builder.rectangle(DrawMode::fill(), bg_rect, Color::from_rgb(56, 56, 56));

        queue_text(
            ctx,
            &Text::new(format!("{}", self.game.points)),
            Point2 {
                x: left + CELL_SIDE,
                y: top + CELL_SIDE,
            },
            Some(WHITE),
        );
        top + height
    }

    fn add_keys(&self, (left, top): (f32, f32), builder: &mut MeshBuilder) {
        let bg_rect = Rect {
            x: left,
            y: top,
            w: (6. + 4.) * CELL_SIDE,
            h: (8. + 5.) * CELL_SIDE,
        };
        builder.rectangle(DrawMode::fill(), bg_rect, Color::from_rgb(56, 56, 56));
        let mut key_bg = |x, y, rel_width, code| {
            let cells = rel_width * 3 - 1;
            let rect = Rect {
                x,
                y,
                w: cells as f32 * CELL_SIDE,
                h: 2. * CELL_SIDE,
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
        key_bg(left + 4. * CELL_SIDE, top + CELL_SIDE, 1, KeyCode::Up);
        // down key
        key_bg(
            left + 4. * CELL_SIDE,
            top + 4. * CELL_SIDE,
            1,
            KeyCode::Down,
        );
        // left key
        key_bg(left + CELL_SIDE, top + 4. * CELL_SIDE, 1, KeyCode::Left);
        // right key
        key_bg(
            left + 7. * CELL_SIDE,
            top + 4. * CELL_SIDE,
            1,
            KeyCode::Right,
        );
        // hold key
        key_bg(left + CELL_SIDE, top + 7. * CELL_SIDE, 1, KeyCode::J);
        // rshift
        key_bg(
            left + 4. * CELL_SIDE,
            top + 7. * CELL_SIDE,
            2,
            KeyCode::RShift,
        );
        // spacebar
        key_bg(left + CELL_SIDE, top + 10. * CELL_SIDE, 3, KeyCode::Space);
    }
}

impl EventHandler for VisGame {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        if !self.paused {
            sleep_until(self.next_frame);
            let start = Instant::now();

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
                self.do_key_action(code)
            }

            self.game.iterate();

            self.next_frame = start + WAIT;
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        clear(ctx, WHITE);

        let mut builder = MeshBuilder::new();

        if self.paused {
            builder.rectangle(
                DrawMode::Fill(FillOptions::default()),
                Rect {
                    x: 0.,
                    y: 0.,
                    w: WINDOW_WIDTH,
                    h: WINDOW_HEIGHT,
                },
                Color::from_rgb(64, 64, 64),
            );
        } else {
            let right = self.add_hold(&mut builder);

            let pos = (right + SPACE_BETWEEN, TOP_MARGIN);
            let right = self.add_grid(pos, &mut builder)?;
            self.add_pixels(pos, &mut builder);
            self.add_flying(pos, &mut builder)?;

            let right = self.add_queue((right + SPACE_BETWEEN, TOP_MARGIN), &mut builder);
            let bottom = self.add_points((right + SPACE_BETWEEN, TOP_MARGIN), &mut builder, ctx);
            self.add_keys(
                (right + SPACE_BETWEEN, bottom + SPACE_BETWEEN),
                &mut builder,
            );
        }

        // build and draw
        let mesh = builder.build(ctx)?;
        draw(ctx, &mesh, DrawParam::default())?;
        draw_queued_text(ctx, DrawParam::default(), None, FilterMode::Linear)?;

        present(ctx)
    }

    fn key_down_event(&mut self, _ctx: &mut Context, code: KeyCode, _mods: KeyMods, _: bool) {
        let mut found = false;
        let mut action = None;
        self.keys.entry(code).and_modify(|v| {
            if v.state == PressedState::Up {
                v.state = match v.repeat {
                    Repeat::Repeat { initial_delay, .. } => PressedState::Fresh(initial_delay),
                    Repeat::NoRepeat => PressedState::Down,
                };
                action = Some(code);
            }
            found = true;
        });
        if let Some(code) = action {
            self.do_key_action(code)
        }

        if !found {
            if let KeyCode::Escape = code {
                self.paused = !self.paused
            }
        }
    }

    fn key_up_event(&mut self, _ctx: &mut Context, code: KeyCode, _mods: KeyMods) {
        self.keys.entry(code).and_modify(|v| {
            v.state = PressedState::Up;
        });
    }
}
