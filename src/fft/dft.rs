use anyhow::{Ok, Result};
use num_complex::{Complex, Complex32, ComplexFloat};
use rustfft::{algorithm::Radix4, Fft};
use sdl3::libc::signal;
use std::{
    f32::consts::TAU,
    f64::consts::E,
    ops::{Add, Sub},
    vec,
};

use crate::fft::{complex::Sine, dft};

#[derive(Clone)]
pub struct FourierTransform {
    pub bins: Vec<Complex32>,
    pub signal: Vec<f32>,
    pub X: Vec<f32>,
}

impl FourierTransform {
    pub fn new() -> Self {
        FourierTransform {
            bins: Vec::new(),
            signal: Vec::new(),
            X: Vec::new(),
        }
    }
}

#[allow(dead_code)]
pub trait Dft {
    fn simple_dft(&mut self) -> Vec<Complex32>;
    fn dft(&mut self);
    fn fft(&mut self);
    fn ditfft(&mut self) -> (Vec<Complex32>);
}

#[allow(dead_code)]
pub trait Window {
    // fn hamming();
    fn rectangular(&mut self, points: u32);
}

#[allow(dead_code)]
impl Window for FourierTransform {
    fn rectangular(&mut self, points: u32) {
        let mut magnitude: Vec<f32> = self.bins.iter().map(|a| a.abs()).collect();
    }
}

impl Dft for Vec<Complex32> {
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

impl Dft for Vec<f32> {
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

impl Dft for FourierTransform {
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

    fn ditfft(&mut self) -> (Vec<Complex32>) {
        todo!()
    }

    // Incomplete radix-2 FFT implementation
    // fn fft(&mut self) {
    //     let signal_length: usize = self.signal.len();
    //     if signal_length != 1 && signal_length % 2 != 0 {
    //         println!("is not multiple of 2");
    //     }
    //     self.bins = vec![Complex32::default(); signal_length as usize];

    //     let (mut even, mut odd) = split(self).unwrap();

    //     let mut recombined_dft = even
    //         .iter()
    //         .zip(odd)
    //         .enumerate()
    //         .map(|(index, (dft_even, dft_odd))| {
    //             let t = E.powf(-TAU * index as f32 / dft_odd);
    //             // for k in self.signal {}
    //         })
    //         .collect::<Vec<_>>();
    // }
}

// pub fn dumbfft() -> Vec<Complex32> {
//     let mut signal: Vec<f32> = Sine::new(1000f32, 96000).collect();
//     // = signal.iter().map(|a|);
//     let n = signal.len();
//     let widdle_factor = Complex32::cis(-TAU / n as f32);

//     if signal > 1 {

//     }
// }

pub fn ifft_test(x: &mut Vec<Complex32>) -> Vec<Complex32> {
    let mut _buffer = vec![Complex::new(0f32, 0f32); x.len()];
    _buffer = x.clone();
    let ifft = Radix4::new(x.len(), rustfft::FftDirection::Inverse);
    ifft.process(&mut _buffer);

    _buffer
}

// Notes for Decimation-in-time Fast Fourier Transform
// MAKE SURE INPUT IS A POWER OF 2 (128, 256, 512, 1024 points, etc.)
// check the size of the even and odd vectors, also adjust "for k" loop to place results in the right real-imaginary part of the array.
// the output Vec MUST be of a complex type! real part representing the cosine amplitudes, imaginary representing the sine amplitudes.
// change your variable names to lowercase for time-domain and uppercase for frequency-domain.

pub fn ditfft(x: &mut Vec<Complex32>) -> Vec<Complex<f32>> {
    let N = x.len();
    let W: Complex<f32> = Complex32::cis(-TAU / N as f32);
    // let mut out_vec: Vec<Complex32> = vec![Complex32::new(0f32, 0f32); N + 1];
    // out_vec.append(&mut x.clone());

    // println!("N: {:?}", N);

    if N <= 1 {
        return x.to_vec();
    }

    let (mut even, mut odds) = split_cplx(x).unwrap();

    even = ditfft(&mut even);
    odds = ditfft(&mut odds);

    for k in 0..N / 2 {
        let t = W.powf(k as f32) * odds[k];
        x[k] = even[k] + t;
        x[k + N / 2] = even[k] - t;
    }

    return x.to_vec();
}

#[allow(dead_code)]
fn split_cplx(input: &mut Vec<Complex32>) -> Result<(Vec<Complex32>, Vec<Complex32>)> {
    let (even_indices, odd_indices): (Vec<_>, Vec<_>) =
        input.iter().enumerate().partition(|(i, _)| i % 2 == 0);

    let even: Vec<Complex32> = even_indices.into_iter().map(|(_, val)| *val).collect();
    let odd: Vec<Complex32> = odd_indices.into_iter().map(|(_, val)| *val).collect();

    Ok((even, odd))
}

#[allow(dead_code)]
fn split(input: &mut FourierTransform) -> Result<(Vec<f32>, Vec<f32>)> {
    let (even_indices, odd_indices): (Vec<_>, Vec<_>) =
        input.X.iter().enumerate().partition(|(i, _)| i % 2 == 0);

    let even: Vec<f32> = even_indices.into_iter().map(|(_, val)| *val).collect();
    let odd: Vec<f32> = odd_indices.into_iter().map(|(_, val)| *val).collect();

    Ok((even, odd))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sine;

    const SAMPLE_RATE: u32 = 500;
    const NR_SAMPLES: usize = 1000;

    fn signal_helper() -> Result<FourierTransform> {
        let mut dft = FourierTransform::new();
        let sine1: Vec<f32> = Sine::new(200f32, SAMPLE_RATE).take(NR_SAMPLES).collect();

        dft.signal = sine1.iter().zip(dft.signal).map(|(a, b)| a + b).collect();

        Ok(dft)
    }

    #[test]
    fn running_dft() {
        let mut dft = signal_helper().unwrap();
        let signal_len: usize = dft.signal.len();

        dft.dft();
        assert_eq!(signal_len, dft.bins.len());
    }

    #[test]
    fn splitting() {
        let mut dft = signal_helper().unwrap();
        let signal_len: usize = dft.signal.len();

        let mut even: Vec<_> = vec![f32::default(); signal_len];
        let mut odd: Vec<_> = vec![f32::default(); signal_len];

        (even, odd) = split(&mut dft).unwrap();

        let size_of_odd: usize = odd.len();
        let size_of_even: usize = even.len();
        let tot_size: usize = size_of_even + size_of_odd;

        assert_eq!(signal_len, tot_size);
    }
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
