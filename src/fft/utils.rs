use crate::fft::*;

pub trait PowerOfTwo {
    fn is_power_of_two(&mut self) -> bool;
    fn round_to_2n(&mut self) -> Self;
}

impl PowerOfTwo for i32 {
    fn is_power_of_two(&mut self) -> bool {
        let a = self.clone();
        if a & (a - 1) == 0 {
            true
        } else {
            false
        }
    }
    fn round_to_2n(&mut self) -> Self {
        let mut a: i32 = self.clone();
        a -= 1;
        a |= a >> 1;
        a |= a >> 2;
        a |= a >> 4;
        a |= a >> 8;
        a |= a >> 16;
        a += 1;
        *self = a;
        return *self;
    }
}

#[allow(dead_code)]
pub fn even_odd_split<T: Copy + Clone>(input: &mut Vec<T>) -> anyhow::Result<(Vec<T>, Vec<T>)> {
    let (even_indices, odd_indices): (Vec<_>, Vec<_>) =
        input.iter().enumerate().partition(|(i, _)| i % 2 == 0);

    let even = even_indices
        .iter()
        .map(|(_, val)| *val)
        .copied()
        .collect::<Vec<_>>();

    let odd = odd_indices
        .iter()
        .map(|(_, val)| *val)
        .copied()
        .collect::<Vec<_>>();

    Ok((even, odd))
}

pub fn ifft_test(x: &mut Vec<Complex32>) -> Vec<Complex32> {
    let mut _buffer = vec![Complex::new(0f32, 0f32); x.len()];
    _buffer = x.clone();
    let ifft = Radix4::new(x.len(), rustfft::FftDirection::Inverse);
    ifft.process(&mut _buffer);

    _buffer
}
