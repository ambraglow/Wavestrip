use anyhow::{Ok, Result};
use dear_imgui_rs::{
    sys::{igColorConvertU32ToFloat4, igPushStyleVar_Float, ImGuiCol_PlotLines},
    *,
};
use dear_imgui_sdl3::{
    enable_native_ime_ui, init_for_opengl, new_frame, process_sys_event, render,
    sdl3_poll_event_ll, shutdown_for_opengl, update_texture,
};
use dear_implot::*;
use gl::*;
use log::{debug, error, info, log_enabled, Level};
use num_complex::{Complex32, ComplexFloat};
use sdl3::{
    event::Event,
    gamepad::Axis,
    sys::{
        events::{SDL_Event, SDL_EVENT_KEY_DOWN, SDL_EVENT_QUIT},
        init::SDL_Quit,
        keyboard::SDL_GetKeyName,
        keycode::{self, SDLK_ESCAPE, SDLK_LGUI},
        log::SDL_Log,
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
    ops::{Not, Sub},
    thread::JoinHandle,
    time::{Instant, SystemTime},
};
use std::{fs::File, thread};
use std::{ops::Add, sync::Mutex};
use std::{sync::mpsc::channel, vec};
use std::{sync::Arc, time::Duration};

use rodio_experiments::{
    buffer::SamplesBuffer, const_source::mixed::IntoMixed, generators::const_source,
    DynamicSourceExt, FixedSource,
};
use rodio_experiments::{
    effects::automatic_gain_control::AutomaticGainControlSettings, speakers::SpeakersBuilder,
};
use rodio_experiments::{fixed_source::mixed::IntoMixed as OtherIntoMixed, Decoder};
use rodio_experiments::{fixed_source::FixedSourceExt, nz};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use crate::fft::{
    complex::Sine,
    dft::{self, ditfft, ifft_test, Dft, FourierTransform},
};

mod app;
mod fft;

pub trait PowerOfTwo {
    fn is_power_of_two(&mut self) -> bool;
    fn round_to_2n(&mut self) -> Self;
}

impl PowerOfTwo for i32 {
    fn is_power_of_two(&mut self) -> bool {
        let a = self.clone();
        if a & (a - 1) == 0 {
            true
        } else {
            false
        }
    }
    fn round_to_2n(&mut self) -> Self {
        let mut a: i32 = self.clone();
        a -= 1;
        a |= a >> 1;
        a |= a >> 2;
        a |= a >> 4;
        a |= a >> 8;
        a |= a >> 16;
        a += 1;
        *self = a;
        return *self;
    }
}

fn handle_keys(event: &SDL_Event) -> anyhow::Result<()> {
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

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut r1: f32 = 1000f32;
    let mut r2: f32 = 1000f32;
    let mut sine_samples: i32 = 1024;
    let mut sample_rate: u32 = 96000;
    let sine_frequency = Arc::new(Mutex::new(500f32));
    let v1: Vec<f32> = (0u8..=10).map(f32::from).collect::<Vec<_>>();
    let mut application = app::Application::new()?;
    let mut ft_lock: FourierTransform = FourierTransform::new();
    // let ft = fft::dft::FourierTransform::new();

    // println!("{:?}", samples.len());

    // let mut samples = samples.iter().map(|a| *a).collect::<Vec<f32>>();

    // Initialize SDL3 + OpenGL3 backends
    init_for_opengl(
        &mut application.imgui_cx,
        &application.window,
        &application.gl_context,
        "#version 150",
    )?;

    let (tx, rx) = mpsc::channel::<FourierTransform>();
    // let tx_clone = tx.clone();

    // tx.send(ft);
    // ft.lock().unwrap().signal.append(&mut samples);

    let testdft = thread::spawn(move || {
        let thread_spawn_time = SystemTime::now();
        println!("spawned dft thread - spawn time: {thread_spawn_time:?}");
        let mut ft: FourierTransform = FourierTransform::new();
        let handle = rodio_experiments::DeviceSinkBuilder::open_default_sink()
            .expect("open default audio stream");

        let file = File::open("./song.mp3").unwrap();
        let sample_rate = 16384;
        let samples = Decoder::try_from(file)
            .unwrap()
            .resample_into_fixed_source(nz!(16384), nz!(1))
            .automatic_gain_control(AutomaticGainControlSettings::default())
            .take_duration(Duration::from_secs(60))
            .collect::<Vec<_>>();
        ft.signal = samples.iter().map(|a| *a).collect::<Vec<f32>>();
        let samples_duration_in_seconds: u64 = ft.signal.len() as u64 / sample_rate as u64;

        let player = rodio_experiments::Player::connect_new(&handle.mixer());
        let last_time = Instant::now();

        loop {
            for sample_chunk in samples.chunks(sample_rate) {
                let loop_start = Instant::now();
                let sample_source: SamplesBuffer =
                    SamplesBuffer::new(nz!(1), nz!(16384), sample_chunk);
                ft.signal = sample_chunk.iter().map(|a| *a).collect::<Vec<_>>();

                if last_time.elapsed().as_secs() != samples_duration_in_seconds {
                    player.append(sample_source);
                    player.play();
                    let mut scary_vec = sample_chunk
                        .iter()
                        .map(|a| Complex32::new(*a, 0.0))
                        .collect::<Vec<_>>();

                    // if loop_start.elapsed().as_secs() == 1 {
                    ft.bins = ditfft(&mut scary_vec);
                    // }
                    tx.send(ft.clone()).unwrap();
                    // println!(
                    //     "{:?} {:?} {:?} {:?}",
                    //     loop_start.elapsed().as_millis(),
                    //     last_time.elapsed().as_secs(),
                    //     SystemTime::now(),
                    //     samples_duration_in_seconds
                    // );
                    thread::sleep(Duration::from_millis(
                        loop_start.elapsed().as_millis() as u64
                    ));
                } else {
                    player.pause();
                    thread::sleep(Duration::from_millis(1000));
                    println!("kick a chicken!!");
                }
            }
        }
        // return ft.simple_dft(&mut ft_signal);
    });

    let event_pump: EventPump = application.sdl_cx.event_pump()?;
    'main: loop {
        if application.quit == true {
            unsafe {
                shutdown_for_opengl(&mut application.imgui_cx);
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
                    application.quit = true;
                }
                SDL_EVENT_KEY_DOWN => handle_keys(&event)?,
                default => (),
            }
        }

        // 2) Start a new frame for the SDL3 + OpenGL backends
        new_frame(&mut application.imgui_cx);

        ft_lock = rx.recv()?;

        // println!("{:?}", ft_lock.bins);
        // ft_lock.signal.append(&mut samples);

        let mut cfg = dear_imgui_rs::Theme::default();
        cfg.colors = vec![ColorOverride {
            id: StyleColor::Text,
            rgba: [1.0, 0.747, 0.0, 1.0],
        }];
        cfg.apply_to_context(&mut application.imgui_cx);

        let ui = application.imgui_cx.frame();
        let plot_ui = ui.implot(&application.plot_cx);

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
            let mut _show_demo_window = application.show_demo_window.clone();
            ui.menu("Menu", || {
                if ui.menu_item("Show demo window") {
                    _show_demo_window = _show_demo_window.not();
                }
            });
            application.show_demo_window = _show_demo_window;
        }

        if application.show_demo_window == true {
            ui.show_demo_window(&mut application.show_demo_window);
        }

        // let mut test_sine = Sine::new(sine_frequency.lock().unwrap().clone(), sample_rate)
        //     .take(sine_samples.clone() as usize)
        //     .map(|e| e as f64)
        //     .collect::<Vec<_>>();

        // let mut sine2 = Sine::new(2000f32, sample_rate)
        //     .take(sine_samples.clone() as usize)
        //     .map(|e| e as f64)
        //     .collect::<Vec<_>>();

        // let test_sine = test_sine
        //     .iter_mut()
        //     .zip(&mut sine2)
        //     .map(|(a, b)| *a + *b)
        //     .collect::<Vec<_>>();

        // ft_lock.signal = test_sine
        //     .iter()
        //     .map(|meow| *meow as f32)
        //     .collect::<Vec<_>>();

        // ft_lock.signal = samples
        //     .lock()
        //     .unwrap()
        //     .iter()
        //     .map(|meow| *meow as f32)
        //     .collect::<Vec<_>>();

        // let ratio: Vec<f64> = v1
        //     .iter()
        //     .map(|vin| vin * (r1 / (r1 + r2)))
        //     .map(f64::from)
        //     .collect();

        ui.window("Settings")
            .flags(window_flags)
            .position(pos, Condition::Always)
            .size(size, Condition::Always)
            .build(|| {
                // if let Some(token) = plot_ui.begin_plot("voltage divider") {
                //     LinePlot::new(
                //         "meow",
                //         &v1.iter().copied().map(f64::from).collect::<Vec<f64>>(),
                //         &ratio,
                //     )
                //     .plot();
                //     token.end();
                // }
                // ui.separator();
                // ui.slider_f32("R1", &mut r1, 0f32, 5000f32);
                // ui.slider_f32("R2", &mut r2, 0f32, 10000f32);
                // ui.separator();

                ui.slider_f32(
                    "frequency",
                    &mut sine_frequency.lock().unwrap(),
                    0f32,
                    20000f32,
                );
                ui.slider_i32("number of samples", &mut sine_samples, 32, 4096);
                if sine_samples.is_power_of_two().not() {
                    println!("not a power of two {sine_samples:?}");
                    sine_samples.round_to_2n();
                }

                if let std::result::Result::Ok(token) = SubplotGrid::new("Plots", 2, 2)
                    .with_size([1200.0, 600.0])
                    .with_flags(SubplotFlags::NO_TITLE)
                    .begin()
                {
                    let x = (0..ft_lock.signal.len())
                        .map(|a| a as f64)
                        .collect::<Vec<_>>();

                    let audio_signal = ft_lock.signal.iter().map(|a| *a as f64).collect::<Vec<_>>();

                    let dft_size = ft_lock.bins.len();

                    let dft_bins = ft_lock
                        .bins
                        .split_at(dft_size / 2)
                        .0
                        .iter()
                        .map(|a| a.im.abs() as f64)
                        .collect::<Vec<_>>();

                    // let ifft = ifft_test(&mut ft_lock.bins);
                    // println!("{:?} {:?}", ifft.len(), ifft);
                    // let ifft = ifft.iter().map(|a| a.re as f64).collect::<Vec<_>>();

                    // let ifft_x: Vec<f64> = (0..ifft.len() as i32)
                    //     .map(|a| a as f64 / sample_rate as f64)
                    //     .collect();

                    let dft_x: Vec<f64> = (0..dft_bins.len() as i32)
                        .map(|a| (sample_rate as f64 * a as f64) / dft_bins.len() as f64 / 2f64)
                        .collect();

                    plot_ui.set_next_axes_limits(0.0, 10000.0, 0.0, 300.0, PlotCond::Always);
                    if let Some(plot_token) = plot_ui.begin_plot("Radix-2 FFT") {
                        plot_ui.setup_y_axis(YAxis::Y1, "Magnitude".into(), AxisFlags::AUTO_FIT);
                        plot_ui.setup_x_axis(XAxis::X1, "Frequency".into(), AxisFlags::AUTO_FIT);
                        LinePlot::new("", &dft_x, &dft_bins).plot();
                        plot_token.end();
                    }

                    if let Some(plot_token) = plot_ui.begin_plot("signal") {
                        // plot_ui.setup_y_axis(YAxis::Y1, "Amplitude".into(), AxisFlags::AUTO_FIT);
                        LinePlot::new("", &x, &audio_signal).plot();
                        plot_token.end();
                    }
                    // if let Some(plot_token) = plot_ui.begin_plot("inverse fft") {
                    //     LinePlot::new("", &ifft_x, &ifft).plot();
                    //     plot_token.end();
                    // }
                    token.end();
                }
            });

        // 4) Render via OpenGL backend
        unsafe {
            gl::load_with(|s: &'static str| {
                std::mem::transmute(application.sdl_video.gl_get_proc_address(s))
            });
            ClearColor(0.0, 0.0, 0.0, 1.0); // Set clear color to black
            Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
        }
        let draw_data = application.imgui_cx.render();
        render(&draw_data);
        application.window.gl_swap_window();
    }

    Ok(())
}
