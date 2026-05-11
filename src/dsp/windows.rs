use crate::dsp::*;

#[allow(dead_code)]
pub trait Window {
    fn blackman(&mut self);
    fn ham(&mut self);
    fn hanning(&mut self);
    fn rectangular(&mut self, window_size: usize);
}

#[derive(Debug)]
pub enum Types {
    NoWindow,
    Blackman,
    Ham,
    Hanning,
    Rectangular,
}

impl Types {
    pub const VALUES: [&str; 5] = ["no window", "Blackman", "Ham", "hanning", "rectangular"];
}

impl Window for Vec<f32> {
    fn blackman(&mut self) {
        let size = self.len();
        let mut blackman = vec![0f32; size];
        blackman.iter_mut().enumerate().for_each(|(n, a)| {
            let meow = 0.42 - 0.5 * ((TAU * n as f32) / (size - 1) as f32).cos()
                + 0.08 * ((4f32 * PI * n as f32) / (size - 1) as f32).cos();
            *a = meow;
        });

        self.iter_mut().zip(blackman).for_each(|(a, b)| *a = *a * b);
    }

    fn ham(&mut self) {
        todo!()
    }

    fn hanning(&mut self) {
        let size = self.len();
        let mut hanning = vec![0f32; size];
        hanning.iter_mut().enumerate().for_each(|(n, b)| {
            let meow = 0.5 * (1.0 - ((TAU * n as f32) / (size - 1) as f32).cos());
            *b = meow;
        });

        self.iter_mut().zip(hanning).for_each(|(a, b)| *a = *a * b);
    }

    fn rectangular(&mut self, window_size: usize) {
        let size = self.len();
        let window_start = (size / 2) - window_size / 2;
        let window_end = (size / 2) + window_size / 2;

        for n in &mut self[0..window_start] {
            *n = 0.0;
        }
        for n in &mut self[window_end..size] {
            *n = 0.0;
        }
    }
}
