use ggez::nalgebra::DMatrix;
use rand::prelude::*;

pub enum Activation {
    Relu,
    Sigmoid,
}

fn relu_activation(x: f64) -> f64 {
    if x < 0. {
        0.
    } else {
        x
    }
}

fn sigmoid_activation(x: f64) -> f64 {
    1. / (1. + (-x).exp())
}

struct Layer {
    weights: DMatrix<f64>,
    activation: fn(f64) -> f64,
}

pub struct NN {
    layers: Box<[Layer]>,
}

fn print_out(label: &str, out: &DMatrix<f64>) {
    print!("{:>5}: [", label);
    for n in out.iter() {
        print!("{:.2}, ", n);
    }
    println!("]");
}

pub struct NNBuilder {
    last_size: usize,
    layers: Vec<Layer>,
    rng: ThreadRng,
}

// with random noise (ncols: size of from layer, nrows: size of to layer)
fn gen_weights(ncols: usize, nrows: usize, rng: &mut ThreadRng) -> DMatrix<f64> {
    // warn: not sure about order of nrows and ncols here, it changed...
    DMatrix::from_fn(nrows, ncols, |_, _| rng.gen_range(0.0, 0.00001))
}

#[derive(Debug)]
pub struct NNCreationError(String);

impl NNBuilder {
    pub fn add_layer(mut self, size: usize, activation: Activation) -> Self {
        self.layers.push(Layer {
            weights: gen_weights(self.last_size, size, &mut self.rng),
            activation: match activation {
                Activation::Relu => relu_activation,
                Activation::Sigmoid => sigmoid_activation,
            },
        });
        self.last_size = size;
        self
    }

    pub fn build(self) -> Result<NN, NNCreationError> {
        if self.layers.is_empty() {
            Err(NNCreationError(
                "can't build network with <= 1 layers".to_string(),
            ))
        } else {
            Ok(NN {
                layers: self.layers.into_boxed_slice(),
            })
        }
    }
}

impl NN {
    // takes size of input layer, output layer is the size passed as the last layer
    pub fn make(first_layer_size: usize) -> NNBuilder {
        NNBuilder {
            last_size: first_layer_size,
            layers: vec![],
            rng: thread_rng(),
        }
    }

    pub fn apply(&self, input: &[f64]) -> Box<[f64]> {
        println!("--start apply--");
        assert_eq!(input.len(), self.layers[0].weights.ncols());
        let mut vec = DMatrix::from_column_slice(input.len(), 1, input);
        for Layer { weights, .. } in self.layers.iter() {
            vec = weights * vec;
            // relu activation
            for x in &mut vec {
                if *x < 0. {
                    *x = 0.
                }
            }
            print_out("inter", &vec);
        }
        println!("-- end --");
        vec.iter().copied().collect::<Vec<_>>().into_boxed_slice()
    }
}
