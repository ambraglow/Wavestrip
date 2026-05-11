use crate::dsp::fourier::Fourier;
use crate::dsp::utils::even_odd_split;
use crate::dsp::*;
use std::ops::MulAssign;

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

#[derive(Debug)]
struct fft {
    N: usize,
    W: Complex32,
    even: Vec<Complex32>,
    odds: Vec<Complex32>,
    current: Vec<Complex32>,
}

impl Fourier for Vec<Complex32> {
    /// this is recursive fft atm
    fn simple_dft(&mut self) -> Vec<Complex32> {
        let n: usize = self.len();

        if n <= 1 {
            return self.to_vec();
        }

        let (mut even, mut odds) = even_odd_split(self).unwrap();

        even = even.simple_dft();
        odds = odds.simple_dft();

        for k in 0..n / 2 {
            let t = Complex32::cis(-TAU / n as f32).powf(k as f32) * odds[k];
            self[k] = even[k] + t;
            self[k + n / 2] = even[k] - t;
        }

        return self.to_vec();
    }
    fn dft(&mut self) -> Vec<Complex32> {
        return Vec::new();
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
    /// Non-recursive FFT reference code:
    /// https://github.com/muditbhargava66/FFT-implementation-in-C/blob/main/algorithms/core/radix2_dit.c
    ///
    fn ditfft(&mut self, twiddle_factors_table: &Vec<Complex32>) -> Vec<Complex32> {
        let n = self.len();
        if n <= 1 {
            return Vec::new();
        }

        let log2n = (n as f64).log2() as u32;
        // log::info!("n: {:?}, log2n: {:?}", n, log2n);

        {
            // let _s = span!("bit reverse looop");
            // TODO: change this method for bit-reversal
            for i in 0..n {
                // let _s = span!("bit reverse");

                let j = bit_reverse(i as u32, log2n).unwrap();
                // log::info!("i: {:?}, j: {:?}", i, j);

                if i < j as usize {
                    // let _s = span!("iter items swap");
                    self.swap(i, j as usize);
                }
            }
        }

        {
            // let _s = span!("fft stages");

            for stage in 1..log2n {
                let m = 1 << stage;
                let half_m = m >> 1;

                // -1.0 is forward fft
                // let _s = span!("twiddle factor angle");
                let w_m: Complex<f32> = Complex32::cis(-1.0 * TAU * 1.0 / m as f32);
                // log::info!("stage: {:?} w_m {w_m}", stage);

                for k in (0..n).step_by(m) {
                    let mut w = Complex32::new(1.0, 0.0);

                    for j in 0..half_m {
                        // let _s = span!("twiddle factor");

                        let t = k + j;
                        let u = t + half_m;

                        let temp_cplx = self[u] * w;

                        self[u] = self[t] - temp_cplx;
                        self[t] = self[t] + temp_cplx;

                        // Update twiddle factor for next butterfly
                        w.mul_assign(w_m);
                    }
                }
            }
        }
        return self.to_vec();
    }
}

pub fn bit_reverse(mut input: u32, log2n: u32) -> anyhow::Result<u32> {
    if log2n <= 8 {
        input = ((input & 0xAAAA) >> 1) | ((input & 0x5555) << 1);
        input = ((input & 0xCCCC) >> 2) | ((input & 0x3333) << 2);
        input = ((input & 0xF0F0) >> 4) | ((input & 0x0F0F) << 4);
        if log2n > 4 {
            input = ((input & 0xFF00) >> 8) | ((input & 0x00FF) << 8);
        }
        return Ok(input >> (16 - log2n));
    }

    let mut r: u32 = 0;
    for i in 0..log2n {
        r = (r << 1) | (input & 1);
        input >>= 1;
    }
    Ok(r)
}
