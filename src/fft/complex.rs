use crate::fft::utils::even_odd_split;
use crate::fft::*;
use crate::fourier::Fourier;
use rand::RngExt;

impl Fourier for Vec<Complex32> {
    fn simple_dft(&mut self) -> Vec<Complex32> {
        let signal = self.len();
        let mut temp: Vec<Complex32> = vec![Complex32::default(); signal as usize];

        (0..signal)
            .flat_map(|frequency_bin| (0..signal).map(move |sample| (frequency_bin, sample)))
            .for_each(|(frequency_bin, sample)| {
                let angle: f32 = -TAU * frequency_bin as f32 * sample as f32 / signal as f32;
                let complex: Complex32 = Complex32::new(angle.cos(), angle.sin());
                temp[frequency_bin] += self[sample] * complex;
            });

        temp
    }
    fn dft(&mut self) {
        todo!()
    }
    fn fft(&mut self) {
        todo!()
    }
    fn ditfft(&mut self) -> Vec<Complex32> {
        let N = self.len();
        let W: Complex<f32> = Complex32::cis(-TAU / N as f32);
        // let mut out_vec: Vec<Complex32> = vec![Complex32::new(0f32, 0f32); N + 1];
        // out_vec.append(&mut x.clone());

        // println!("N: {:?}", N);

        if N <= 1 {
            return self.to_vec();
        }

        let (mut even, mut odds) = even_odd_split(self).unwrap();

        even = even.ditfft();
        odds = odds.ditfft();

        for k in 0..N / 2 {
            let t = W.powf(k as f32) * odds[k];
            self[k] = even[k] + t;
            self[k + N / 2] = even[k] - t;
        }

        return self.to_vec();
    }
}

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
