use anyhow::{Ok, Result};

use crate::fft::{
    complex::Sine,
    fourier::{self, ditfft, twiddle_precompute, Dft, FourierTransform},
};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::*;
use std::time::Instant;

mod app;
mod fft;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut r1: f32 = 1000f32;
    let mut r2: f32 = 1000f32;
    let mut sine_samples: i32 = 1024;
    let sine_frequency = Arc::new(Mutex::new(500f32));
    let v1: Vec<f32> = (0u8..=10).map(f32::from).collect::<Vec<_>>();
    let mut application = app::Application::new()?;
    let mut ft_lock = (vec![Complex32::new(0.0, 0.0); 5], vec![0f32; 5]);

    application.main_loop()?;

    // let (tx, rx) = mpsc::channel::<(Vec<Complex32>, Vec<f32>)>();

    Ok(())
}
