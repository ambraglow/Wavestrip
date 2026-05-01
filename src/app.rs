use anyhow::{Ok, Result};
use dear_imgui_rs::{ConfigFlags, Context};
use dear_imgui_sdl3::init_for_opengl;
use dear_implot::PlotContext;
use sdl3::video::GLContext;

use crate::fft::{
    complex::Sine,
    dft::{self, Dft},
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
}
