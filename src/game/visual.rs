use crate::game::{Game, Pixel, GAME_HEIGHT, GAME_WIDTH};
use crate::support::sleep_until;
use ggez::conf::{FullscreenType, WindowMode};
use ggez::event::{EventHandler, KeyMods};
use ggez::graphics::{
    clear, draw, present, Color, DrawMode, DrawParam, FillOptions, MeshBuilder, Rect,
    StrokeOptions, BLACK, WHITE,
};
use ggez::input::keyboard::KeyCode;
use ggez::mint::Point2;
use ggez::{Context, ContextBuilder, GameError, GameResult};
use std::time::{Duration, Instant};

pub struct VisGame {
    game: Game,
    next_frame: Instant,
}

impl VisGame {
    pub fn run() {
        let window_mode = WindowMode {
            width: 400.0,
            height: 800.0,
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

        let mut game = Self {
            game: Game::new(),
            next_frame: Instant::now(),
        };

        ggez::event::run(ctx, event_loop, &mut game).expect("game exited unsuccessfully");
    }
}

const LEFT_MARGIN: f32 = 10.;
const TOP_MARGIN: f32 = 10.;
const CELL_SIDE: f32 = 30.;

const FPS: u64 = 60;
const WAIT: Duration = Duration::from_millis(1000 / FPS);

impl EventHandler for VisGame {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        sleep_until(self.next_frame);
        let start = Instant::now();

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
            let rotation = flying.rotation;
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

        let mesh = builder.build(ctx)?;
        draw(ctx, &mesh, DrawParam::default());
        present(ctx)
    }

    fn key_down_event(&mut self, ctx: &mut Context, code: KeyCode, _mods: KeyMods, _: bool) {
        match code {
            KeyCode::Left => self.game.teleport_flying_piece(-1, 0),
            KeyCode::Right => self.game.teleport_flying_piece(1, 0),
            KeyCode::Up => self.game.rotate_flying_piece(1),
            KeyCode::RShift => self.game.rotate_flying_piece(-1),
            _ => (),
        }
    }
}
