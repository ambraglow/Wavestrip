use anyhow::{Ok, Result};
use sdl3::libc::ARPOP_REPLY;

use crate::fft::{
    complex::Sine,
    fourier::{self, ditfft, twiddle_precompute, FourierTransform},
};
use num_complex::Complex32;
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
    // let sine_frequency = Arc::new(Mutex::new(500f32));
    let v1: Vec<f32> = (0u8..=10).map(f32::from).collect::<Vec<_>>();

    dear_imgui_sdl3::enable_native_ime_ui();

    let mut application = app::App::new()?;
    application.main_loop()?;

    // dear_imgui_ash::multi_viewport_sdl3::enable(
    //     &mut application.imgui.renderer,
    //     &mut application.imgui.context,
    //     application.vk.ctx.entry.clone(),
    //     application.vk.ctx.instance.clone(),
    //     application.vk.ctx.physical_device,
    //     application.vk.ctx.queue,
    //     application.vk.ctx.queue_family_index,
    //     application.vk.ctx.queue_family_index,
    // );

    Ok(())
}
