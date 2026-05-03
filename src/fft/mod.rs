use anyhow::{Ok, Result};
use core::panic::PanicInfo;
use num_complex::{Complex, Complex32, ComplexFloat};
use rustfft::{algorithm::Radix4, Fft};
use sdl3::libc::signal;
use std::{
    f32::consts::TAU,
    f64::consts::E,
    ops::{Add, Sub},
    vec,
};

pub mod complex;
pub mod float;
pub mod fourier;
pub mod utils;
