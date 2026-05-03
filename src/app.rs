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
use num_complex::{Complex32, ComplexFloat};
use rodio_experiments::{
    buffer::SamplesBuffer, const_source::mixed::IntoMixed, generators::const_source,
    DynamicSourceExt, FixedSource,
};
use rodio_experiments::{
    effects::automatic_gain_control::AutomaticGainControlSettings, speakers::SpeakersBuilder,
};
use rodio_experiments::{fixed_source::mixed::IntoMixed as OtherIntoMixed, Decoder};
use rodio_experiments::{fixed_source::FixedSourceExt, nz};
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
    ops::{Not, Sub},
    thread::JoinHandle,
    time::{Instant, SystemTime},
};
use std::{fs::File, thread};
use std::{ops::Add, sync::Mutex};
use std::{sync::mpsc::channel, vec};
use std::{sync::Arc, time::Duration};

use crate::fft::{
    complex::Sine,
    fourier::{self, Dft},
};
pub struct Application {
    pub imgui_cx: dear_imgui_rs::Context,
    pub plot_cx: PlotContext,
    pub sdl_cx: sdl3::Sdl,
    pub sdl_video: sdl3::VideoSubsystem,
    pub window: sdl3::video::Window,
    pub gl_context: GLContext,
    pub quit: bool,
    pub show_demo_window: bool,
}

impl Application {
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

        Ok(Application {
            imgui_cx: _imgui_cx,
            plot_cx: _plot_cx,
            sdl_cx: _sdl,
            sdl_video: _video,
            window,
            gl_context,
            quit: false,
            show_demo_window: false,
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
    pub fn main_loop(&mut self) -> anyhow::Result<()> {
        init_for_opengl(
            &mut self.imgui_cx,
            &self.window,
            &self.gl_context,
            "#version 150",
        )?;

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

            // 2) Start a new frame for the SDL3 + OpenGL backends
            new_frame(&mut self.imgui_cx);

            ft_lock = rx.recv()?;

            // println!("{:?}", ft_lock.bins);
            // ft_lock.signal.append(&mut samples);

            let mut cfg = dear_imgui_rs::Theme::default();
            cfg.colors = vec![ColorOverride {
                id: StyleColor::Text,
                rgba: [1.0, 0.747, 0.0, 1.0],
            }];
            cfg.apply_to_context(&mut self.imgui_cx);

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
            }

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

                    // ui.slider_f32(
                    //     "frequency",
                    //     &mut sine_frequency.lock().unwrap(),
                    //     0f32,
                    //     20000f32,
                    // );
                    // ui.slider_i32("number of samples", &mut sine_samples, 32, 4096);
                    // if sine_samples.is_power_of_two().not() {
                    //     println!("not a power of two {sine_samples:?}");
                    //     sine_samples.round_to_2n();
                    // }

                    if let std::result::Result::Ok(token) = SubplotGrid::new("Plots", 2, 2)
                        .with_size([1200.0, 600.0])
                        .with_flags(SubplotFlags::NO_TITLE)
                        .begin()
                    {
                        let x = (0..ft_lock.1.len()).map(|a| a as f64).collect::<Vec<_>>();

                        let audio_signal = ft_lock.1.iter().map(|a| *a as f64).collect::<Vec<_>>();

                        let dft_size = ft_lock.0.len();

                        let dft_bins = ft_lock
                            .0
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
                            .map(|a| (16384 as f64 * a as f64) / dft_bins.len() as f64 / 2f64)
                            .collect();

                        plot_ui.set_next_axes_limits(0.0, 10000.0, 0.0, 900.0, PlotCond::Once);
                        if let Some(plot_token) = plot_ui.begin_plot("Radix-2 FFT") {
                            plot_ui.setup_y_axis(
                                YAxis::Y1,
                                "Magnitude".into(),
                                AxisFlags::AUTO_FIT,
                            );
                            plot_ui.setup_x_axis(XAxis::X1, "Frequency".into(), AxisFlags::INVERT);
                            LinePlot::new("", &dft_x, &dft_bins).plot();
                            plot_token.end();
                        }

                        if let Some(plot_token) = plot_ui.begin_plot("signal") {
                            plot_ui.setup_y_axis(
                                YAxis::Y1,
                                "Amplitude".into(),
                                AxisFlags::AUTO_FIT,
                            );
                            LinePlot::new("", &x, &audio_signal).plot();
                            plot_token.end();
                        }
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
