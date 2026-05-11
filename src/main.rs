use anyhow::Ok;

mod app;
mod fft;
mod tasks;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    // let mut r1: f32 = 1000f32;
    // let mut r2: f32 = 1000f32;
    // let mut sine_samples: i32 = 1024;
    // let sine_frequency = Arc::new(Mutex::new(500f32));
    // let v1: Vec<f32> = (0u8..=10).map(f32::from).collect::<Vec<_>>();
    dear_imgui_sdl3::enable_native_ime_ui();

    let mut application = app::App::new()?;
    application.main_loop()?;

    Ok(())
}
