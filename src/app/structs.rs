use crate::app::*;

use ash::vk;
use dear_imgui_ash::{AshRenderer, Options};
use dear_imgui_rs::{ConfigFlags, Context};
use dear_implot::PlotContext;
use num_complex::Complex32;

use sdl3::video::WindowPos;
use std::sync::Arc;
use std::vec;
use std::{
    sync::mpsc::{self},
    time::Instant,
};

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
        let tray = AppTray::new();

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
            second_thread: None,
            tray: Arc::new(tray),
        })
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

    pub fn update_texture(&mut self) {
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
}
