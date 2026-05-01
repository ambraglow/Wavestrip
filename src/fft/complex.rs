use rand::RngExt;
use std::f32::consts::TAU;

pub struct RandComplex<T> {
    complex: T,
    step: T,
}

pub struct Sine {
    freq: f32,
    sample_rate: u32,
    samples_nr: u32,
}

impl RandComplex<f32> {
    pub fn default(step: f32, opt_complex: Option<f32>) -> Self {
        if let Some(complex) = opt_complex {
            RandComplex {
                complex: complex,
                step: step,
            }
        } else {
            RandComplex {
                complex: 0.0,
                step: step,
            }
        }
    }
    pub fn random_vec(samples_nr: usize, step: f32) -> Option<Vec<f32>> {
        let complex = RandComplex::default(step, None);
        Some(complex.take(samples_nr).collect())
    }
}

impl Sine {
    pub fn new(freq: f32, sample_rate: u32) -> Self {
        Sine {
            freq,
            sample_rate,
            ..Default::default()
        }
    }
}

impl Default for Sine {
    fn default() -> Self {
        Sine {
            freq: 2000.0,
            samples_nr: 0,
            sample_rate: 44000,
        }
    }
}

impl Iterator for Sine {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.samples_nr = self.samples_nr.wrapping_add(1);
        let value = TAU * self.freq * (self.samples_nr as f32 / self.sample_rate as f32);
        Some(value.sin())
    }
}

impl Iterator for RandComplex<f32> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let mut rng = rand::rng();
        // rng.reseed().unwrap();
        self.step = rng.random_range(0.01..1.0);
        self.complex = self.step;
        Some(self.complex)
    }
}
