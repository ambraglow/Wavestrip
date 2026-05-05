use anyhow::{Ok, Result};
use dear_imgui_rs::{
    sys::{igColorConvertU32ToFloat4, igPushStyleVar_Float, ImGuiCol_PlotLines},
    *,
};
use dear_imgui_rs::{ConfigFlags, Context};
use dear_imgui_sdl3::init_for_opengl;
use dear_imgui_sdl3::{
    enable_native_ime_ui, new_frame, process_sys_event, render, sdl3_poll_event_ll,
    shutdown_for_opengl, update_texture,
};
use dear_implot::PlotContext;
use dear_implot::*;
use gl::*;
use log::{debug, error, info, log_enabled, Level};
use num_complex::{Complex, Complex32, ComplexFloat};
use rand::seq::IteratorRandom;
use rodio_experiments::{
    buffer::SamplesBuffer, const_source::mixed::IntoMixed, generators::const_source,
    microphone::MicrophoneBuilder, source::AutomaticGainControlSettings, DynamicSourceExt,
    FixedSource, Player, Source,
};
use rodio_experiments::{effects::const_source::AutomaticGainControl, speakers::SpeakersBuilder};
use rodio_experiments::{fixed_source::mixed::IntoMixed as OtherIntoMixed, Decoder};
use rodio_experiments::{fixed_source::FixedSourceExt, nz};
use rustfft::num_traits::{ToPrimitive, Zero};
use sdl3::video::GLContext;
use sdl3::{
    event::Event,
    gamepad::Axis,
    sys::{
        events::{SDL_Event, SDL_EVENT_KEY_DOWN, SDL_EVENT_QUIT},
        init::SDL_Quit,
        keyboard::SDL_GetKeyName,
        keycode::{self, SDLK_ESCAPE, SDLK_LGUI},
        log::SDL_Log,
        metadata::iostream::METADATA_SDL_IOWhence,
        render::SDL_RenderClear,
        stdinc::SDL_log,
        video::SDL_GL_GetProcAddress,
    },
    video::GLProfile,
    EventPump,
};
use std::{
    convert::TryFrom,
    ffi::CString,
    ops::{Deref, Not, Sub},
    sync::mpsc::{self, Receiver, Sender},
    thread::{Builder, JoinHandle},
    time::{Instant, SystemTime},
    u64,
};
use std::{fs::File, thread};
use std::{ops::Add, sync::Mutex};
use std::{sync::atomic::AtomicBool, thread::Thread};
use std::{sync::mpsc::channel, vec};
use std::{sync::Arc, time::Duration};

use crate::fft::{
    complex::Sine,
    fourier::{self, ditfft, Fourier, FourierVecf32},
    utils::PowerOfTwo,
};
pub struct Application<T: Clone, E: Clone> {
    pub imgui_cx: dear_imgui_rs::Context,
    pub plot_cx: PlotContext,
    pub sdl_cx: sdl3::Sdl,
    pub sdl_video: sdl3::VideoSubsystem,
    pub window: sdl3::video::Window,
    pub gl_context: GLContext,
    pub channels: (Sender<(T, E)>, Receiver<(T, E)>),
    pub status: Arc<AtomicBool>,
    pub second_thread: Option<JoinHandle<()>>,
    pub quit: bool,
    pub show_demo_window: bool,
    pub start_playback_thread: bool,
}

const FRACTION_STEP: usize = 2048;

impl Application<Vec<Complex32>, Vec<f32>> {
    pub fn new() -> Result<Self> {
        let _sdl = sdl3::init()?;
        let _video = _sdl.video()?;
        {
            let gl_attr = _video.gl_attr();
            gl_attr.set_context_profile(sdl3::video::GLProfile::Core);
            gl_attr.set_context_version(3, 3);
        }
        let window = _video
            .window("thingymajig", 1280, 720)
            .high_pixel_density()
            .opengl()
            .resizable()
            .build()?;
        let gl_context = window.gl_create_context()?;

        window.gl_make_current(&gl_context)?;

        let mut _imgui_cx = dear_imgui_rs::Context::create();
        let _plot_cx = PlotContext::create(&_imgui_cx);
        let io = _imgui_cx.io_mut();
        let mut flags = io.config_flags();
        flags.insert(ConfigFlags::DOCKING_ENABLE);
        io.set_config_flags(flags);

        let channels = mpsc::channel::<(Vec<Complex32>, Vec<f32>)>();

        Ok(Application {
            imgui_cx: _imgui_cx,
            plot_cx: _plot_cx,
            sdl_cx: _sdl,
            sdl_video: _video,
            window,
            gl_context,
            channels,
            status: Arc::new(AtomicBool::new(true)),
            second_thread: None,
            quit: false,
            show_demo_window: false,
            start_playback_thread: true,
        })
    }
    pub fn handle_keys(&mut self, event: &SDL_Event) -> anyhow::Result<()> {
        unsafe {
            match event.key.key {
                default => SDL_Log(
                    "Key pressed: %s!".to_cstring_safe().unwrap().as_ptr(),
                    SDL_GetKeyName(event.key.key),
                ),
            }
        }
        Ok(())
    }
    pub fn playback_thread(&mut self) -> anyhow::Result<JoinHandle<()>> {
        let tx_clone = self.channels.0.clone();
        let atomic_status = self.status.clone();
        // println!("wruff");

        let meow = Builder::new()
            .name("fft".to_string())
            .spawn(move || {
                let file = std::fs::File::open("./daft-punk.mp3").unwrap();
                let stream = Decoder::try_from(file).unwrap();
                let source_sample_rate = stream.sample_rate();
                println!("{:?}", source_sample_rate);
                let step_interval =
                    (FRACTION_STEP / source_sample_rate.get().to_usize().unwrap()) * 1000;
                let stream_samples = stream
                    .automatic_gain_control(AutomaticGainControlSettings::default())
                    .resample_into_fixed_source(source_sample_rate, nz!(1))
                    .collect::<Vec<f32>>();

                let samples_buffer =
                    SamplesBuffer::new(nz!(1), source_sample_rate, stream_samples.clone());

                let mut mixer = rodio_experiments::DeviceSinkBuilder::open_default_sink().unwrap();
                let player = rodio_experiments::Player::connect_new(mixer.mixer());
                mixer.log_on_drop(false);
                player.append(samples_buffer);

                let mut last_sample: u64 = 0;
                let mut pos_sample: u64 = 0;

                loop {
                    player.play();
                    if player.empty() {
                        atomic_status.store(false, std::sync::atomic::Ordering::SeqCst);
                        break;
                    }
                    while !player.empty() {
                        if atomic_status.load(std::sync::atomic::Ordering::SeqCst) == false {
                            player.stop();
                            break;
                        };
                        let player_position = player.get_pos();
                        while player_position.as_secs() <= player_position.as_secs() + 1 {
                            // get the current sample index
                            pos_sample = (player.get_pos().as_millis().to_u64().unwrap()
                                * source_sample_rate.get().to_u64().unwrap())
                                / 1000;
                            thread::sleep(Duration::from_millis(step_interval as u64));
                            // check if the current sample index is greater than the last + 1024
                            if pos_sample > last_sample {
                                // println!("i'm not insane i swear");
                                let len = pos_sample.to_usize().unwrap()
                                    - last_sample.to_usize().unwrap();

                                if len < FRACTION_STEP {
                                    break;
                                }

                                let b = stream_samples[last_sample.to_usize().unwrap()
                                    ..pos_sample.to_usize().unwrap()]
                                    .to_vec()
                                    .clone();
                                let a = stream_samples[last_sample.to_usize().unwrap()
                                    ..pos_sample.to_usize().unwrap()]
                                    .to_vec()
                                    .iter()
                                    .take(FRACTION_STEP as usize)
                                    .map(|a| *a)
                                    .collect::<Vec<f32>>();

                                // println!("a length {:?}", a.len());
                                // attempt at zero-padding
                                // a.append(&mut vec![0f32; 8192 - FRACTION_STEP]);

                                let mut out = a
                                    .iter()
                                    .map(|a| Complex32::new(*a, 0.0))
                                    .collect::<Vec<Complex32>>();

                                // println!(
                                //     "points spacing: {:?} out len: {:?}",
                                //     (SAMPLE_RATE / out.len() as u64),
                                //     out.len()
                                // );

                                out.ditfft();

                                tx_clone.send((out, b)).unwrap();
                                // println!("{:?} {:?}", pos_sample, last_sample);
                            }
                            // clone current index into last_sample
                            last_sample = pos_sample.clone();
                        }
                    }
                }
            })
            .unwrap();

        Ok(meow)
    }

    pub fn main_loop(&mut self) -> anyhow::Result<()> {
        let mut ft_lock = (vec![Complex32::new(0.0, 0.0); 5], vec![0f32; 5]);

        let file = std::fs::File::open("./daft-punk.mp3").unwrap();
        let stream = Decoder::try_from(file).unwrap();
        let source_sample_rate = stream.sample_rate();

        drop(stream);

        init_for_opengl(
            &mut self.imgui_cx,
            &self.window,
            &self.gl_context,
            "#version 150",
        )?;

        let mut cfg = dear_imgui_rs::Theme::default();
        cfg.colors = vec![ColorOverride {
            id: StyleColor::PlotLines,
            rgba: [1.0, 0.747, 0.0, 1.0],
        }];
        cfg.apply_to_context(&mut self.imgui_cx);

        dear_implot::push_colormap(Colormap::Dark);

        println!("init passed!");

        let event_pump: EventPump = self.sdl_cx.event_pump()?;
        'main: loop {
            if self.quit == true {
                unsafe {
                    shutdown_for_opengl(&mut self.imgui_cx);
                    SDL_Quit();
                    return Ok(());
                }
            };
            // 1) Poll SDL3 events and feed ImGui
            while let Some(event) = sdl3_poll_event_ll() {
                if process_sys_event(&event) {
                    // ImGui consumed the event; continue if you do not need it.
                }
                // Handle your own events or quit logic as needed...
                match event.event_type() {
                    SDL_EVENT_QUIT => {
                        self.quit = true;
                    }
                    SDL_EVENT_KEY_DOWN => self.handle_keys(&event)?,
                    default => (),
                }
            }

            if self.start_playback_thread == true {
                if self.second_thread.is_none() {
                    self.second_thread = Some(self.playback_thread()?);
                    // self.start_playback_thread = false;
                } else {
                    if self.second_thread.is_some() {
                        // println!("blablalbalba");
                        ft_lock = self.channels.1.recv()?;
                    }
                }
                // ft_lock = self.channels.1.recv()?;
            } else {
                if let Some(thread) = self.second_thread.take() {
                    thread.join().expect("couldn't join thread");
                };
                self.start_playback_thread = false;
                self.status
                    .store(false, std::sync::atomic::Ordering::SeqCst);
            };

            // 2) Start a new frame for the SDL3 + OpenGL backends
            new_frame(&mut self.imgui_cx);

            // println!("{:?}", ft_lock.bins);
            // ft_lock.signal.append(&mut samples);

            let ui = self.imgui_cx.frame();
            let plot_ui = ui.implot(&self.plot_cx);

            let viewport = ui.main_viewport();
            // Ensure this window is associated with the main viewport (safe wrapper)
            ui.set_next_window_viewport(viewport.id().into());
            let pos = viewport.pos();
            let size = viewport.size();

            let window_flags: WindowFlags = WindowFlags::NO_COLLAPSE
                | WindowFlags::NO_RESIZE
                | WindowFlags::NO_MOVE
                | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
                | WindowFlags::NO_NAV_FOCUS;

            if let Some(_bar) = ui.begin_main_menu_bar() {
                let mut _show_demo_window = self.show_demo_window.clone();
                ui.menu("Menu", || {
                    if ui.menu_item("Show demo window") {
                        _show_demo_window = _show_demo_window.not();
                    }
                });
                self.show_demo_window = _show_demo_window;
            }

            if self.show_demo_window == true {
                ui.show_demo_window(&mut self.show_demo_window);
                dear_implot::show_demo_window(&mut self.show_demo_window);
            }

            let mut _playback = self.start_playback_thread.clone();
            if ui.button("play") {
                _playback = _playback.not();
            }
            self.start_playback_thread = _playback;
            self.status
                .store(_playback, std::sync::atomic::Ordering::SeqCst);

            ui.window("Settings")
                .flags(window_flags)
                .position(pos, Condition::Always)
                .size(size, Condition::Always)
                .build(|| {
                    if let std::result::Result::Ok(token) = SubplotGrid::new("Plots", 2, 1)
                        .with_size([1200.0, 600.0])
                        .with_flags(SubplotFlags::NO_TITLE)
                        .begin()
                    {
                        let x = (0..ft_lock.1.len()).map(|a| a as f64).collect::<Vec<_>>();

                        let audio_signal = ft_lock.1.iter().map(|a| *a as f64).collect::<Vec<_>>();

                        let dft_size = ft_lock.0.len();

                        let mut dft_bins = ft_lock
                            .0
                            .split_at(ft_lock.0.len() / 2 + 1)
                            .0
                            .iter()
                            .map(|a| a.abs().log10() as f64 * 20f64)
                            .collect::<Vec<_>>();

                        let max_db = dft_bins
                            .iter()
                            .copied()
                            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                            .unwrap();

                        dft_bins = dft_bins.iter().map(|a| *a - max_db).collect::<Vec<f64>>();

                        let dft_x: Vec<f64> = (0..dft_bins.len() as i32)
                            .map(|a| (source_sample_rate.get() as f64 * a as f64) / dft_size as f64)
                            .collect();

                        plot_ui.set_next_y_axis_limits(YAxis::Y1, -100.0, 10.0, PlotCond::Always);
                        plot_ui.set_next_x_axis_limits(
                            XAxis::X1,
                            0.0,
                            source_sample_rate.get() as f64 / 2.0,
                            PlotCond::Always,
                        );
                        if let Some(plot_token) = plot_ui.begin_plot("Radix-2 FFT") {
                            plot_ui.setup_y_axis(
                                YAxis::Y1,
                                "Magnitude".into(),
                                AxisFlags::AUTO_FIT,
                            );
                            // plot_ui.setup_x_axis(
                            //     XAxis::X1,
                            //     "Frequency".into(),
                            //     AxisFlags::AUTO_FIT,
                            // );
                            LinePlot::new("", &dft_x, &dft_bins).plot();
                            plot_token.end();
                        }

                        plot_ui.set_next_y_axis_limits(YAxis::Y1, -2.25, 2.25, PlotCond::Always);
                        if let Some(plot_token) = plot_ui.begin_plot("signal") {
                            plot_ui.setup_x_axis(
                                XAxis::X1,
                                "Amplitude".into(),
                                AxisFlags::AUTO_FIT,
                            );
                            LinePlot::new("", &x, &audio_signal).plot();
                            plot_token.end();
                        }

                        token.end();
                    }
                });

            // 4) Render via OpenGL backend
            unsafe {
                gl::load_with(|s: &'static str| {
                    std::mem::transmute(self.sdl_video.gl_get_proc_address(s))
                });
                ClearColor(0.0, 0.0, 0.0, 1.0); // Set clear color to black
                Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
            }
            let draw_data = self.imgui_cx.render();
            render(&draw_data);
            self.window.gl_swap_window();
        }
    }
}

// let ifft = ifft_test(&mut ft_lock.bins);
// println!("{:?} {:?}", ifft.len(), ifft);
// let ifft = ifft.iter().map(|a| a.re as f64).collect::<Vec<_>>();

// let ifft_x: Vec<f64> = (0..ifft.len() as i32)
//     .map(|a| a as f64 / sample_rate as f64)
//     .collect();

// if let Some(plot_token) = plot_ui.begin_plot("inverse fft") {
//     LinePlot::new("", &ifft_x, &ifft).plot();
//     plot_token.end();
// }

// if let Some(plot_token) = plot_ui.begin_plot("welcome to my spectrum") {
//     HeatmapPlot::new("Heat", &data, 10, 10)
//         .with_scale(-1.0, 1.0)
//         .with_bounds(0.0, 0.0, 1.0, 1.0)
//         .with_label_format(Some("%.2f"))
//         .plot();
//     plot_token.end();
// }
