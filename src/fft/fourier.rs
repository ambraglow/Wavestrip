use std::{iter::FromIterator, ops::Deref};

use rodio_experiments::source::FromIter;

use crate::fft::{utils::even_odd_split, *};

#[derive(Clone)]
pub struct FourierTransform {
    pub bins: Vec<Complex32>,
    pub signal: Vec<f32>,
    pub twiddle_factors_table: Vec<Complex32>,
}

impl FourierTransform {
    pub fn new() -> Self {
        FourierTransform {
            bins: Vec::new(),
            signal: Vec::new(),
            twiddle_factors_table: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FourierVecf32(pub Vec<f32>);

impl Deref for FourierVecf32 {
    type Target = Vec<f32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<f32> for FourierVecf32 {
    fn from_iter<T: IntoIterator<Item = f32>>(iter: T) -> Self {
        FourierVecf32(iter.into_iter().collect())
    }
}

impl From<Vec<Complex32>> for FourierVecf32 {
    fn from(value: Vec<Complex32>) -> Self {
        FourierVecf32(value.iter().map(|a| a.re).collect())
    }
}

impl From<Vec<f32>> for FourierVecf32 {
    fn from(value: Vec<f32>) -> Self {
        FourierVecf32(value)
    }
}

#[allow(dead_code)]
pub trait Fourier {
    fn simple_dft(&mut self) -> Vec<Complex32>;
    fn dft(&mut self);
    fn fft(&mut self);
    fn ditfft(&mut self) -> Vec<Complex32>;
}

#[allow(dead_code)]
pub trait Window {
    // fn hamming();
    fn rectangular(&mut self, points: u32) -> anyhow::Result<Vec<f32>>;
}

#[allow(dead_code)]
impl Window for FourierTransform {
    fn rectangular(&mut self, points: u32) -> anyhow::Result<Vec<f32>> {
        Ok(self.bins.iter().map(|a| a.abs()).collect::<Vec<_>>())
    }
}

impl Fourier for Vec<f32> {
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
        todo!();
    }

    fn fft(&mut self) {
        todo!();
    }

    fn ditfft(&mut self) -> Vec<Complex32> {
        todo!();
    }
}

impl Into<Vec<Complex32>> for FourierVecf32 {
    fn into(self) -> Vec<Complex32> {
        self.0.iter().map(|a| Complex32::new(*a, 0.0)).collect()
    }
}

impl Fourier for FourierTransform {
    fn simple_dft(&mut self) -> Vec<Complex32> {
        let signal = self.signal.len();
        let mut temp: Vec<Complex32> = vec![Complex32::default(); signal as usize];

        (0..signal)
            .flat_map(|frequency_bin| (0..signal).map(move |sample| (frequency_bin, sample)))
            .for_each(|(frequency_bin, sample)| {
                let angle: f32 = -TAU * frequency_bin as f32 * sample as f32 / signal as f32;
                let complex: Complex32 = Complex32::new(angle.cos(), angle.sin());
                temp[frequency_bin] += self.signal[sample] * complex;
            });

        temp
    }

    /// Simple DFT
    fn dft(&mut self) {
        let signal = self.signal.len() as i32;
        self.bins = vec![Complex32::default(); signal as usize];

        (0..signal as usize)
            .flat_map(|frequency_bin| {
                (0..signal as usize).map(move |sample| (frequency_bin, sample))
            })
            .for_each(|(frequency_bin, sample)| {
                let angle: f32 = -TAU * frequency_bin as f32 * sample as f32 / signal as f32;
                let complex: Complex32 = Complex32::new(angle.cos(), angle.sin());
                self.bins[frequency_bin] += self.signal[sample] * complex;
            });
    }

    fn fft(&mut self) {
        todo!()
    }

    fn ditfft(&mut self) -> Vec<Complex32> {
        todo!()
    }
}

#[allow(non_snake_case)]
pub fn twiddle_precompute(N: usize) -> Vec<Complex32> {
    let W: Complex<f32> = Complex32::cis(-TAU / N as f32);

    let twiddle_factors = (0..(N / 2) - 1)
        .map(|n| W.powf(n as f32))
        .collect::<Vec<Complex32>>();

    return twiddle_factors;
}

// Notes for Decimation-in-time Fast Fourier Transform
// MAKE SURE INPUT IS A POWER OF 2 (128, 256, 512, 1024 points, etc.)
// check the size of the even and odd vectors, also adjust "for k" loop to place results in the right real-imaginary part of the array.
// the output Vec MUST be of a complex type! real part representing the cosine amplitudes, imaginary representing the sine amplitudes.
// change your variable names to lowercase for time-domain and uppercase for frequency-domain.
#[allow(non_snake_case)]
pub fn ditfft(x: &mut Vec<Complex32>) -> Vec<Complex<f32>> {
    let N = x.len();
    let W: Complex<f32> = Complex32::cis(-TAU / N as f32);
    // let mut out_vec: Vec<Complex32> = vec![Complex32::new(0f32, 0f32); N + 1];
    // out_vec.append(&mut x.clone());

    // println!("N: {:?}", N);

    if N <= 1 {
        return x.to_vec();
    }

    let (mut even, mut odds) = even_odd_split(x).unwrap();

    even = ditfft(&mut even);
    odds = ditfft(&mut odds);

    for k in 0..N / 2 {
        let t = W.powf(k as f32) * odds[k];
        x[k] = even[k] + t;
        x[k + N / 2] = even[k] - t;
    }

    return x.to_vec();
}

// This is basically what happens in the iterator but as for loops, code kept for reference
// for frequency_bin in 0..signal {
//     let mut sum: Complex32 = Complex32::default();

//     for sample in 0..signal {
//         // if inverse.unwrap() {
//         //     let angle: f32 = TAU * a as f32 * b as f32 / size as f32;
//         // }
//         let angle: f32 = -TAU * frequency_bin as f32 * sample as f32 / signal as f32;
//         let complex: Complex32 = Complex32::new(angle.cos(), angle.sin());

//         sum += self.signal[sample as usize] * complex;

//         // if inverse.unwrap() {
//         //     sum.im /= size as f32;
//         //     sum.re /= size as f32;
//         // }
//         // println!("i: {a} j: {b} angle: {angle}");
//     }
//     self.bins.push(sum);
// }

// quick and dirty implementation of dft
// reference code from https://rosettacode.org/wiki/Discrete_Fourier_transform - C example
// pub fn dft(input: &mut Vec<f32>, inverse: Option<bool>) -> Option<Vec<Complex32>> {
//     let size = input.len() as i32;
//     let mut temp: Vec<Complex32> = Vec::new();

//     for a in 0..size {
//         let mut sum: Complex32 = Complex32::default();
//         for b in 0..size {
//             let angle: f32 = -TAU * a as f32 * b as f32 / size as f32;
//             let complex: Complex32 = Complex32::new(angle.cos(), angle.sin());

//             sum += input[b as usize] * complex;
//         }
//         temp.push(sum);
//     }
//     Some(temp)
// }
