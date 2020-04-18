use ggez::nalgebra::DMatrix;
use rand::prelude::*;

struct Layer(DMatrix<f64>);

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

impl NN {
    // layer_sizes[0] is input layer and layer_sizes[-1] is output layer
    pub fn new(layer_sizes: &[usize]) -> Self {
        let rng = &mut thread_rng();
        let mut layers = vec![];
        let mut windows = layer_sizes.windows(2);
        while let Some(&[nrows, ncols]) = windows.next() {
            // randomize initial state
            let max = 0.00001;
            let mat: DMatrix<f64> = DMatrix::from_fn(ncols, nrows, |_, _| rng.gen_range(0.0, max));
            layers.push(Layer(mat));
        }
        println!("len: {}", layers.len());
        println!(
            "dims: {:?}",
            layers
                .iter()
                .map(|m| (m.0.ncols(), m.0.nrows()))
                .collect::<Vec<_>>()
        );
        Self {
            layers: layers.into_boxed_slice(),
        }
    }

    pub fn apply(&self, input: &[f64]) -> Box<[f64]> {
        println!("--start apply--");
        assert_eq!(input.len(), self.layers[0].0.ncols());
        let mut vec = DMatrix::from_column_slice(input.len(), 1, input);
        for Layer(weights) in self.layers.iter() {
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
