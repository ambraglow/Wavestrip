use crate::dsp::fourier::Fourier;
use crate::dsp::utils::even_odd_split;
use crate::dsp::*;

pub struct RandComplex<T> {
    complex: T,
    step: T,
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
    fn fft(&mut self) -> Vec<Complex<f32>> {
        let N = self.len();
        let W: Complex<f32> = Complex32::cis(-TAU / N as f32);
        // let mut out_vec: Vec<Complex32> = vec![Complex32::new(0f32, 0f32); N + 1];
        // out_vec.append(&mut x.clone());

        // println!("N: {:?}", N);

        if N <= 1 {
            return self.to_vec();
        }

        let (mut even, mut odds) = even_odd_split(self).unwrap();

        even = even.fft();
        odds = odds.fft();

        for k in 0..N / 2 {
            let t = W.powf(k as f32) * odds[k];
            self[k] = even[k] + t;
            self[k + N / 2] = even[k] - t;
        }

        return self.to_vec();
    }
    fn ditfft(&mut self, twiddle_factors_table: &Vec<Complex32>) -> Vec<Complex32> {
        let n = self.len();
        let fft_n = twiddle_factors_table.len();
        // let W: Complex<f32> = Complex32::cis(-TAU / N as f32);
        // let mut out_vec: Vec<Complex32> = vec![Complex32::new(0f32, 0f32); N + 1];
        // out_vec.append(&mut x.clone());

        if n <= 1 {
            return self.to_vec();
        }

        let (mut even, mut odds) = even_odd_split(self).unwrap();

        even = even.ditfft(&twiddle_factors_table);
        odds = odds.ditfft(&twiddle_factors_table);

        let mut t: Complex32;
        for k in 0..n / 2 {
            if n / 2 != fft_n / 2 {
                // println!("{:?} {N:?} {:?}", k, FRACTION_STEP - N + k);
                t = twiddle_factors_table[fft_n - n + k] * odds[k];
            } else {
                // 0..1024
                log::debug!("{:?} {n:?} {:?}", k, fft_n - n + k);
                t = twiddle_factors_table[k] * odds[k];
            }

            self[k] = even[k] + t;
            self[k + n / 2] = even[k] - t;
        }

        self.to_vec()
    }
}
