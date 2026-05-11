use anyhow::Ok;
use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use dear_imgui_rs::*;
use dear_implot::*;
use num_complex::{Complex32, ComplexFloat};
use rayon::iter::*;
use rustfft::num_traits::ToPrimitive;
use sdl3::EventPump;
use sdl3::event::EventType::Last;
use sdl3::keyboard::Scancode::U;
use std::ops::Div;
use std::ops::Rem;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::vec;
use std::{
    ops::Not,
    thread::{Builder, JoinHandle},
    time::Instant,
    u64,
};
use tracy_client::span;

use crate::app::App;
use crate::app::AppTray;
use crate::app::vkash::*;
use crate::dsp::windows;
use crate::dsp::{fourier::Fourier, utils::PowerOfTwo, windows::Window};

type MainChannels = (
    Sender<(Vec<Complex32>, Vec<f32>)>,
    Receiver<(Vec<Complex32>, Vec<f32>)>,
);

impl App<Vec<Complex32>, Vec<f32>> {
    pub fn bench_thread(&mut self) -> anyhow::Result<JoinHandle<()>> {
        self.tray.update_stream()?;
        log::info!("stream updated");

        let tx_clone = self.channels.0.clone();
        let tray = self.tray.clone();
        let source_sample_rate = self.tray.playback.sample_rate.lock().unwrap().get();

        let meow = Builder::new()
            .name("fft".to_string())
            .spawn(move || {
                let span = span!("playback thread");

                let player = &tray.playback.player;

                let mut step_interval = (tray.fft_step.load(std::sync::atomic::Ordering::SeqCst)
                    as f32
                    / source_sample_rate as f32)
                    * 1000f32;

                let stream_samples = tray.playback.samples_buffer.lock().unwrap();

                let mut last_sample: u64 = 0;
                let mut sample_position: u64 = 0;

                let mut fraction_step: usize =
                    tray.fft_step.load(std::sync::atomic::Ordering::SeqCst);

                let mut player_position = player.get_pos();
                let mut last_player_position = player.get_pos();
                let mut sample_interval: usize = 0;

                let mut hop_interval = 0.0;
                let mut a: Vec<f32> = Vec::new();
                let mut audio_out: Vec<f32> = Vec::new();
                let mut out: Vec<Complex32> = Vec::new();

                tray.status.store(true, std::sync::atomic::Ordering::SeqCst);

                player.set_volume(0.1);

                loop {
                    let l = span!("loop iteration");
                    player.play();
                    while !player.empty() {
                        if tray.status.load(std::sync::atomic::Ordering::SeqCst) == false {
                            log::debug!(
                                "fft thread {:?} received stop signal",
                                thread::current().id()
                            );
                            player.stop();
                            break;
                        }
                        let player_loop = span!("player loop iteration");

                        thread::sleep(Duration::from_millis(hop_interval as u64));

                        fraction_step = tray.fft_step.load(std::sync::atomic::Ordering::SeqCst);
                        step_interval =
                            (fraction_step as f32 / source_sample_rate as f32) * 1000f32;

                        sample_position = (player.get_pos().as_millis().to_u64().unwrap()
                            * source_sample_rate as u64)
                            / 1000;

                        if sample_position > stream_samples.len() as u64 {
                            // stale position from the previous song — restart tracking
                            last_sample = 0;
                            sample_position = 0;
                        }

                        sample_interval =
                            sample_position.to_usize().unwrap() - last_sample.to_usize().unwrap();

                        let mut hop = (fraction_step).max(1);
                        if fraction_step >= 4096 {
                            hop = 1024;
                        }

                        hop_interval = (hop as f32 / source_sample_rate as f32) * 1000f32;

                        if sample_interval >= hop {
                            tray.playback.bins_buffer.lock().unwrap().clear();
                            // let _ = bg_channels.0.try_send(BgMsg::Forward { step_interval });

                            let mut window_buffer = tray.playback.window_buffer.lock().unwrap();
                            if window_buffer.len() != fraction_step {
                                window_buffer.resize(fraction_step, 0.0);
                            }

                            if sample_position > stream_samples.len() as u64 {
                                last_sample = 0;
                                sample_position = 0;
                            }

                            const MAX_FRAMES_PER_ROUND: usize = 5;
                            let mut produced = 0;

                            while produced < MAX_FRAMES_PER_ROUND
                                && last_sample as usize + hop <= sample_position as usize
                            {
                                let frame_start = last_sample as usize;
                                let frame_end = frame_start + hop;

                                if frame_end > stream_samples.len() {
                                    break;
                                }

                                a = stream_samples[frame_start..frame_end].to_vec();

                                for j in 0..(fraction_step - hop) {
                                    window_buffer[j] = window_buffer[j + hop];
                                }
                                for j in 0..hop {
                                    window_buffer[fraction_step - hop + j] = a[j];
                                }

                                a = window_buffer.clone();

                                match tray.fft_window.load(std::sync::atomic::Ordering::SeqCst) {
                                    0 => (),
                                    1 => a.blackman(),
                                    3 => a.hanning(),
                                    4 => a.rectangular(fraction_step / 2),
                                    _ => (),
                                };

                                audio_out = a.clone();

                                out = a
                                    .iter()
                                    .map(|a| Complex32::new(*a, 0.0))
                                    .collect::<Vec<Complex32>>();

                                out.ditfft(&mut Vec::new());

                                tray.playback.bins_buffer.lock().unwrap().push(out.clone());

                                log::debug!(
                                    "sending frame with start: {frame_start} - end: {frame_end} - size: {:?}", frame_end - frame_start
                                );
                                tx_clone.send((out.clone(), audio_out.clone())).unwrap();

                                produced += 1;
                                last_sample += hop as u64;
                            }

                            drop(window_buffer);
                        } else {
                            break;
                        }
                    }
                    if tray.status.load(std::sync::atomic::Ordering::SeqCst) == false {
                        log::debug!(
                            "fft thread {:?} received stop signal",
                            thread::current().id()
                        );
                        player.stop();
                        break;
                    }
                    if player.empty() {
                        log::debug!("player is empty!");
                        tray.start_playback
                            .store(false, std::sync::atomic::Ordering::SeqCst);
                        break;
                    }
                }
            })
            .unwrap();

        log::debug!(
            "thread spawned: {:?} {:?}",
            meow.thread().name().unwrap(),
            meow.thread().id()
        );

        Ok(meow)
    }

    pub fn main_loop(&mut self) -> anyhow::Result<()> {
        let _span = span!("main loop");

        let mut ft = (vec![Complex32::new(0.0, 0.0); 1], vec![0f32; 1]);
        let mut _current_filename = "".to_string();

        // self.init_external_texture()?;
        self.imgui_config();
        self.imgui.context.io_mut();

        let _event_pump: EventPump = self.sdl_context.event_pump()?;
        'main: loop {
            let now = Instant::now();
            let dt = (now - self.imgui.last_frame).as_secs_f32();
            self.imgui.last_frame = now;
            self.imgui.context.io_mut().set_delta_time(dt);

            if self.handle_events() {
                self.tray
                    .status
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                break 'main;
            }

            if self
                .tray
                .start_playback
                .load(std::sync::atomic::Ordering::SeqCst)
                == true
            {
                if self.second_thread.is_none() {
                    log::info!("spawning compute thread");

                    if let Err(e) = self.bench_thread().map(|(thread)| {
                        self.second_thread = Some(thread);
                        self.tray
                            .start_playback
                            .store(false, std::sync::atomic::Ordering::SeqCst);
                    }) {
                        if self.tray.status.load(std::sync::atomic::Ordering::SeqCst) != false {
                            log::error!("{}", e);
                            self.tray.playback.filename.lock().unwrap().clear();
                            self.tray
                                .playback
                                .filename
                                .lock()
                                .unwrap()
                                .push_str(format!("./assets/").as_str());

                            self.tray
                                .status
                                .store(false, std::sync::atomic::Ordering::SeqCst);
                        }
                    }
                } else {
                    log::info!("joining other thread and starting another");
                    self.tray
                        .status
                        .store(false, std::sync::atomic::Ordering::SeqCst);

                    match self.bench_thread() {
                        std::result::Result::Ok((main_handle)) => {
                            self.second_thread = Some(main_handle);
                            self.tray
                                .start_playback
                                .store(false, std::sync::atomic::Ordering::SeqCst);
                        }
                        Err(_) => panic!("new glenn level explosion"),
                    }
                }
            } else {
                if self.second_thread.is_some() {
                    if !self.tray.playback.player.empty() {
                        match self.channels.1.try_recv() {
                            std::result::Result::Ok(msg) => ft = msg,
                            Err(_) => log::debug!("no message received"),
                        }
                    }
                }
            }

            let mut step = self.tray.fft_step.load(std::sync::atomic::Ordering::SeqCst) as i32;
            let mut window_butt = self
                .tray
                .fft_window
                .load(std::sync::atomic::Ordering::SeqCst);

            dear_imgui_sdl3::sdl3_new_frame(&mut self.imgui.context);

            // self.update_texture();
            let ui = self.imgui.context.frame();
            ui.dockspace_over_main_viewport();

            let plot_ui = ui.implot(&self.imgui.plot_context);

            let viewport = ui.main_viewport();
            // Ensure this window is associated with the main viewport (safe wrapper)
            ui.set_next_window_viewport(viewport.id().into());
            let pos = viewport.pos();
            let size = viewport.size();

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
                .start_playback
                .load(std::sync::atomic::Ordering::SeqCst);

            ui.window("Settings")
                .flags(self.window_flags)
                .position(pos, Condition::FirstUseEver)
                .size(size, Condition::FirstUseEver)
                .build(|| {
                    let group_token = ui.begin_group();
                    ui.separator();
                    ui.input_text("file", self.tray.playback.filename.lock().as_mut().unwrap())
                        .build();
                    ui.separator();
                    let mut audio_len = self
                        .tray
                        .playback
                        .audio_length
                        .lock()
                        .unwrap()
                        .as_secs_f64();
                    ui.text(format!(
                        "progress: {:#?}:{:02?}/{:?}:{:?}",
                        self.tray.playback.player.get_pos().as_secs().div(60),
                        self.tray.playback.player.get_pos().as_secs_f64().rem(60f64) as u64,
                        audio_len.div(60f64) as u64,
                        audio_len.rem(60f64) as u64
                    ));
                    ui.text(format!(
                        "source audio sample rate: {:?}",
                        self.tray.playback.sample_rate.try_lock().unwrap().get()
                    ));
                    ui.separator();
                    ui.slider_i32("fft_size", &mut step, 1, 8192);
                    step.round_to_2n();
                    ui.separator();
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
                    if ui.button("stop") {
                        _playback = !_playback;
                    }
                    group_token.end();

                    // TODO: spectrogram;
                    // ui.image(&mut *self.img_tex, [256.0, 256.0]);
                });

            ui.window("Graphs and visualization")
                .flags(self.window_flags)
                .position(pos, Condition::FirstUseEver)
                .size(size, Condition::FirstUseEver)
                .build(|| {
                    let avail = ui.content_region_avail();

                    if let std::result::Result::Ok(token) = SubplotGrid::new("Plots", 2, 1)
                        .with_size(avail)
                        .with_flags(SubplotFlags::NO_TITLE)
                        .begin()
                    {
                        let x = (0..ft.1.len()).map(|a| a as f64).collect::<Vec<_>>();
                        let audio = ft.1.iter().map(|a| *a as f64).collect::<Vec<_>>();

                        let dft_size = ft.0.len();

                        if dft_size > 2 {
                            let _s = span!("fft data and graph");

                            let mut frequency_bins = ft.0[1..ft.0.len() / 4]
                                .iter()
                                .map(|a| a.abs().log10() as f64 * 20f64)
                                .collect::<Vec<_>>();

                            let dft_x: Vec<f64> = (0..frequency_bins.len() as i32)
                                .map(|a| {
                                    (self.tray.playback.sample_rate.try_lock().unwrap().get()
                                        as f64
                                        * a as f64)
                                        / dft_size as f64
                                })
                                .collect();

                            if let Some(plot_token) = plot_ui.begin_plot("Frequencies") {
                                plot_ui.setup_y_axis(
                                    YAxis::Y1,
                                    "Magnitude (dB)".into(),
                                    AxisFlags::AUTO_FIT,
                                );
                                plot_ui.setup_x_axis(
                                    XAxis::X1,
                                    "Frequencies (Hz)".into(),
                                    AxisFlags::AUTO_FIT,
                                );
                                plot_ui.setup_y_axis_limits(
                                    YAxis::Y1,
                                    -80.0,
                                    100.0,
                                    PlotCond::Always,
                                );
                                plot_ui.setup_x_axis_scale(XAxis::X1, 2);
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
                                LinePlot::new("", &x, &audio).plot();
                                plot_token.end();
                            }
                        }

                        token.end();
                    }
                });

            self.tray
                .start_playback
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
