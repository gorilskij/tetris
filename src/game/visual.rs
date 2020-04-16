use crate::game::{intersects_with, Game, PieceId, Pixel, GAME_HEIGHT, GAME_WIDTH};
use crate::support::sleep_until;
use ggez::conf::{FullscreenType, WindowMode};
use ggez::event::{EventHandler, KeyMods};
use ggez::graphics::{
    clear, draw, present, Color, DrawMode, DrawParam, FillOptions, MeshBuilder, Rect, BLACK, WHITE,
};
use ggez::input::keyboard::KeyCode;
use ggez::mint::Point2;
use ggez::{Context, ContextBuilder, GameResult};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use std::cmp::min;
#[allow(unused_imports)]
use tuple_map::*;

// fresh indicates the key was just pressed (with iterations left to wait)
#[derive(Debug, Eq, PartialEq)]
enum PressedState {
    Up,
    Down,
    Fresh(u8),
}

enum Repeat {
    // initial_delay is number of 'delay's to wait
    // i.e. if delay is 3 frames and initial_delay is 2, initial delay will be 6 frames
    Repeat { initial_delay: u8, delay: u8 },
    NoRepeat,
}

struct KeyInfo {
    state: PressedState,
    repeat: Repeat,
}

type Keys = HashMap<KeyCode, KeyInfo>;

pub struct VisGame {
    game: Game,
    paused: bool,
    next_frame: Instant,
    keys: Keys,
}

const WINDOW_WIDTH: f32 = 750.;
const WINDOW_HEIGHT: f32 = 650.;

impl VisGame {
    pub fn run() {
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
        let mut game = Self {
            game: Game::new(),
            paused: false,
            next_frame: Instant::now(),
            keys: keys,
        };

        ggez::event::run(ctx, event_loop, &mut game).expect("game exited unsuccessfully");
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
            KeyCode::Space => self.game.slam_down(),
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
        for rel_y in 0..4 {
            for rel_x in 0..4 {
                if mask[rel_y][rel_x] {
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
    #[must_use]
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

    #[must_use]
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
            for rel_y in 0..4 {
                for rel_x in 0..4 {
                    if mask[rel_y][rel_x] {
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

    fn add_queue(&mut self, (left, top): (f32, f32), builder: &mut MeshBuilder) {
        // background
        let bg_rect = Rect {
            x: left,
            y: top,
            w: (4. + 2.) * CELL_SIDE,
            h: (4. * 3. + 5.) * CELL_SIDE,
        };
        builder.rectangle(
            DrawMode::Fill(FillOptions::default()),
            bg_rect,
            Color::from_rgb(56, 56, 56),
        );
        // pieces
        let x = left + CELL_SIDE;
        for (i, id) in self.game.piece_queue.iter().enumerate() {
            let y = top + (i as f32 * 5. + (i + 1) as f32) * CELL_SIDE;
            self.add_piece_at((x, y), id, builder);
        }
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

            self.add_queue((right + SPACE_BETWEEN, TOP_MARGIN), &mut builder);
        }

        // build and draw
        let mesh = builder.build(ctx)?;
        draw(ctx, &mesh, DrawParam::default())?;
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
            match code {
                KeyCode::Escape => self.paused = !self.paused,
                _ => (),
            }
        }
        //         // KeyCode::Up => self.game.rotate_flying_piece(1),
        //         // KeyCode::RShift => self.game.rotate_flying_piece(-1),
        //         // KeyCode::Space => self.game.slam_down(),
        //         // KeyCode::J => self.game.switch_hold(),
        //         // KeyCode::Escape => self.paused = !self.paused,
        //         _ => (),
        //     }
        // }
    }

    fn key_up_event(&mut self, _ctx: &mut Context, code: KeyCode, _mods: KeyMods) {
        self.keys.entry(code).and_modify(|v| {
            v.state = PressedState::Up;
        });
    }
}
