use crate::{
    game::{visual::VisGame, GAME_HEIGHT, GAME_WIDTH},
    neural_network::{ActivationType, NN},
    run_game,
};
use ggez::{
    event::{EventHandler, KeyMods},
    input::keyboard::KeyCode,
    Context, GameResult,
};

// don't know how useful this actually is
pub struct NNVisGame {
    vis: VisGame,
    nn: NN,
}

impl NNVisGame {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            vis: VisGame::new(),
            // all cells as input, 7 keys as output
            // nn: NN::new(&[GAME_WIDTH * GAME_HEIGHT, 20, 10, 7]),
            nn: NN::make(GAME_WIDTH * GAME_HEIGHT)
                .add_layer(20, ActivationType::Relu)
                .add_layer(10, ActivationType::Relu)
                .add_layer(7, ActivationType::Sigmoid)
                .build()
                .unwrap(),
        }
    }

    #[allow(dead_code)]
    pub fn run(&mut self) -> GameResult<()> {
        run_game(self)
    }
}

pub(crate) const KEY_ORDER: [KeyCode; 7] = [
    KeyCode::Up,
    KeyCode::Down,
    KeyCode::Left,
    KeyCode::Right,
    KeyCode::J,
    KeyCode::RShift,
    KeyCode::Space,
];

fn print_out(label: &str, out: &[f64]) {
    print!("{:>5}: [", label);
    for n in out[..out.len() - 1].iter() {
        print!("{:.2}, ", n);
    }
    println!("{:.2}]", out[out.len() - 1]);
}

impl EventHandler for NNVisGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        println!("update");
        let input = self.vis.game.get_cells();
        print_out("in", &input);
        let mut output = self
            .nn
            .apply(&input)
            .iter()
            .copied()
            .collect::<Vec<_>>()
            .into_boxed_slice();
        print_out("raw", &output);
        // normalize with sigmoid
        for out in output.iter_mut() {
            *out = 1. / (1. + (-*out).exp())
        }
        print_out("norm", &output);
        for (i, out) in output.iter_mut().enumerate() {
            let code = KEY_ORDER[i];
            let is_pressed = self.vis.keys[&code].state.is_pressed();
            let should_be_pressed = *out > 0.5;
            if is_pressed && !should_be_pressed {
                self.vis.key_up_event(ctx, code, KeyMods::default())
            } else if !is_pressed && should_be_pressed {
                self.vis
                    .key_down_event(ctx, code, KeyMods::default(), false)
            }
        }
        self.vis.update(ctx)
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.vis.draw(ctx)
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        _keycode: KeyCode,
        _keymods: KeyMods,
        _repeat: bool,
    ) {
        panic!("no forwarding yet")
    }

    fn key_up_event(&mut self, _ctx: &mut Context, _keycode: KeyCode, _keymods: KeyMods) {
        panic!("no forwarding yet")
    }
}
