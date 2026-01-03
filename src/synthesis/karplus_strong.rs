use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;

/// A Karplus-Strong string synthesis model.
///
/// Simulates a plucked string using a delay line and a low-pass filter.
pub struct KarplusStrong {
    pitch: AudioParam,
    gate: AudioParam,
    damping: AudioParam,
    pick_position: AudioParam, // 0.0 to 0.5 (0.5 = middle of string)

    delay_line: Vec<f32>,
    write_ptr: usize,
    sample_rate: f32,

    last_gate: f32,
    rng_state: u32,

    pitch_buffer: Vec<f32>,
    gate_buffer: Vec<f32>,
    damping_buffer: Vec<f32>,
    pick_pos_buffer: Vec<f32>,
}

impl KarplusStrong {
    /// Creates a new KarplusStrong synthesizer.
    ///
    /// # Arguments
    /// * `pitch` - Fundamental frequency in Hz.
    /// * `gate` - Gate signal to trigger a pluck (0.0 -> 1.0).
    /// * `damping` - Damping factor (0.0 - 1.0), higher values mean shorter decay.
    /// * `pick_position` - Pluck position (0.0 = bridge, 0.5 = middle).
    pub fn new(
        pitch: AudioParam,
        gate: AudioParam,
        damping: AudioParam,
        pick_position: AudioParam,
    ) -> Self {
        let sample_rate = 44100.0;
        let buffer_size = (sample_rate / 20.0) as usize;

        KarplusStrong {
            pitch,
            gate,
            damping,
            pick_position,
            delay_line: vec![0.0; buffer_size],
            write_ptr: 0,
            sample_rate,
            last_gate: 0.0,
            rng_state: 12345,
            pitch_buffer: Vec::new(),
            gate_buffer: Vec::new(),
            damping_buffer: Vec::new(),
            pick_pos_buffer: Vec::new(),
        }
    }

    fn next_random(rng_state: &mut u32) -> f32 {
        *rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let val = (*rng_state >> 16) & 0x7FFF;
        (val as f32 / 32768.0) * 2.0 - 1.0
    }
}

impl FrameProcessor<Mono> for KarplusStrong {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.pitch_buffer.len() < len {
            self.pitch_buffer.resize(len, 0.0);
        }
        if self.gate_buffer.len() < len {
            self.gate_buffer.resize(len, 0.0);
        }
        if self.damping_buffer.len() < len {
            self.damping_buffer.resize(len, 0.0);
        }
        if self.pick_pos_buffer.len() < len {
            self.pick_pos_buffer.resize(len, 0.0);
        }

        self.pitch
            .process(&mut self.pitch_buffer[0..len], sample_index);
        self.gate
            .process(&mut self.gate_buffer[0..len], sample_index);
        self.damping
            .process(&mut self.damping_buffer[0..len], sample_index);
        self.pick_position
            .process(&mut self.pick_pos_buffer[0..len], sample_index);

        let delay_len = self.delay_line.len();
        if delay_len == 0 {
            return;
        }

        for (i, sample) in buffer.iter_mut().enumerate() {
            let gate_val = self.gate_buffer[i];

            if gate_val >= 0.5 && self.last_gate < 0.5 {
                let pitch_val = self.pitch_buffer[i];
                let period = (self.sample_rate / pitch_val).max(1.0) as usize;
                let pick_pos = self.pick_pos_buffer[i].clamp(0.01, 0.5);
                let pick_offset = (period as f32 * pick_pos) as usize;

                if period < delay_len {
                    for j in 0..period {
                        let idx = (self.write_ptr + j) % delay_len;
                        let noise = Self::next_random(&mut self.rng_state);
                        self.delay_line[idx] = noise;
                    }

                    for j in 0..(period - pick_offset) {
                        let idx = (self.write_ptr + j) % delay_len;
                        let delayed_idx = (self.write_ptr + j + pick_offset) % delay_len;
                        self.delay_line[delayed_idx] -= self.delay_line[idx];
                    }
                }
            }
            self.last_gate = gate_val;

            let pitch_val = self.pitch_buffer[i];
            let period = (self.sample_rate / pitch_val).max(1.0);

            let read_pos = (self.write_ptr as f32 - period + delay_len as f32) % delay_len as f32;
            let idx_a = read_pos as usize;
            let idx_b = (idx_a + 1) % delay_len;
            let frac = read_pos - idx_a as f32;

            let delayed_sample =
                self.delay_line[idx_a] * (1.0 - frac) + self.delay_line[idx_b] * frac;
            let damping_val = self.damping_buffer[i];

            let avg = (delayed_sample + self.delay_line[self.write_ptr]) * 0.5;
            let feedback = (delayed_sample * (1.0 - damping_val) + avg * damping_val) * 0.996;

            self.delay_line[self.write_ptr] = feedback;
            *sample = feedback;

            self.write_ptr = (self.write_ptr + 1) % delay_len;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.pitch.set_sample_rate(sample_rate);
        self.gate.set_sample_rate(sample_rate);
        self.damping.set_sample_rate(sample_rate);
        self.pick_position.set_sample_rate(sample_rate);

        let buffer_size = (sample_rate / 20.0) as usize;
        if buffer_size > self.delay_line.len() {
            self.delay_line.resize(buffer_size, 0.0);
        }
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "KarplusStrong"
    }
}
