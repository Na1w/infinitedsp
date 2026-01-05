use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;

const PARAM_CHUNK_SIZE: usize = 64;

/// A digital delay effect with linear interpolation.
///
/// Provides a clean delay line with feedback and dry/wet mix control.
/// Supports sample-accurate modulation of delay time.
pub struct Delay {
    buffer: Vec<f32>,
    write_ptr: usize,
    delay_time: AudioParam,
    feedback: AudioParam,
    mix: AudioParam,
    max_delay_seconds: f32,
    sample_rate: f32,
    delay_buffer: [f32; PARAM_CHUNK_SIZE],
    feedback_buffer: [f32; PARAM_CHUNK_SIZE],
    mix_buffer: [f32; PARAM_CHUNK_SIZE],
}

impl Delay {
    /// Creates a new Delay.
    ///
    /// # Arguments
    /// * `max_delay_seconds`: Maximum buffer size in seconds.
    /// * `delay_time`: Delay time in seconds.
    /// * `feedback`: Feedback amount (0.0 - 1.0).
    /// * `mix`: Dry/Wet mix (0.0 - 1.0).
    pub fn new(
        max_delay_seconds: f32,
        delay_time: AudioParam,
        feedback: AudioParam,
        mix: AudioParam,
    ) -> Self {
        let sample_rate = 44100.0;
        let size = (max_delay_seconds * sample_rate) as usize;

        Delay {
            buffer: vec![0.0; size],
            write_ptr: 0,
            delay_time,
            feedback,
            mix,
            max_delay_seconds,
            sample_rate,
            delay_buffer: [0.0; PARAM_CHUNK_SIZE],
            feedback_buffer: [0.0; PARAM_CHUNK_SIZE],
            mix_buffer: [0.0; PARAM_CHUNK_SIZE],
        }
    }

    /// Sets the delay time parameter.
    pub fn set_delay_time(&mut self, delay_time: AudioParam) {
        self.delay_time = delay_time;
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

impl FrameProcessor<Mono> for Delay {
    fn process(&mut self, buffer: &mut [f32], start_sample_index: u64) {
        let len = self.buffer.len();
        if len == 0 {
            return;
        }
        let len_f = len as f32;

        let mut current_sample_index = start_sample_index;

        for chunk in buffer.chunks_mut(PARAM_CHUNK_SIZE) {
            let chunk_len = chunk.len();

            self.delay_time
                .process(&mut self.delay_buffer[0..chunk_len], current_sample_index);
            self.feedback.process(
                &mut self.feedback_buffer[0..chunk_len],
                current_sample_index,
            );
            self.mix
                .process(&mut self.mix_buffer[0..chunk_len], current_sample_index);

            for (i, sample) in chunk.iter_mut().enumerate() {
                let input = *sample;

                let delay_seconds = self.delay_buffer[i];
                let fb = self.feedback_buffer[i];
                let mix = self.mix_buffer[i];

                let delay_samples = delay_seconds * self.sample_rate;
                let read_ptr_f = self.write_ptr as f32 - delay_samples;

                let mut read_ptr_norm = read_ptr_f;
                while read_ptr_norm < 0.0 {
                    read_ptr_norm += len_f;
                }
                while read_ptr_norm >= len_f {
                    read_ptr_norm -= len_f;
                }

                let idx_a = read_ptr_norm as usize;
                let idx_b = (idx_a + 1) % len;
                let frac = read_ptr_norm - idx_a as f32;

                let delayed = self.buffer[idx_a] * (1.0 - frac) + self.buffer[idx_b] * frac;
                let next_val = input + delayed * fb;
                self.buffer[self.write_ptr] = next_val;

                *sample = input * (1.0 - mix) + delayed * mix;
                self.write_ptr = (self.write_ptr + 1) % len;
            }

            current_sample_index += chunk_len as u64;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.delay_time.set_sample_rate(sample_rate);
        self.feedback.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);

        let new_size = (self.max_delay_seconds * sample_rate) as usize;
        if new_size > self.buffer.len() {
            self.buffer.resize(new_size, 0.0);
        }
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_ptr = 0;
        self.delay_time.reset();
        self.feedback.reset();
        self.mix.reset();
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Delay (Digital)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_interpolation() {
        let mut delay = Delay::new(
            1.0,
            AudioParam::Static(0.5 / 100.0),
            AudioParam::Static(0.0),
            AudioParam::Static(1.0),
        );
        delay.set_sample_rate(100.0);

        let mut buffer = [1.0, 0.0, 0.0, 0.0];
        delay.process(&mut buffer, 0);

        assert_eq!(buffer[0], 0.0);
        assert!((buffer[1] - 0.5).abs() < 1e-5);
    }
}
