use crate::core::audio_param::AudioParam;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;

/// A physical model of a brass instrument.
///
/// Uses a waveguide synthesis approach with a non-linear lip valve model.
pub struct BrassModel {
    pitch: AudioParam,
    breath_pressure: AudioParam,
    lip_tension: AudioParam,

    delay_line: Vec<f32>,
    write_ptr: usize,
    sample_rate: f32,

    dc_block: f32,

    pitch_buffer: Vec<f32>,
    breath_buffer: Vec<f32>,
    tension_buffer: Vec<f32>,

    rng_state: u32,
}

impl BrassModel {
    /// Creates a new BrassModel.
    ///
    /// # Arguments
    /// * `pitch` - Fundamental frequency in Hz.
    /// * `breath` - Breath pressure (0.0 - 1.0).
    /// * `tension` - Lip tension (0.0 - 1.0).
    pub fn new(pitch: AudioParam, breath: AudioParam, tension: AudioParam) -> Self {
        let sample_rate = 44100.0;
        let buffer_size = (sample_rate / 20.0) as usize;

        BrassModel {
            pitch,
            breath_pressure: breath,
            lip_tension: tension,
            delay_line: vec![0.0; buffer_size],
            write_ptr: 0,
            sample_rate,
            dc_block: 0.0,
            pitch_buffer: Vec::new(),
            breath_buffer: Vec::new(),
            tension_buffer: Vec::new(),
            rng_state: 12345,
        }
    }

    fn next_random(rng_state: &mut u32) -> f32 {
        *rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let val = (*rng_state >> 16) & 0x7FFF;
        (val as f32 / 32768.0) * 2.0 - 1.0
    }
}

impl FrameProcessor for BrassModel {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.pitch_buffer.len() < len {
            self.pitch_buffer.resize(len, 0.0);
        }
        if self.breath_buffer.len() < len {
            self.breath_buffer.resize(len, 0.0);
        }
        if self.tension_buffer.len() < len {
            self.tension_buffer.resize(len, 0.0);
        }

        self.pitch
            .process(&mut self.pitch_buffer[0..len], sample_index);
        self.breath_pressure
            .process(&mut self.breath_buffer[0..len], sample_index);
        self.lip_tension
            .process(&mut self.tension_buffer[0..len], sample_index);

        let delay_len = self.delay_line.len();
        if delay_len == 0 {
            return;
        }

        for (i, sample) in buffer.iter_mut().enumerate() {
            let pitch_val = self.pitch_buffer[i];
            let breath = self.breath_buffer[i];
            let tension = self.tension_buffer[i];

            let noise = Self::next_random(&mut self.rng_state) * 0.2 * breath;

            let period = (self.sample_rate / pitch_val).max(2.0);

            let read_pos = (self.write_ptr as f32 - period + delay_len as f32) % delay_len as f32;
            let idx_a = read_pos as usize;
            let idx_b = (idx_a + 1) % delay_len;
            let frac = read_pos - idx_a as f32;

            let bore_out = self.delay_line[idx_a] * (1.0 - frac) + self.delay_line[idx_b] * frac;

            let feedback = -0.9 * bore_out;

            let input_signal = breath + noise + feedback;
            // libm::tanhf
            let excitation = libm::tanhf(input_signal * (1.0 + tension));

            self.dc_block = excitation * 0.5 + self.dc_block * 0.5;

            self.delay_line[self.write_ptr] = self.dc_block;

            *sample = bore_out;

            self.write_ptr = (self.write_ptr + 1) % delay_len;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.pitch.set_sample_rate(sample_rate);
        self.breath_pressure.set_sample_rate(sample_rate);
        self.lip_tension.set_sample_rate(sample_rate);

        let buffer_size = (sample_rate / 20.0) as usize;
        if buffer_size > self.delay_line.len() {
            self.delay_line.resize(buffer_size, 0.0);
        }
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "BrassModel"
    }
}
