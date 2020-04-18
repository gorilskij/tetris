use crate::game::visual::VisGame;
use crate::game::{GAME_HEIGHT, GAME_WIDTH};
use crate::neural_network::NN;
use ggez::event::{EventHandler, KeyMods};
use ggez::{Context, GameResult};
use ggez::input::keyboard::KeyCode;
use crate::run_game;

pub struct NNVisGame {
    vis: VisGame,
    nn: NN,
}

impl NNVisGame {
    pub fn new() -> Self {
        Self {
            vis: VisGame::new(),
            // all cells as input, 7 keys as output
            nn: NN::new(&[GAME_WIDTH * GAME_HEIGHT, 20, 10, 7]),
        }
    }

    pub fn run(&mut self) -> GameResult<()> {
        run_game(self)
    }
}

const KEY_ORDER: [KeyCode; 7] = [KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right, KeyCode::J, KeyCode::RShift, KeyCode::Space];

impl EventHandler for NNVisGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        println!("update");
        let input = self.vis.game.get_cells();
        let mut output = self.nn.apply(&input);
        // normalize with sigmoid
        for out in output.iter_mut() {
            *out = 1. / (1. + (-*out).exp())
        }
        println!("out: {:?}", output);
        for (i, out) in output.iter_mut().enumerate() {
            let code = KEY_ORDER[i];
            let is_pressed = self.vis.keys[&code].state.is_pressed();
            let should_be_pressed = *out > 0.5;
            if is_pressed && !should_be_pressed {
                self.vis.key_up_event(ctx, code, KeyMods::default())
            } else if !is_pressed && should_be_pressed {
                self.vis.key_down_event(ctx, code, KeyMods::default(), false)
            }
        }
        self.vis.update(ctx)
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.vis.draw(ctx)
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        _keymods: KeyMods,
        _repeat: bool,
    ) {
        panic!("no forwarding yet")
    }

    fn key_up_event(&mut self, _ctx: &mut Context, _keycode: KeyCode, _keymods: KeyMods) {
        panic!("no forwarding yet")
    }
}
