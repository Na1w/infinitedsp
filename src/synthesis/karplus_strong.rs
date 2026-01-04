use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;

/// A Karplus-Strong string synthesis model.
///
/// Simulates a plucked string using a delay line and a low-pass filter.
pub struct KarplusStrong {
    delay_line: Vec<f32>,
    write_ptr: usize,
    pitch: AudioParam,
    gate: AudioParam,
    damping: AudioParam,
    pick_position: AudioParam,
    sample_rate: f32,

    last_gate: f32,
    filter_state: f32,
    noise_burst_samples: usize,
    current_burst_sample: usize,

    pitch_buffer: Vec<f32>,
    gate_buffer: Vec<f32>,
    damping_buffer: Vec<f32>,
    pick_buffer: Vec<f32>,

    rng_state: u32,
}

impl KarplusStrong {
    /// Creates a new KarplusStrong model.
    ///
    /// # Arguments
    /// * `pitch` - Frequency of the string in Hz.
    /// * `gate` - Trigger signal (0.0 -> 1.0 plucks the string).
    /// * `damping` - High-frequency damping (0.0 - 1.0).
    /// * `pick_position` - Position of the pluck (0.0 - 1.0).
    pub fn new(
        pitch: AudioParam,
        gate: AudioParam,
        damping: AudioParam,
        pick_position: AudioParam,
    ) -> Self {
        let sample_rate = 44100.0;
        let max_delay = (sample_rate / 20.0) as usize;

        KarplusStrong {
            delay_line: vec![0.0; max_delay],
            write_ptr: 0,
            pitch,
            gate,
            damping,
            pick_position,
            sample_rate,
            last_gate: 0.0,
            filter_state: 0.0,
            noise_burst_samples: 0,
            current_burst_sample: 0,
            pitch_buffer: Vec::new(),
            gate_buffer: Vec::new(),
            damping_buffer: Vec::new(),
            pick_buffer: Vec::new(),
            rng_state: 12345,
        }
    }

    fn next_random(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let val = (self.rng_state >> 16) & 0x7FFF;
        (val as f32 / 32768.0) * 2.0 - 1.0
    }
}

impl FrameProcessor<Mono> for KarplusStrong {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        let dl_len = self.delay_line.len();

        if self.pitch_buffer.len() < len {
            self.pitch_buffer.resize(len, 0.0);
        }
        if self.gate_buffer.len() < len {
            self.gate_buffer.resize(len, 0.0);
        }
        if self.damping_buffer.len() < len {
            self.damping_buffer.resize(len, 0.0);
        }
        if self.pick_buffer.len() < len {
            self.pick_buffer.resize(len, 0.0);
        }

        self.pitch
            .process(&mut self.pitch_buffer[0..len], sample_index);
        self.gate
            .process(&mut self.gate_buffer[0..len], sample_index);
        self.damping
            .process(&mut self.damping_buffer[0..len], sample_index);
        self.pick_position
            .process(&mut self.pick_buffer[0..len], sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let pitch = self.pitch_buffer[i];
            let gate = self.gate_buffer[i];
            let damping = self.damping_buffer[i];
            let _pick = self.pick_buffer[i];
            if gate >= 0.5 && self.last_gate < 0.5 {
                let period = self.sample_rate / pitch.max(1.0);
                self.noise_burst_samples = period as usize;
                self.current_burst_sample = 0;
            }
            self.last_gate = gate;

            let mut input = 0.0;
            if self.current_burst_sample < self.noise_burst_samples {
                input = self.next_random();
                self.current_burst_sample += 1;
            }

            let period = self.sample_rate / pitch.max(1.0);
            let delay_samples = period;

            let read_ptr_f =
                (self.write_ptr as f32 - delay_samples + dl_len as f32) % dl_len as f32;
            let idx_a = read_ptr_f as usize;
            let idx_b = (idx_a + 1) % dl_len;
            let frac = read_ptr_f - idx_a as f32;

            let delayed = self.delay_line[idx_a] * (1.0 - frac) + self.delay_line[idx_b] * frac;

            let filtered = damping * self.filter_state + (1.0 - damping) * delayed;
            self.filter_state = filtered;

            let feedback = filtered * 0.995;

            let output = input + feedback;
            self.delay_line[self.write_ptr] = output;
            self.write_ptr = (self.write_ptr + 1) % dl_len;

            *sample = output;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.pitch.set_sample_rate(sample_rate);
        self.gate.set_sample_rate(sample_rate);
        self.damping.set_sample_rate(sample_rate);
        self.pick_position.set_sample_rate(sample_rate);

        let max_delay = (sample_rate / 20.0) as usize;
        if max_delay > self.delay_line.len() {
            self.delay_line.resize(max_delay, 0.0);
        }
    }

    fn reset(&mut self) {
        self.delay_line.fill(0.0);
        self.write_ptr = 0;
        self.filter_state = 0.0;
        self.current_burst_sample = self.noise_burst_samples;
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "KarplusStrong"
    }
}
