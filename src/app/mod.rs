use crate::app::vkash::*;
use anyhow::{Ok, Result};
use ash::khr::{surface as khr_surface, swapchain as khr_swapchain};
use ash::vk::{API_VERSION_1_3, KHR_PORTABILITY_SUBSET_NAME};
use ash::{Device, Entry, Instance, vk};
use dear_imgui_ash::AshRenderer;
use dear_imgui_rs::Context;
use dear_implot::PlotContext;
use sdl3::sys::init::SDL_Quit;
use std::error::Error;
use std::ffi::CString;
use std::matches;
use std::time::Instant;
use std::vec;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize},
        mpsc::{Receiver, Sender},
    },
    thread::JoinHandle,
};

#[allow(dead_code)]
mod structs;
#[allow(dead_code)]
pub mod vkash;

pub struct AppTray {
    pub player: Arc<Mutex<rodio_experiments::Player>>,
    pub mixer: Arc<Mutex<rodio_experiments::MixerDeviceSink>>,
    pub filename: Arc<Mutex<String>>,
    pub status: AtomicBool,
    pub fft_step: AtomicUsize,
    pub fft_window: AtomicUsize,
    pub show_demo_window: AtomicBool,
    pub start_playback_thread: AtomicBool,
}

impl AppTray {
    pub fn new() -> Self {
        let filename = Arc::new(Mutex::new("./assets/last-moan.m4a".to_string()));

        let mixer = Arc::new(Mutex::new(
            rodio_experiments::DeviceSinkBuilder::open_default_sink().unwrap(),
        ));

        let player = Arc::new(Mutex::new(rodio_experiments::Player::connect_new(
            mixer.lock().unwrap().mixer(),
        )));

        AppTray {
            player,
            mixer,
            filename,
            status: AtomicBool::new(true),
            fft_step: AtomicUsize::new(1024),
            fft_window: AtomicUsize::new(1),
            show_demo_window: AtomicBool::new(false),
            start_playback_thread: AtomicBool::new(false),
        }
    }
}

pub struct App<T: Clone, E: Clone> {
    pub vk: VulkanState,
    pub imgui: ImguiState,
    pub sdl_context: sdl3::Sdl,
    pub sdl_video: sdl3::VideoSubsystem,
    pub window: sdl3::video::Window,
    pub channels: (Sender<(T, E)>, Receiver<(T, E)>),
    pub second_thread: Option<JoinHandle<()>>,
    pub tray: Arc<AppTray>,
}

impl<T: Clone, E: Clone> Drop for App<T, E> {
    fn drop(&mut self) {
        unsafe {
            SDL_Quit();
        }
        dear_imgui_sdl3::shutdown(&mut self.imgui.context);
        self.destroy_external_texture();
    }
}

impl<T: Clone, E: Clone> App<T, E> {
    pub fn destroy_external_texture(&mut self) {
        let Some(external) = self.imgui.external.take() else {
            return;
        };

        self.imgui.renderer.unregister_texture(external.tex_id);

        unsafe {
            self.vk
                .ctx
                .device
                .destroy_sampler(external.sampler_nearest, None);
            self.vk
                .ctx
                .device
                .destroy_sampler(external.sampler_linear, None);
            self.vk
                .ctx
                .device
                .destroy_image_view(external.image_view, None);
            self.vk.ctx.device.destroy_image(external.image, None);
            self.vk.ctx.device.free_memory(external.image_mem, None);
        }
    }
}
