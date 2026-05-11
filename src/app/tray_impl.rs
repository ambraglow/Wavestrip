use crate::app::*;

impl AppTray {
    pub fn update_stream(&self) -> anyhow::Result<()> {
        let filename = self.playback.filename.lock().unwrap().to_string();
        log::info!("loading file: {:?}", filename);

        self.playback.samples_buffer.lock().unwrap().clear();
        self.playback.bins_buffer.lock().unwrap().clear();
        self.playback.window_buffer.lock().unwrap().clear();

        let file = std::fs::File::open(&filename).context("invalid filename")?;
        let decoder = Decoder::try_from(file)?;

        let sample_rate = decoder.sample_rate();
        *self.playback.sample_rate.lock().unwrap() = sample_rate;

        let collected_samples = decoder
            .resample_into_fixed_source(sample_rate, nz!(1))
            .collect::<Vec<f32>>();

        *self.playback.audio_length.lock().unwrap() =
            Duration::from_secs_f64(collected_samples.len() as f64 / sample_rate.get() as f64);

        self.playback
            .samples_buffer
            .lock()
            .unwrap()
            .append(&mut collected_samples.clone());

        self.playback
            .player
            .append(SamplesBuffer::new(nz!(1), sample_rate, collected_samples));

        self.playback.player.pause();

        Ok(())
    }

    pub fn new() -> Self {
        let filename = Mutex::new("./assets/last-moan.m4a".to_string());

        let mut _mixer = rodio_experiments::DeviceSinkBuilder::open_default_sink().unwrap();
        _mixer.log_on_drop(false);

        let mixer = _mixer;
        let player = rodio_experiments::Player::connect_new(mixer.mixer());

        let _file = std::fs::File::open(filename.lock().unwrap().to_string()).unwrap();

        let decoder = Mutex::new(Decoder::try_from(_file).unwrap());
        let samples_buffer = Mutex::new(Vec::new());
        let length = Mutex::new(decoder.lock().unwrap().total_duration().unwrap());
        let sample_rate = Mutex::new(decoder.lock().unwrap().sample_rate());

        let playback = Arc::new(Playback {
            player,
            mixer,
            decoder,
            samples_buffer,
            window_buffer: Mutex::new(Vec::new()),
            bins_buffer: Mutex::new(Vec::new()),
            filename,
            sample_rate,
            audio_length: length,
        });

        AppTray {
            playback,
            status: AtomicBool::new(true),
            fft_step: AtomicUsize::new(2048),
            fft_window: AtomicUsize::new(1),
            show_demo_window: AtomicBool::new(false),
            start_playback: AtomicBool::new(false),
            stop_playback: AtomicBool::new(false),
        }
    }
}

impl<T: Clone, E: Clone> App<T, E> {
    pub fn destroy_external_texture(&mut self) {
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
