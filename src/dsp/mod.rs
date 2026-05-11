use anyhow::Ok;
use num_complex::{Complex, Complex32};
use rand::RngExt;
use rustfft::{Fft, algorithm::Radix4};
use std::f32::consts::{PI, TAU};

#[allow(dead_code)]
pub mod complex;

#[allow(dead_code)]
pub mod fourier;

#[allow(dead_code)]
pub mod sine;

#[allow(dead_code)]
pub mod utils;

#[allow(dead_code)]
pub mod windows;
