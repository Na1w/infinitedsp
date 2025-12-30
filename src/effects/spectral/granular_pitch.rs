use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use alloc::vec::Vec;
use alloc::vec;

/// A granular pitch shifter.
///
/// Shifts pitch by resampling grains of audio.
pub struct GranularPitchShift {
    buffer: Vec<f32>,
    write_ptr: usize,
    phasor: f32,
    window_size: f32,
    semitones: AudioParam,
    pitch_factor: f32,
    window_ms: f32,
    sample_rate: f32,
    semitones_buffer: Vec<f32>,
}

impl GranularPitchShift {
    /// Creates a new GranularPitchShift.
    ///
    /// # Arguments
    /// * `window_ms` - Grain size in milliseconds.
    /// * `semitones` - Pitch shift amount in semitones.
    pub fn new(window_ms: f32, semitones: AudioParam) -> Self {
        let sample_rate = 44100.0;
        let window_size = window_ms * sample_rate / 1000.0;
        let buffer_size = (sample_rate * 0.5) as usize;

        GranularPitchShift {
            buffer: vec![0.0; buffer_size],
            write_ptr: 0,
            phasor: 0.0,
            window_size,
            semitones,
            pitch_factor: 1.0,
            window_ms,
            sample_rate,
            semitones_buffer: Vec::new(),
        }
    }

    /// Sets the pitch shift amount in semitones.
    pub fn set_semitones(&mut self, semitones: AudioParam) {
        self.semitones = semitones;
    }
}

impl FrameProcessor for GranularPitchShift {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = self.buffer.len() as f32;

        if self.semitones_buffer.len() < buffer.len() {
            self.semitones_buffer.resize(buffer.len(), 0.0);
        }
        self.semitones.process(&mut self.semitones_buffer, sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let semitones = self.semitones_buffer[i];
            self.pitch_factor = libm::powf(2.0, semitones / 12.0);
            let inc = 1.0 - self.pitch_factor;

            let input = *sample;
            self.buffer[self.write_ptr] = input;

            self.phasor += inc;
            if self.phasor >= self.window_size {
                self.phasor -= self.window_size;
            } else if self.phasor < 0.0 {
                self.phasor += self.window_size;
            }

            let delay1 = self.phasor;
            let r1 = (self.write_ptr as f32 - delay1 + len) % len;
            let val1 = self.buffer[r1 as usize];

            let mut delay2 = self.phasor + self.window_size * 0.5;
            if delay2 >= self.window_size { delay2 -= self.window_size; }
            let r2 = (self.write_ptr as f32 - delay2 + len) % len;
            let val2 = self.buffer[r2 as usize];

            let x1 = delay1 / self.window_size;
            let gain1 = if x1 < 0.5 { 2.0 * x1 } else { 2.0 * (1.0 - x1) };

            let x2 = delay2 / self.window_size;
            let gain2 = if x2 < 0.5 { 2.0 * x2 } else { 2.0 * (1.0 - x2) };

            *sample = val1 * gain1 + val2 * gain2;

            self.write_ptr = (self.write_ptr + 1) % self.buffer.len();
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.semitones.set_sample_rate(sample_rate);
        self.window_size = self.window_ms * sample_rate / 1000.0;

        let needed = (sample_rate * 0.5) as usize;
        if needed > self.buffer.len() {
            self.buffer.resize(needed, 0.0);
        }
    }
}
