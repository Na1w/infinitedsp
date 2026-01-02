use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use core::f32::consts::PI;
use alloc::vec::Vec;
use alloc::vec;

/// A modulated delay effect, used for Chorus and Flanger.
pub struct ModulatedDelay {
    buffer: Vec<f32>,
    write_ptr: usize,

    lfo_phase: f32,
    lfo_inc: f32,
    depth: AudioParam,
    base_delay: f32,

    feedback: AudioParam,
    mix: AudioParam,
    sample_rate: f32,

    depth_buffer: Vec<f32>,
    feedback_buffer: Vec<f32>,
    mix_buffer: Vec<f32>,
}

impl ModulatedDelay {
    /// Creates a new Chorus effect.
    ///
    /// Uses a longer delay time and moderate modulation depth.
    pub fn new_chorus() -> Self {
        let sample_rate = 44100.0;
        let buffer_size = (sample_rate * 0.1) as usize;

        ModulatedDelay {
            buffer: vec![0.0; buffer_size],
            write_ptr: 0,
            lfo_phase: 0.0,
            lfo_inc: 2.0 * PI * 1.5 / sample_rate,
            depth: AudioParam::Static(0.002 * sample_rate),
            base_delay: 0.015 * sample_rate,
            feedback: AudioParam::Static(0.4),
            mix: AudioParam::Static(0.5),
            sample_rate,
            depth_buffer: Vec::new(),
            feedback_buffer: Vec::new(),
            mix_buffer: Vec::new(),
        }
    }

    /// Creates a new Flanger effect.
    ///
    /// Uses a short delay time and higher feedback.
    pub fn new_flanger() -> Self {
        let sample_rate = 44100.0;
        let buffer_size = (sample_rate * 0.1) as usize;

        ModulatedDelay {
            buffer: vec![0.0; buffer_size],
            write_ptr: 0,
            lfo_phase: 0.0,
            lfo_inc: 2.0 * PI * 0.5 / sample_rate,
            depth: AudioParam::Static(0.005 * sample_rate),
            base_delay: 0.005 * sample_rate,
            feedback: AudioParam::Static(0.7),
            mix: AudioParam::Static(0.5),
            sample_rate,
            depth_buffer: Vec::new(),
            feedback_buffer: Vec::new(),
            mix_buffer: Vec::new(),
        }
    }

    /// Sets the modulation depth parameter.
    pub fn set_depth(&mut self, depth: AudioParam) {
        self.depth = depth;
    }

    /// Sets the feedback parameter.
    pub fn set_feedback(&mut self, feedback: AudioParam) {
        self.feedback = feedback;
    }

    /// Sets the mix parameter.
    pub fn set_mix(&mut self, mix: AudioParam) {
        self.mix = mix;
    }
}

impl FrameProcessor for ModulatedDelay {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = self.buffer.len();
        let len_f = len as f32;
        let block_size = buffer.len();

        if self.depth_buffer.len() < block_size { self.depth_buffer.resize(block_size, 0.0); }
        if self.feedback_buffer.len() < block_size { self.feedback_buffer.resize(block_size, 0.0); }
        if self.mix_buffer.len() < block_size { self.mix_buffer.resize(block_size, 0.0); }

        self.depth.process(&mut self.depth_buffer[0..block_size], sample_index);
        self.feedback.process(&mut self.feedback_buffer[0..block_size], sample_index);
        self.mix.process(&mut self.mix_buffer[0..block_size], sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let input = *sample;
            let depth = self.depth_buffer[i];
            let feedback = self.feedback_buffer[i];
            let mix = self.mix_buffer[i];

            self.lfo_phase += self.lfo_inc;
            if self.lfo_phase > 2.0 * PI { self.lfo_phase -= 2.0 * PI; }

            let lfo = libm::sinf(self.lfo_phase);
            let current_delay = self.base_delay + lfo * depth;

            let read_pos = (self.write_ptr as f32 - current_delay + len_f) % len_f;
            let idx_a = read_pos as usize;
            let idx_b = (idx_a + 1) % len;
            let frac = read_pos - idx_a as f32;

            let delayed = self.buffer[idx_a] * (1.0 - frac) + self.buffer[idx_b] * frac;

            self.buffer[self.write_ptr] = input + delayed * feedback;

            *sample = input * (1.0 - mix) + delayed * mix;

            self.write_ptr = (self.write_ptr + 1) % len;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        let old_sr = self.sample_rate;
        self.sample_rate = sample_rate;
        self.depth.set_sample_rate(sample_rate);
        self.feedback.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);

        self.lfo_inc = self.lfo_inc * old_sr / sample_rate;

        self.base_delay = self.base_delay * sample_rate / old_sr;

        let needed = (sample_rate * 0.1) as usize;
        if needed > self.buffer.len() {
            self.buffer.resize(needed, 0.0);
        }
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "ModulatedDelay (Chorus/Flanger)"
    }
}
