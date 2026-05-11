use anyhow::Ok;
use dear_imgui_rs::*;
use dear_imgui_sdl3::{process_sys_event, sdl3_poll_event_ll};
use dear_implot::*;
use num_complex::{Complex32, ComplexFloat};
use std::num::NonZero;

use rodio_experiments::Decoder;
use rodio_experiments::nz;
use rodio_experiments::{DynamicSourceExt, Source, buffer::SamplesBuffer};
use rustfft::num_traits::ToPrimitive;
use sdl3::sys::events::SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED;
use sdl3::{
    EventPump,
    sys::events::{SDL_EVENT_QUIT, SDL_EVENT_WINDOW_CLOSE_REQUESTED},
};
use std::thread;
use std::time::Duration;
use std::vec;
use std::{
    convert::TryFrom,
    ops::Not,
    thread::{Builder, JoinHandle},
    time::Instant,
    u64,
};

use crate::app::App;
use crate::app::vkash::*;
use crate::dsp::windows;
use crate::dsp::{fourier::Fourier, utils::PowerOfTwo, windows::Window};

impl App<Vec<Complex32>, Vec<f32>> {
    pub fn playback_thread(&mut self) -> anyhow::Result<JoinHandle<()>> {
        let tx_clone = self.channels.0.clone();
        let tray = self.tray.clone();

        let file = std::fs::File::open(tray.filename.lock().unwrap().to_string()).unwrap();
        let stream = Decoder::try_from(file).unwrap();
        let source_sample_rate = NonZero::new(stream.sample_rate().get()).unwrap();
        let stream_samples = stream
            .resample_into_fixed_source(source_sample_rate, nz!(1))
            .collect::<Vec<f32>>();

        let meow = Builder::new()
            .name("fft".to_string())
            .spawn(move || {
                let step_interval = (tray.fft_step.load(std::sync::atomic::Ordering::SeqCst)
                    / source_sample_rate.get() as usize)
                    * 1000;

                let samples_buffer =
                    SamplesBuffer::new(nz!(1), source_sample_rate, stream_samples.clone());

                let mut mixer = rodio_experiments::DeviceSinkBuilder::open_default_sink().unwrap();
                mixer.log_on_drop(false);

                let player = rodio_experiments::Player::connect_new(mixer.mixer());
                player.append(samples_buffer);

                let mut last_sample: u64 = 0;
                let mut pos_sample: u64 = 0;

                tray.status.store(true, std::sync::atomic::Ordering::SeqCst);

                loop {
                    player.play();

                    while !player.empty() {
                        if tray.status.load(std::sync::atomic::Ordering::SeqCst) == false {
                            break;
                        }
                        let player_position = player.get_pos();
                        while player_position.as_secs() <= player_position.as_secs() + 1 {
                            let fraction_step: usize =
                                tray.fft_step.load(std::sync::atomic::Ordering::SeqCst);

                            pos_sample = (player.get_pos().as_millis().to_u64().unwrap()
                                * source_sample_rate.get() as u64)
                                / 1000;
                            thread::sleep(Duration::from_millis(step_interval as u64 + 2));

                            if pos_sample > last_sample {
                                let len = pos_sample.to_usize().unwrap()
                                    - last_sample.to_usize().unwrap();

                                if len < fraction_step {
                                    break;
                                } else {
                                    let mut a = stream_samples[last_sample.to_usize().unwrap()
                                        ..pos_sample.to_usize().unwrap()]
                                        .to_vec()
                                        .iter()
                                        .take(fraction_step)
                                        .map(|a| *a)
                                        .collect::<Vec<f32>>();

                                    if a.iter().all(|a| *a != 0.0) {
                                        match tray
                                            .fft_window
                                            .load(std::sync::atomic::Ordering::SeqCst)
                                        {
                                            0 => None,
                                            1 => Some(a.blackman()),
                                            3 => Some(a.hanning()),
                                            4 => Some(a.rectangular(52)),
                                            _ => None,
                                        };

                                        // a.append(&mut vec![0f32; 2048 - fraction_step]);

                                        let b: Vec<f32> = a.clone();

                                        let mut out = a
                                            .iter()
                                            .map(|a| Complex32::new(*a, 0.0))
                                            .collect::<Vec<Complex32>>();

                                        out.fft();

                                        log::debug!(
                                            "points spacing: {:?} out len: {:?}",
                                            (source_sample_rate.get() as u64 / out.len() as u64),
                                            out.len()
                                        );

                                        tx_clone.send((out, b)).unwrap();
                                    }
                                }

                                log::debug!("{:?} {:?}", pos_sample, last_sample);
                            }
                            last_sample = pos_sample.clone();
                        }
                    }
                    if tray.status.load(std::sync::atomic::Ordering::SeqCst) == false {
                        break;
                    }
                    if player.empty() {
                        println!("player is empty!");
                        tray.start_playback_thread
                            .store(false, std::sync::atomic::Ordering::SeqCst);
                        break;
                    }
                }
            })
            .unwrap();

        println!("thread: {:?}", meow.thread().id());

        // println!("joinhandle: {:?}", meow);

        Ok(meow)
    }

    pub fn main_loop(&mut self) -> anyhow::Result<()> {
        let mut ft = (vec![Complex32::new(0.0, 0.0); 1], vec![0f32; 1]);
        let file = std::fs::File::open(self.tray.filename.lock().unwrap().to_string()).unwrap();
        let stream = Decoder::try_from(file).unwrap();
        let source_sample_rate = stream.sample_rate();
        drop(stream);

        let _ = self.init_external_texture();

        let mut cfg = dear_imgui_rs::Theme::default();
        cfg.colors = vec![ColorOverride {
            id: StyleColor::PlotLines,
            rgba: [1.0, 0.747, 0.0, 1.0],
        }];
        cfg.apply_to_context(&mut self.imgui.context);

        dear_implot::push_colormap(Colormap::Dark);

        println!("init passed!");

        self.imgui
            .context
            .io_mut()
            .set_display_size([1200.0, 720.0]);

        let _event_pump: EventPump = self.sdl_context.event_pump()?;
        'main: loop {
            let now = Instant::now();
            let dt = (now - self.imgui.last_frame).as_secs_f32();
            self.imgui.last_frame = now;
            self.imgui.context.io_mut().set_delta_time(dt);

            // 1) Poll SDL3 events and feed ImGui
            while let Some(event) = sdl3_poll_event_ll() {
                if process_sys_event(&event) {
                    // ImGui consumed the event; continue if you do not need it.
                }
                // Handle your own events or quit logic as needed...
                match event.event_type() {
                    SDL_EVENT_QUIT => {
                        break 'main;
                    }
                    SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED => unsafe {
                        if event.window.windowID == self.window.id() {
                            let (w, h) = self.window.size_in_pixels();
                            if w > 0 && h > 0 {
                                self.vk.swapchain_dirty = true;
                            }
                        }
                    },
                    SDL_EVENT_WINDOW_CLOSE_REQUESTED => unsafe {
                        if event.window.windowID == self.window.id() {
                            break 'main;
                        }
                    },
                    _default => (),
                }
            }

            // println!("before the playback if");
            if self
                .tray
                .start_playback_thread
                .load(std::sync::atomic::Ordering::SeqCst)
                == true
            {
                if self.second_thread.is_none() {
                    println!("spawned dft thread");
                    match self.playback_thread() {
                        std::result::Result::Ok(handle) => {
                            self.second_thread = Some(handle);
                            self.tray
                                .start_playback_thread
                                .store(false, std::sync::atomic::Ordering::SeqCst);
                        }
                        Err(_) => println!("new glenn level explosion"),
                    }
                } else {
                    println!("joining other thread and starting another");
                    self.tray
                        .status
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                    match self.playback_thread() {
                        std::result::Result::Ok(handle) => {
                            self.second_thread = Some(handle);
                            self.tray
                                .start_playback_thread
                                .store(false, std::sync::atomic::Ordering::SeqCst);
                        }
                        Err(_) => log::debug!("new glenn level explosion"),
                    }
                }
            } else {
                if self.second_thread.is_some() {
                    match self.channels.1.try_recv() {
                        std::result::Result::Ok(msg) => ft = msg,
                        Err(_) => log::debug!("no message received"),
                    }
                }
                // println!(
                //     "start playback thread {:?}",
                //     self.tray
                //         .start_playback_thread
                //         .load(std::sync::atomic::Ordering::SeqCst)
                // );
            }
            // println!("after the playback if");

            let mut step = self.tray.fft_step.load(std::sync::atomic::Ordering::SeqCst) as i32;
            let mut window_butt = self
                .tray
                .fft_window
                .load(std::sync::atomic::Ordering::SeqCst);

            dear_imgui_sdl3::sdl3_new_frame(&mut self.imgui.context);

            self.update_texture();
            let ui = self.imgui.context.frame();
            ui.dockspace_over_main_viewport();

            let plot_ui = ui.implot(&self.imgui.plot_context);

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
                let mut _show_demo_window = self
                    .tray
                    .show_demo_window
                    .load(std::sync::atomic::Ordering::SeqCst);
                ui.menu("Menu", || {
                    if ui.menu_item("Show demo window") {
                        _show_demo_window = _show_demo_window.not();
                    }
                });
                self.tray
                    .show_demo_window
                    .store(_show_demo_window, std::sync::atomic::Ordering::SeqCst);
            }

            if self
                .tray
                .show_demo_window
                .load(std::sync::atomic::Ordering::SeqCst)
                == true
            {
                ui.show_demo_window(
                    &mut self
                        .tray
                        .show_demo_window
                        .load(std::sync::atomic::Ordering::SeqCst),
                );
                dear_implot::show_demo_window(
                    &mut self
                        .tray
                        .show_demo_window
                        .load(std::sync::atomic::Ordering::SeqCst),
                );
            }

            let mut _playback = self
                .tray
                .start_playback_thread
                .load(std::sync::atomic::Ordering::SeqCst);

            ui.window("Settings")
                .flags(window_flags)
                .position(pos, Condition::Always)
                .size(size, Condition::Always)
                .build(|| {
                    let group_token = ui.begin_group();
                    ui.text(format!(
                        "source audio sample rate: {:?}",
                        source_sample_rate.get()
                    ));
                    ui.slider_i32("fft_size", &mut step, 1, 2048);
                    step.round_to_2n();
                    ui.separator();
                    // ui.checkbox("windowing", &mut window_butt);
                    if let Some(combo_box) =
                        ui.begin_combo("FFT window", windows::Types::VALUES[window_butt])
                    {
                        for (i, a) in windows::Types::VALUES.iter().enumerate() {
                            let _a = ui.selectable(a);
                            if _a {
                                window_butt = i;
                            }
                        }
                        combo_box.end();
                    };
                    ui.separator();
                    if ui.button("play") {
                        _playback = !_playback;
                    }
                    ui.separator();
                    group_token.end();

                    if let std::result::Result::Ok(token) = SubplotGrid::new("Plots", 2, 2)
                        .with_size([1200.0, 600.0])
                        .with_flags(SubplotFlags::NO_TITLE)
                        .begin()
                    {
                        let x = (0..ft.1.len()).map(|a| a as f64).collect::<Vec<_>>();

                        let audio_1 = ft.1.iter().map(|a| *a as f64).collect::<Vec<_>>();

                        let dft_size = ft.0.len();
                        // println!("dft size {:?}", dft_size);

                        if dft_size > 2 {
                            let mut frequency_bins = ft.0[1..ft.0.len() / 2]
                                .iter()
                                .map(|a| a.abs().powf(2.0).log10() as f64 * 10f64)
                                .collect::<Vec<_>>();

                            let max_db = frequency_bins
                                .iter()
                                .copied()
                                .max_by(|a, b| {
                                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                                })
                                .unwrap();

                            frequency_bins = frequency_bins
                                .iter()
                                .map(|a| *a - max_db)
                                .collect::<Vec<f64>>();

                            let dft_x: Vec<f64> = (0..frequency_bins.len() as i32)
                                .map(|a| {
                                    (source_sample_rate.get() as f64 * a as f64) / dft_size as f64
                                })
                                .collect();

                            if let Some(plot_token) = plot_ui.begin_plot("Radix-2 FFT") {
                                plot_ui.setup_y_axis(
                                    YAxis::Y1,
                                    "Magnitude (dB)".into(),
                                    AxisFlags::AUTO_FIT,
                                );
                                plot_ui.setup_x_axis(
                                    XAxis::X1,
                                    "Frequency log".into(),
                                    AxisFlags::AUTO_FIT,
                                );
                                plot_ui.setup_y_axis_limits(
                                    YAxis::Y1,
                                    -150.0,
                                    20.0,
                                    PlotCond::Always,
                                );
                                // plot_ui.setup_x_axis_scale(XAxis::X1, 2);
                                LinePlot::new("", &dft_x, &frequency_bins).plot();
                                plot_token.end();
                            }
                            plot_ui.set_next_y_axis_limits(
                                YAxis::Y1,
                                -2.25,
                                2.25,
                                PlotCond::Always,
                            );
                            if let Some(plot_token) = plot_ui.begin_plot("Audio") {
                                plot_ui.setup_x_axis(XAxis::X1, "".into(), AxisFlags::AUTO_FIT);
                                // plot_ui.setup_y_axis(YAxis::Y1, "".into(), AxisFlags::AUTO_FIT);
                                LinePlot::new("", &x, &audio_1).plot();
                                plot_token.end();
                            }
                            let mut window_data = vec![1f32; audio_1.len()];
                            window_data.blackman();
                            // window_data.iter_mut().for_each(|a| *a = a);

                            if let Some(plot_token) = plot_ui.begin_plot("Window data") {
                                plot_ui.setup_x_axis(XAxis::X1, "".into(), AxisFlags::AUTO_FIT);
                                // plot_ui.setup_y_axis(YAxis::Y1, "".into(), AxisFlags::AUTO_FIT);
                                LinePlot::new(
                                    "",
                                    &x,
                                    &window_data.iter().map(|a| *a as f64).collect::<Vec<f64>>(),
                                )
                                .plot();
                                plot_token.end();
                            }
                            // plot_ui
                            //     .heatmap_plot("rust is a spectrum", &frequency_bins, n_freq, n_freq)
                            //     .unwrap();
                        }

                        token.end();
                    }
                });

            self.tray
                .start_playback_thread
                .store(_playback, std::sync::atomic::Ordering::SeqCst);

            self.tray
                .fft_step
                .store(step as usize, std::sync::atomic::Ordering::SeqCst);

            self.tray
                .fft_window
                .store(window_butt, std::sync::atomic::Ordering::SeqCst);

            {
                let draw_data = self.imgui.context.render();
                let clear_color = self.imgui.clear_color;
                render_main_window(
                    &mut self.vk,
                    &mut self.imgui.renderer,
                    &self.window,
                    clear_color,
                    &draw_data,
                )
                .unwrap();
            }
        }

        self.destroy_external_texture();
        Ok(())
    }
}
