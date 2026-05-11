use anyhow::Ok;

mod app;
mod dsp;
mod tasks;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    tracy_client::Client::start();
    dear_imgui_sdl3::enable_native_ime_ui();

    let mut application = app::App::new()?;
    application.main_loop()?;

    Ok(())
}
