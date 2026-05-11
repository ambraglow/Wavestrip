use crate::dsp::*;

pub struct Sine {
    freq: f32,
    sample_rate: u32,
    samples_nr: u32,
}

impl Sine {
    pub fn new(freq: f32, sample_rate: u32) -> Self {
        Sine {
            freq,
            sample_rate,
            ..Default::default()
        }
    }
}

impl Default for Sine {
    fn default() -> Self {
        Sine {
            freq: 2000.0,
            samples_nr: 0,
            sample_rate: 44000,
        }
    }
}

impl Iterator for Sine {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.samples_nr = self.samples_nr.wrapping_add(1);
        let value = TAU * self.freq * (self.samples_nr as f32 / self.sample_rate as f32);
        Some(value.sin())
    }
}
