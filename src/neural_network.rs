use ggez::nalgebra::DMatrix;
use itertools::Itertools;
use rand::prelude::*;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Error, LineWriter, Write};
use std::num::{ParseFloatError, ParseIntError};
use std::path::Path;
use std::iter::once;

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

#[derive(Copy, Clone, PartialEq)]
pub enum ActivationType {
    Relu,
    Sigmoid,
}

impl ActivationType {
    fn fn_ptr(self) -> fn(f64) -> f64 {
        match self {
            ActivationType::Relu => relu_activation,
            ActivationType::Sigmoid => sigmoid_activation,
        }
    }
}

struct Activation {
    typ: ActivationType,
    fnp: fn(f64) -> f64,
}

impl PartialEq for Activation {
    fn eq(&self, other: &Self) -> bool {
        self.typ == other.typ
    }
}

#[derive(PartialEq)]
struct Layer {
    weights: DMatrix<f64>,
    activation: Activation,
}

#[derive(PartialEq)]
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
    pub fn add_layer(mut self, size: usize, activation_type: ActivationType) -> Self {
        self.layers.push(Layer {
            weights: gen_weights(self.last_size + 1 /* bias */, size, &mut self.rng),
            activation: Activation {
                typ: activation_type,
                fnp: activation_type.fn_ptr(),
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

    pub fn apply(&self, input: &[f64]) -> DMatrix<f64> {
        // println!("--start apply--");
        assert_eq!(input.len() + 1, self.layers[0].weights.ncols());
        let mut data = DMatrix::from_iterator(input.len(), 1, input.iter().copied());
        for Layer {
            weights,
            activation,
        } in self.layers.iter()
        {
            // insert bias as first element
            data = data.insert_row(0, 1.);

            data = weights * data;
            data.apply(activation.fnp);
            // print_out("inter", &vec);
        }
        // println!("-- end --");
        // data.iter().copied().collect::<Vec<_>>().into_boxed_slice()
        data
    }
}

#[derive(Debug)]
pub enum NNReadError {
    IoError(io::Error),
    ParseIntError(ParseIntError),
    ParseFloatError(ParseFloatError),
    Other(String),
}

impl From<io::Error> for NNReadError {
    fn from(e: Error) -> Self {
        Self::IoError(e)
    }
}

impl From<ParseIntError> for NNReadError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseIntError(e)
    }
}

impl From<ParseFloatError> for NNReadError {
    fn from(e: ParseFloatError) -> Self {
        Self::ParseFloatError(e)
    }
}

pub type NNReadResult<T> = Result<T, NNReadError>;

#[test]
fn test_nn_serialization() {
    use crate::{
        game::{GAME_HEIGHT, GAME_WIDTH},
        neural_network::{ActivationType, NN},
    };
    let nn = NN::make(GAME_WIDTH * GAME_HEIGHT)
        .add_layer(20, ActivationType::Relu)
        .add_layer(10, ActivationType::Relu)
        .add_layer(7, ActivationType::Sigmoid)
        .build()
        .unwrap();
    let file_path = "temporary_test_nn.txt";
    nn.write_out(file_path).unwrap();
    let read = NN::read_in(file_path).unwrap();
    assert!(nn == read);
    std::fs::remove_file(file_path).unwrap();
}

impl NN {
    // overwrites!
    #[allow(dead_code)]
    pub fn write_out<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut lw = LineWriter::new(File::create(path)?);
        lw.write_all(format!("LAYERS: {}\n", self.layers.len()).as_bytes())?;
        for Layer {
            weights,
            activation,
        } in self.layers.iter()
        {
            // nrows x ncols (height x width)
            let size = format!("{}x{}", weights.nrows(), weights.ncols());
            let activation = match activation.typ {
                ActivationType::Relu => b"R",
                ActivationType::Sigmoid => b"S",
            };
            let ws = weights.iter().map(|w| format!("{}", w)).join(",");
            lw.write_all(size.as_bytes())?;
            lw.write_all(b" ")?;
            lw.write_all(activation)?;
            lw.write_all(b" ")?;
            lw.write_all(ws.as_bytes())?;
            lw.write_all(b"\n")?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn read_in<P: AsRef<Path>>(path: P) -> NNReadResult<Self> {
        let mut br = BufReader::new(File::open(path)?);
        let num_layers = {
            let mut line = String::new();
            br.read_line(&mut line)?;
            let prefix = "LAYERS: ";
            if line.starts_with(prefix) {
                line.chars()
                    .skip(prefix.len())
                    .take_while(|c| c.is_numeric())
                    .collect::<String>()
                    .parse::<usize>()?
            } else {
                return Err(NNReadError::Other("invalid first line format".to_string()));
            }
        };
        let layer_read_error = |i| NNReadError::Other(format!("invalid layer at index {}", i));
        let mut layers = Vec::with_capacity(num_layers);
        for (i, line) in br.lines().enumerate() {
            let line = line?;
            let mut split = line.split(' ');
            let size = {
                let mut iter = split.next().ok_or_else(|| layer_read_error(i))?.split('x');
                (
                    iter.next()
                        .ok_or_else(|| layer_read_error(i))?
                        .parse::<usize>()?,
                    iter.next()
                        .ok_or_else(|| layer_read_error(i))?
                        .parse::<usize>()?,
                )
            };
            let activation = {
                let typ = match split.next().ok_or_else(|| layer_read_error(i))? {
                    "R" => ActivationType::Relu,
                    "S" => ActivationType::Sigmoid,
                    s => return Err(NNReadError::Other(format!("invalid activation: {}", s))),
                };
                Activation {
                    typ,
                    fnp: typ.fn_ptr(),
                }
            };
            let weights = {
                let ws = split
                    .next()
                    .ok_or_else(|| layer_read_error(i))?
                    .split(',')
                    .map(|s| s.parse::<f64>())
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice();
                if ws.len() != size.0 * size.1 {
                    return Err(NNReadError::Other(format!(
                        "size ({}x{}) expects {} weights but only {} are given",
                        size.0,
                        size.1,
                        size.0 * size.1,
                        ws.len()
                    )));
                }
                DMatrix::from_iterator(size.0, size.1, ws.iter().copied())
            };
            layers.push(Layer {
                weights,
                activation,
            })
        }
        if num_layers != layers.len() {
            return Err(NNReadError::Other(format!(
                "expected {} layers but got {}",
                num_layers,
                layers.len()
            )));
        }

        Ok(Self {
            layers: layers.into_boxed_slice(),
        })
    }
}
