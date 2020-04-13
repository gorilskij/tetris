use crate::game::{Game, Pixel, GAME_HEIGHT, GAME_WIDTH};
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

// fresh indicates the key was just pressed (with iterations left to wait)
#[derive(Debug, Eq, PartialEq)]
enum PressedState {
    Up,
    Down,
    Fresh(u8),
}

type PressedKeys = HashMap<KeyCode, PressedState>;

pub struct VisGame {
    game: Game,
    next_frame: Instant,
    pressed_keys: PressedKeys,
}

impl VisGame {
    pub fn run() {
        let window_mode = WindowMode {
            width: 550.,
            height: 650.,
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

        let mut keys = PressedKeys::new();
        keys.insert(KeyCode::Left, PressedState::Up);
        keys.insert(KeyCode::Right, PressedState::Up);
        keys.insert(KeyCode::Down, PressedState::Up);
        let mut game = Self {
            game: Game::new(),
            next_frame: Instant::now(),
            pressed_keys: keys,
        };

        ggez::event::run(ctx, event_loop, &mut game).expect("game exited unsuccessfully");
    }
}

const LEFT_MARGIN: f32 = 10.;
const TOP_MARGIN: f32 = 10.;
const QUEUE_MARGIN: f32 = 30.; // between game and piece queue
const CELL_SIDE: f32 = 30.;

const FPS: u64 = 60;
const WAIT: Duration = Duration::from_millis(1000 / FPS);

impl VisGame {
    // unconditional
    fn teleport(game: &mut Game, code: KeyCode) {
        match code {
            KeyCode::Left => game.teleport_flying_piece(-1, 0),
            KeyCode::Right => game.teleport_flying_piece(1, 0),
            KeyCode::Down => game.teleport_flying_piece(0, 1),
            c => panic!("invalid teleportation KeyCode: {:?}", c),
        }
    }

    // for repetition
    fn key_pressed_teleport(&mut self, code: KeyCode) {
        let pressed_keys = &mut self.pressed_keys;
        let game = &mut self.game;
        pressed_keys.entry(code).and_modify(|v| match v {
            PressedState::Fresh(0) | PressedState::Down => {
                Self::teleport(game, code);
                *v = PressedState::Down;
            }
            PressedState::Fresh(x) => *x -= 1,
            _ => (),
        });
    }
}

impl EventHandler for VisGame {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        sleep_until(self.next_frame);
        let start = Instant::now();

        if self.game.tick % 5 == 0 {
            self.key_pressed_teleport(KeyCode::Left);
            self.key_pressed_teleport(KeyCode::Right);
            self.key_pressed_teleport(KeyCode::Down);
        }

        self.game.iterate();

        self.next_frame = start + WAIT;
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        clear(ctx, WHITE);

        let mut builder = MeshBuilder::new();

        // make grid
        for x in 0..=GAME_WIDTH {
            let x = LEFT_MARGIN + x as f32 * CELL_SIDE;
            builder.line(
                &[
                    Point2 { x, y: TOP_MARGIN },
                    Point2 {
                        x,
                        y: TOP_MARGIN + GAME_HEIGHT as f32 * CELL_SIDE,
                    },
                ],
                1.,
                BLACK,
            )?;
        }
        for y in 0..=GAME_HEIGHT {
            let y = TOP_MARGIN + y as f32 * CELL_SIDE;
            builder.line(
                &[
                    Point2 { x: LEFT_MARGIN, y },
                    Point2 {
                        x: LEFT_MARGIN + GAME_WIDTH as f32 * CELL_SIDE,
                        y,
                    },
                ],
                1.,
                BLACK,
            )?;
        }

        const MARGIN: f32 = 0.1;
        const SIDE: f32 = CELL_SIDE - 2. * MARGIN;

        // add lying pieces
        for (r, row) in self.game.board.iter().enumerate() {
            for (c, px) in row.iter().enumerate() {
                if let Pixel::Full(id) = px {
                    let left = LEFT_MARGIN + c as f32 * CELL_SIDE + MARGIN;
                    let top = TOP_MARGIN + r as f32 * CELL_SIDE + MARGIN;
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

        // add flying piece
        if let Some(flying) = self.game.flying.as_ref() {
            let rotation = flying.mask;
            for rel_y in 0..4 {
                for rel_x in 0..4 {
                    if rotation[rel_y][rel_x] {
                        let abs_y = (rel_y as isize + flying.pos.1) as usize;
                        let abs_x = (rel_x as isize + flying.pos.0) as usize;
                        let vis_y = TOP_MARGIN + abs_y as f32 * CELL_SIDE;
                        let vis_x = LEFT_MARGIN + abs_x as f32 * CELL_SIDE;
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

        // -- piece queue --
        // background
        let left = LEFT_MARGIN + GAME_WIDTH as f32 * CELL_SIDE + QUEUE_MARGIN;
        let top = TOP_MARGIN;
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
            let mask = self.game.mask_map[&id][0];
            for rel_y in 0..4 {
                for rel_x in 0..4 {
                    if mask[rel_y][rel_x] {
                        let rect = Rect {
                            x: x + rel_x as f32 * CELL_SIDE,
                            y: y + rel_y as f32 * CELL_SIDE,
                            w: SIDE,
                            h: SIDE,
                        };
                        builder.rectangle(DrawMode::Fill(FillOptions::default()), rect, id.color());
                    }
                }
            }
        }

        // build and draw
        let mesh = builder.build(ctx)?;
        draw(ctx, &mesh, DrawParam::default())?;
        present(ctx)
    }

    fn key_down_event(&mut self, _ctx: &mut Context, code: KeyCode, _mods: KeyMods, _: bool) {
        let mut found = false;
        let game = &mut self.game;
        self.pressed_keys.entry(code).and_modify(|v| {
            if *v == PressedState::Up {
                *v = PressedState::Fresh(2);
                Self::teleport(game, code);
            }
            found = true;
        });

        if !found {
            match code {
                KeyCode::Up => self.game.rotate_flying_piece(1),
                KeyCode::RShift => self.game.rotate_flying_piece(-1),
                KeyCode::Space => self.game.slam_down(),
                _ => (),
            }
        }
    }

    fn key_up_event(&mut self, _ctx: &mut Context, code: KeyCode, _mods: KeyMods) {
        self.pressed_keys.entry(code).and_modify(|v| {
            *v = PressedState::Up;
        });
    }
}
