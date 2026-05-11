use crate::app::vkash::*;
use anyhow::{Context as anyhow_context, Ok, Result};
use ash::khr::{surface as khr_surface, swapchain as khr_swapchain};
use ash::vk;
use ash::vk::{API_VERSION_1_3, KHR_PORTABILITY_SUBSET_NAME};
use ash::{Device, Entry, Instance};
use dear_imgui_ash::AshRenderer;
use dear_imgui_ash::Options;
use dear_imgui_rs::{ConfigFlags, Context, WindowFlags};
use dear_implot::PlotContext;
use num_complex::Complex32;
use rodio_experiments::buffer::SamplesBuffer;
use rodio_experiments::{Decoder, DynamicSourceExt, Source, nz};
use sdl3::sys::init::SDL_Quit;
use sdl3::video::WindowPos;
use std::error::Error;
use std::ffi::CString;
use std::fs::File;
use std::io::BufReader;
use std::num::NonZero;
use std::time::{Duration, Instant};
use std::vec;
use std::{matches, mem};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize},
        mpsc::{Receiver, Sender},
    },
    thread::JoinHandle,
};

use std::sync::mpsc::{self};

#[allow(dead_code)]
pub mod app_impl;
#[allow(dead_code)]
pub mod tray_impl;
#[allow(dead_code)]
pub mod vkash;

pub struct App<T: Clone, E: Clone> {
    pub vk: VulkanState,
    pub imgui: ImguiState,
    pub sdl_context: sdl3::Sdl,
    pub sdl_video: sdl3::VideoSubsystem,
    pub window: sdl3::video::Window,
    pub channels: (
        crossbeam::channel::Sender<(T, E)>,
        crossbeam::channel::Receiver<(T, E)>,
    ),
    pub second_thread: Option<JoinHandle<()>>,
    pub tray: Arc<AppTray>,
    pub img_tex: dear_imgui_rs::OwnedTextureData,
    pub window_flags: WindowFlags,
}

pub struct AppTray {
    pub playback: Arc<Playback>,
    pub status: AtomicBool,
    pub fft_step: AtomicUsize,
    pub fft_window: AtomicUsize,
    pub show_demo_window: AtomicBool,
    pub start_playback_thread: AtomicBool,
}
pub struct Playback {
    pub player: rodio_experiments::Player,
    pub mixer: rodio_experiments::MixerDeviceSink,
    pub decoder: Mutex<Decoder<BufReader<File>>>,
    pub samples_buffer: Mutex<Vec<f32>>,
    pub bins_buffer: Mutex<Vec<Vec<Complex32>>>,
    pub filename: Mutex<String>,
    pub sample_rate: Mutex<NonZero<u32>>,
    pub audio_length: Mutex<Duration>,
}
