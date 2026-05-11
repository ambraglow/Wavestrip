use crate::dsp::utils::even_odd_split;
use crate::dsp::*;

#[derive(Clone)]
pub struct FourierTransform<T: Copy> {
    pub bins: Vec<Complex32>,
    pub split_bins: (Vec<T>, Vec<T>),
    pub signal: Vec<f32>,
    pub twiddle_factors_table: Vec<Complex32>,
}

impl FourierTransform<Complex32> {
    pub fn new() -> Self {
        FourierTransform {
            bins: Vec::new(),
            split_bins: (Vec::new(), Vec::new()),
            signal: Vec::new(),
            twiddle_factors_table: Vec::new(),
        }
    }

    pub fn twiddle_precompute(&mut self, n: usize) -> Vec<Complex32> {
        let w: Complex<f32> = Complex32::cis(-TAU / n as f32);

        let twiddle_factors = (0..n).map(|n| w.powf(n as f32)).collect::<Vec<Complex32>>();

        return twiddle_factors;
    }

    pub fn add_zero_padding(&mut self, n: usize) {
        // self.signal
        // .append(&mut vec![0f32; ZERO_PADDING - FRACTION_STEP]);
        // self.bins = self.signal.clone();
    }
}

#[allow(dead_code)]
pub trait Fourier {
    fn simple_dft(&mut self) -> Vec<Complex32>;
    fn dft(&mut self) -> Vec<Complex32>;
    fn fft(&mut self) -> Vec<Complex32>;
    fn ditfft(&mut self, twiddle_factors_table: &Vec<Complex32>) -> Vec<Complex32>;
}

impl Fourier for FourierTransform<Complex32> {
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
    fn dft(&mut self) -> Vec<Complex32> {
        let signal = self.signal.len() as i32;
        self.bins = vec![Complex32::default(); signal as usize].into();

        (0..signal as usize)
            .flat_map(|frequency_bin| {
                (0..signal as usize).map(move |sample| (frequency_bin, sample))
            })
            .for_each(|(frequency_bin, sample)| {
                let angle: f32 = -TAU * frequency_bin as f32 * sample as f32 / signal as f32;
                let complex: Complex32 = Complex32::new(angle.cos(), angle.sin());
                self.bins[frequency_bin] += self.signal[sample] * complex;
            });

        return Vec::new();
    }
    fn fft(&mut self) -> Vec<num_complex::Complex<f32>> {
        todo!()
    }
    fn ditfft(&mut self, twiddle_factors_table: &Vec<Complex32>) -> Vec<Complex32> {
        todo!()
    }
}

pub fn twiddle_precompute(n: usize) -> Vec<Complex32> {
    let w: Complex<f32> = Complex32::cis(-TAU / n as f32);

    let mut twiddle_factors = (0..n / 2)
        .map(|n| w.powf(n as f32))
        .collect::<Vec<Complex32>>();

    // let (mut a, _) = even_odd_split(&mut twiddle_factors).unwrap();
    // let mut n: usize = a.len();
    // twiddle_factors.append(&mut a);
    // let mut a = twiddle_factors.clone();
    let mut twiddle_n: usize = twiddle_factors.len();
    while twiddle_n != 1 {
        println!("{n:?}");
        let (mut a, _) = even_odd_split(&mut twiddle_factors).unwrap();
        let mut meow = a.iter().map(|a| w.powc(*a)).collect::<Vec<_>>();
        twiddle_factors.append(&mut meow);

        while a.len() < twiddle_factors.len() {
            (a, _) = even_odd_split(&mut a).unwrap();
            let mut meow = a.iter().map(|a| w.powc(*a)).collect::<Vec<_>>();
            twiddle_factors.append(&mut meow);
            println!("{:?}", a.len());
            if a.len() <= 1 {
                let mut meow = a.iter().map(|a| w.powc(*a)).collect::<Vec<_>>();
                twiddle_factors.append(&mut meow);
                break;
            }
        }
        twiddle_n = a.len();

        // let mut meow = a.iter().map(|a| W.powc(*a)).collect::<Vec<_>>();
        // twiddle_factors.append(&mut meow);
    }

    return twiddle_factors;
}

// Notes for Decimation-in-time Fast Fourier Transform
// MAKE SURE INPUT IS A POWER OF 2 (128, 256, 512, 1024 points, etc.)
// check the size of the even and odd vectors, also adjust "for k" loop to place results in the right real-imaginary part of the array.
// the output Vec MUST be of a complex type! real part representing the cosine amplitudes, imaginary representing the sine amplitudes.
// change your variable names to lowercase for time-domain and uppercase for frequency-domain.
#[allow(non_snake_case)]
pub fn ditfft(x: &mut Vec<Complex32>, twiddle_factors_table: &Vec<Complex32>) -> Vec<Complex<f32>> {
    let N = x.len();
    let fft_N = twiddle_factors_table.len();
    // let W: Complex<f32> = Complex32::cis(-TAU / N as f32);
    // let mut out_vec: Vec<Complex32> = vec![Complex32::new(0f32, 0f32); N + 1];
    // out_vec.append(&mut x.clone());

    // println!("N: {:?}", N);

    if N <= 1 {
        return x.to_vec();
    }

    let (mut even, mut odds) = even_odd_split(x).unwrap();

    even = ditfft(&mut even, &twiddle_factors_table);
    odds = ditfft(&mut odds, &twiddle_factors_table);

    let mut t: Complex32;
    for k in 0..N / 2 {
        if N / 2 != fft_N / 2 {
            // println!("{:?} {N:?} {:?}", k, FRACTION_STEP - N + k);
            t = twiddle_factors_table[fft_N % fft_N / 2] * odds[k];
        } else {
            // 0..1024
            // println!("{:?} {N:?} {:?}", k, FRACTION_STEP - N + k);
            t = twiddle_factors_table[k] * odds[k];
        }

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
