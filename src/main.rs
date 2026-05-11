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

// loop {
//     // let meow = application.playback_thread();

//     log::info!("starting fft with size N: 512");
//     application
//         .tray
//         .fft_step
//         .store(512, std::sync::atomic::Ordering::SeqCst);
//     let a = application.bench_thread()?;
//     thread::sleep(Duration::from_secs(5));
//     application
//         .tray
//         .status
//         .store(false, std::sync::atomic::Ordering::SeqCst);
//     a.join().unwrap();

//     log::info!("starting fft with size N: 1024");
//     application
//         .tray
//         .fft_step
//         .store(1024, std::sync::atomic::Ordering::SeqCst);
//     let a = application.bench_thread()?;
//     thread::sleep(Duration::from_secs(5));
//     application
//         .tray
//         .status
//         .store(false, std::sync::atomic::Ordering::SeqCst);
//     a.join().unwrap();

//     log::info!("starting fft with size N: 2048");
//     application
//         .tray
//         .fft_step
//         .store(2048, std::sync::atomic::Ordering::SeqCst);
//     let a = application.bench_thread()?;
//     thread::sleep(Duration::from_secs(5));
//     application
//         .tray
//         .status
//         .store(false, std::sync::atomic::Ordering::SeqCst);
//     a.join().unwrap();
//     break;
// }
