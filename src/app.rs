use anyhow::{Ok, Result};
use ash::vk;
use dear_imgui_ash::{AshRenderer, Options};
use dear_imgui_rs::*;
use dear_imgui_rs::{ConfigFlags, Context};
use dear_imgui_sdl3::{process_sys_event, sdl3_poll_event_ll};
use dear_implot::PlotContext;
use dear_implot::*;
use log::debug;
use num_complex::{Complex32, ComplexFloat};
use rodio_experiments::{Decoder, fixed_source::mixed::IntoMixed as OtherIntoMixed};
use rodio_experiments::{
    DynamicSourceExt, FixedSource, Player, Source,
    buffer::SamplesBuffer,
    source::{self, AutomaticGainControlSettings},
};
use rodio_experiments::{fixed_source::FixedSourceExt, nz};
use rustfft::num_traits::{ToPrimitive, Zero};
use sdl3::sys::vulkan::SDL_Vulkan_LoadLibrary;
use sdl3::{
    EventPump,
    event::Event,
    gamepad::Axis,
    sys::{
        events::{SDL_EVENT_KEY_DOWN, SDL_EVENT_QUIT, SDL_EVENT_WINDOW_CLOSE_REQUESTED, SDL_Event},
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
};
use sdl3::{
    sys::events::SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED,
    video::{GLContext, SwapInterval, VkInstance, WindowPos},
};
use std::{
    convert::TryFrom,
    f64::consts::{PI, TAU},
    ffi::CString,
    ops::{Deref, Not, Sub},
    sync::{
        atomic::AtomicUsize,
        mpsc::{self, Receiver, Sender},
    },
    thread::{Builder, JoinHandle},
    time::{Instant, SystemTime},
    u64,
};
use std::{fs::File, thread};
use std::{ops::Add, sync::Mutex};
use std::{sync::Arc, time::Duration};
use std::{sync::atomic::AtomicBool, thread::Thread};
use std::{sync::mpsc::channel, vec};

mod vkash;
use crate::{
    app::vkash::*,
    fft::{
        complex::Sine,
        fourier::{self, Fourier, FourierTransform, ditfft, twiddle_precompute},
        utils::PowerOfTwo,
        windows::Window,
    },
};
use vkash::{ImguiState, VulkanState};

pub static FRACTION_STEP: usize = 2048;

pub struct App<T: Clone, E: Clone> {
    pub vk: VulkanState,
    pub imgui: ImguiState,
    pub sdl_context: sdl3::Sdl,
    pub sdl_video: sdl3::VideoSubsystem,
    pub window: sdl3::video::Window,
    pub channels: (Sender<(T, E)>, Receiver<(T, E)>),
    pub status: Arc<AtomicBool>,
    pub second_thread: Option<JoinHandle<()>>,
    pub show_demo_window: bool,
    pub start_playback_thread: bool,
    pub fft_step: Arc<AtomicUsize>,
    pub fft_window: Arc<AtomicBool>,
}

impl<T: Clone, E: Clone> Drop for App<T, E> {
    fn drop(&mut self) {
        dear_imgui_sdl3::shutdown(&mut self.imgui.context);
        self.destroy_external_texture();
    }
}

impl<T: Clone, E: Clone> App<T, E> {
    fn destroy_external_texture(&mut self) {
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

impl App<Vec<Complex32>, Vec<f32>> {
    pub fn new() -> Result<Self> {
        let sdl_context = sdl3::init()?;
        let video = sdl_context.video()?;

        let main_scale = video
            .get_primary_display()?
            .get_content_scale()
            .unwrap_or(1.0);

        video.vulkan_load_library_default()?;

        let mut window = video
            .window(
                "thingymajig",
                (1200.0 * main_scale) as u32,
                (720.0 * main_scale) as u32,
            )
            .resizable()
            .high_pixel_density()
            .vulkan()
            .build()?;
        window.set_position(WindowPos::Centered, WindowPos::Centered);

        let ctx = VulkanContext::new(&window, "Thingimajig")?;
        let surface_format = pick_surface_format(&ctx)?;
        let render_pass = create_render_pass(&ctx.device, surface_format.format)?;
        let swapchain = SwapchainState::new(&ctx, &window, render_pass, surface_format)?;

        let mut imgui_context = Context::create();
        let plot_context = PlotContext::create(&imgui_context);

        {
            let io = imgui_context.io_mut();
            let mut flags = io.config_flags();
            flags.insert(ConfigFlags::DOCKING_ENABLE);
            io.set_config_flags(flags);

            let style = imgui_context.style_mut();
            style.set_font_scale_dpi(main_scale);
        }

        // SDL3 platform backend for Vulkan (sets Platform_CreateVkSurface for multi-viewport).
        dear_imgui_sdl3::init_for_vulkan(&mut imgui_context, &window)?;
        dear_imgui_sdl3::set_gamepad_mode(dear_imgui_sdl3::GamepadMode::AutoAll);

        // Create a managed ImGui texture (CPU-side pixels; backend will create GPU texture).
        let tex_w: u32 = 128;
        let tex_h: u32 = 128;
        let mut img_tex = dear_imgui_rs::texture::TextureData::new();
        img_tex.create(
            dear_imgui_rs::texture::TextureFormat::RGBA32,
            tex_w as i32,
            tex_h as i32,
        );
        let mut pixels = vec![0u8; (tex_w * tex_h * 4) as usize];
        for y in 0..tex_h {
            for x in 0..tex_w {
                let i = ((y * tex_w + x) * 4) as usize;
                pixels[i + 0] = (x as f32 / tex_w as f32 * 255.0) as u8;
                pixels[i + 1] = (y as f32 / tex_h as f32 * 255.0) as u8;
                pixels[i + 2] = 128;
                pixels[i + 3] = 255;
            }
        }
        img_tex.set_data(&pixels);
        img_tex.set_status(dear_imgui_rs::texture::TextureStatus::WantCreate);

        // Register user-created textures so renderer backends can see them via DrawData::textures().
        // This avoids TexID==0 assertions and lets the backend handle Create/Update/Destroy.
        let mut registered_user_textures = Vec::new();
        registered_user_textures.push(imgui_context.register_user_texture_token(&mut *img_tex));

        // Renderer.
        let framebuffer_srgb = is_srgb_format(swapchain.surface_format.format);
        let mut renderer = AshRenderer::with_default_allocator(
            &ctx.instance,
            ctx.physical_device,
            ctx.device.clone(),
            ctx.queue,
            ctx.command_pool,
            render_pass,
            &mut imgui_context,
            Some(Options {
                in_flight_frames: FRAMES_IN_FLIGHT,
                framebuffer_srgb,
                ..Default::default()
            }),
        )?;
        renderer.set_viewport_clear_color([0.1, 0.12, 0.15, 1.0]);

        // Frame sync objects.
        let frames = (0..FRAMES_IN_FLIGHT)
            .map(|_| create_frame_sync(&ctx.device, ctx.command_pool))
            .collect::<Result<Vec<_>, _>>()?;
        let images_in_flight = vec![vk::Fence::null(); swapchain.images.len()];

        let channels = mpsc::channel::<(Vec<Complex32>, Vec<f32>)>();
        Ok(App {
            imgui: ImguiState {
                _registered_user_textures: registered_user_textures,
                context: imgui_context,
                plot_context,
                renderer,
                last_frame: Instant::now(),
                clear_color: [0.1, 0.12, 0.15, 1.0],
                img_tex,
                tex_size: (tex_w, tex_h),
                frame: 0,
                external: None,
            },
            vk: VulkanState {
                ctx,
                render_pass,
                swapchain,
                frames,
                images_in_flight,
                frame_index: 0,
                swapchain_dirty: false,
            },
            sdl_context,
            sdl_video: video,
            window,
            channels,
            status: Arc::new(AtomicBool::new(true)),
            second_thread: None,
            show_demo_window: false,
            start_playback_thread: true,
            fft_step: Arc::new(AtomicUsize::new(1024)),
            fft_window: Arc::new(AtomicBool::new(true)),
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
    pub fn init_external_texture(&mut self) -> anyhow::Result<()> {
        if self.imgui.external.is_some() {
            return Ok(());
        }

        let external = create_external_rgba_texture(
            &self.vk.ctx.instance,
            self.vk.ctx.physical_device,
            &self.vk.ctx.device,
            self.vk.ctx.queue,
            self.vk.ctx.command_pool,
            &mut self.imgui.renderer,
        )
        .unwrap();
        self.imgui.external = Some(external);
        Ok(())
    }
    fn update_texture(&mut self) {
        let (w, h) = self.imgui.tex_size;
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let t = self.imgui.frame as f32 * 0.08;
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                let fx = x as f32 / w as f32;
                let fy = y as f32 / h as f32;
                pixels[i + 0] = ((fx * 255.0 + t.sin() * 128.0).clamp(0.0, 255.0)) as u8;
                pixels[i + 1] = ((fy * 255.0 + (t * 1.7).cos() * 128.0).clamp(0.0, 255.0)) as u8;
                pixels[i + 2] = (((fx + fy + t * 0.1).sin().abs()) * 255.0) as u8;
                pixels[i + 3] = 255;
            }
        }
        self.imgui.img_tex.set_data(&pixels);
        self.imgui.frame = self.imgui.frame.wrapping_add(1);
    }

    pub fn playback_thread(&mut self) -> anyhow::Result<JoinHandle<()>> {
        let tx_clone = self.channels.0.clone();
        let atomic_status = self.status.clone();
        let fft_step = self.fft_step.clone();
        let window = self.fft_window.clone();

        let meow = Builder::new()
            .name("fft".to_string())
            .spawn(move || {
                let file = std::fs::File::open("./assets/badapple.mp3").unwrap();
                let stream = Decoder::try_from(file).unwrap();
                let source_sample_rate = stream.sample_rate();

                let step_interval = (fft_step.load(std::sync::atomic::Ordering::SeqCst)
                    / source_sample_rate.get() as usize)
                    * 1000;

                let stream_samples = stream
                    // .automatic_gain_control(AutomaticGainControlSettings {
                    //     target_level: 2.0,
                    //     ..Default::default()
                    // })
                    .resample_into_fixed_source(source_sample_rate, nz!(1))
                    .collect::<Vec<f32>>();

                let samples_buffer =
                    SamplesBuffer::new(nz!(1), source_sample_rate, stream_samples.clone());

                let mut mixer = rodio_experiments::DeviceSinkBuilder::open_default_sink().unwrap();
                mixer.log_on_drop(false);

                let player = rodio_experiments::Player::connect_new(mixer.mixer());
                player.append(samples_buffer);

                let mut last_sample: u64 = 0;
                let mut pos_sample: u64 = 0;

                loop {
                    player.play();
                    if player.empty() {
                        log::debug!("player is empty!");
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
                            let fraction_step: usize =
                                fft_step.load(std::sync::atomic::Ordering::SeqCst);

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
                                        if window.load(std::sync::atomic::Ordering::SeqCst) {
                                            a.blackman();
                                            a.append(&mut vec![0f32; 2048 - fraction_step]);
                                        }

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
                }
            })
            .unwrap();

        Ok(meow)
    }

    pub fn main_loop(&mut self) -> anyhow::Result<()> {
        let mut ft = (vec![Complex32::new(0.0, 0.0); 1], vec![0f32; 1]);
        let file = std::fs::File::open("./assets/badapple.mp3").unwrap();
        let stream = Decoder::try_from(file).unwrap();
        let total_duration = stream.total_duration().unwrap().as_secs();
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

        let event_pump: EventPump = self.sdl_context.event_pump()?;
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
                    SDL_EVENT_KEY_DOWN => self.handle_keys(&event)?,
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
                    default => (),
                }
            }

            if self.start_playback_thread == true {
                if self.second_thread.is_none() {
                    self.second_thread = Some(self.playback_thread()?);
                } else {
                    if self.second_thread.is_some() {
                        ft = self.channels.1.recv()?;
                    }
                }
            } else {
                if let Some(thread) = self.second_thread.take() {
                    thread.join().expect("couldn't join thread");
                };
                self.start_playback_thread = false;
                self.status
                    .store(false, std::sync::atomic::Ordering::SeqCst);
            };

            let mut step = self.fft_step.load(std::sync::atomic::Ordering::SeqCst) as i32;
            let mut window_butt = self.fft_window.load(std::sync::atomic::Ordering::SeqCst);

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
            ui.text(format!(
                "source audio sample rate: {:?}",
                source_sample_rate.get()
            ));

            ui.window("Settings")
                .flags(window_flags)
                .position(pos, Condition::Always)
                .size(size, Condition::Always)
                .build(|| {
                    let group_token = ui.begin_group();
                    ui.slider_i32("fft_size", &mut step, 1, 2048);
                    step.round_to_2n();
                    ui.separator();
                    ui.checkbox("windowing", &mut window_butt);
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

                        if dft_size > 1 {
                            let mut frequency_bins = ft.0[1..ft.0.len() / 2]
                                .iter()
                                .map(|a| a.abs().powf(2.0).log10() as f64 * 10f64)
                                .collect::<Vec<_>>();

                            // let mut frequency_bins = ft.0[1..ft.0.len() / 2]
                            //     .iter()
                            //     .map(|a| a.abs() as f64)
                            //     .collect::<Vec<_>>();

                            let bins_len = frequency_bins.len();
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

            self.status
                .store(_playback, std::sync::atomic::Ordering::SeqCst);

            self.fft_step
                .store(step as usize, std::sync::atomic::Ordering::SeqCst);

            self.fft_window
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
