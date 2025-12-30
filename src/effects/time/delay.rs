use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use wide::f32x4;
use alloc::vec::Vec;
use alloc::vec;

pub struct Delay {
    buffer: Vec<f32>,
    write_ptr: usize,
    delay_samples: usize,
    delay_time: AudioParam,
    feedback: AudioParam,
    mix: AudioParam,
    max_delay_seconds: f32,
    sample_rate: usize,

    delay_buffer: Vec<f32>,
    feedback_buffer: Vec<f32>,
    mix_buffer: Vec<f32>,
}

impl Delay {
    /// Creates a new Delay.
    ///
    /// # Arguments
    /// * `max_delay_seconds`: Maximum buffer size in seconds.
    /// * `delay_time`: Delay time in seconds.
    /// * `feedback`: Feedback amount (0.0 - 1.0).
    /// * `mix`: Dry/Wet mix (0.0 - 1.0).
    pub fn new(max_delay_seconds: f32, delay_time: AudioParam, feedback: AudioParam, mix: AudioParam) -> Self {
        let sample_rate = 44100;
        let size = (max_delay_seconds * sample_rate as f32) as usize;
        let default_delay = (sample_rate / 2).min(if size > 0 { size - 1 } else { 0 });

        Delay {
            buffer: vec![0.0; size],
            write_ptr: 0,
            delay_samples: default_delay,
            delay_time,
            feedback,
            mix,
            max_delay_seconds,
            sample_rate,
            delay_buffer: Vec::new(),
            feedback_buffer: Vec::new(),
            mix_buffer: Vec::new(),
        }
    }

    pub fn set_delay_time(&mut self, delay_time: AudioParam) {
        self.delay_time = delay_time;
    }

    pub fn set_feedback(&mut self, feedback: AudioParam) {
        self.feedback = feedback;
    }

    pub fn set_mix(&mut self, mix: AudioParam) {
        self.mix = mix;
    }
}

impl FrameProcessor for Delay {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = self.buffer.len();
        if len == 0 { return; }

        let block_size = buffer.len();

        if self.delay_buffer.len() < block_size { self.delay_buffer.resize(block_size, 0.0); }
        if self.feedback_buffer.len() < block_size { self.feedback_buffer.resize(block_size, 0.0); }
        if self.mix_buffer.len() < block_size { self.mix_buffer.resize(block_size, 0.0); }

        self.delay_time.process(&mut self.delay_buffer[0..block_size], sample_index);
        self.feedback.process(&mut self.feedback_buffer[0..block_size], sample_index);
        self.mix.process(&mut self.mix_buffer[0..block_size], sample_index);

        // For Digital Delay, we use the first sample of delay_time for the whole block to keep SIMD optimization.
        // If sample-accurate modulation is needed, TapeDelay should be used.
        let current_delay_s = self.delay_buffer[0];
        self.delay_samples = (current_delay_s * self.sample_rate as f32).round() as usize;
        if self.delay_samples >= len {
            self.delay_samples = if len > 0 { len - 1 } else { 0 };
        }

        let read_ptr_start = (self.write_ptr + len - self.delay_samples) % len;

        let write_end = self.write_ptr + block_size;
        let read_end = read_ptr_start + block_size;

        if write_end <= len && read_end <= len {
            let (chunks, remainder) = buffer.as_chunks_mut::<4>();
            let (fb_chunks, fb_rem) = self.feedback_buffer[0..block_size].as_chunks::<4>();
            let (mix_chunks, mix_rem) = self.mix_buffer[0..block_size].as_chunks::<4>();

            let mut w_ptr = self.write_ptr;
            let mut r_ptr = read_ptr_start;

            for ((chunk, fb_chunk), mix_chunk) in chunks.iter_mut().zip(fb_chunks).zip(mix_chunks) {
                let input = f32x4::from(*chunk);
                let feedback_vec = f32x4::from(*fb_chunk);
                let mix_vec = f32x4::from(*mix_chunk);
                let dry_mix_vec = f32x4::splat(1.0) - mix_vec;

                let delayed_slice = &self.buffer[r_ptr..r_ptr+4];
                let delayed = f32x4::from(unsafe { *(delayed_slice.as_ptr() as *const [f32; 4]) });

                let next_val = input + delayed * feedback_vec;
                let next_val_arr = next_val.to_array();
                self.buffer[w_ptr..w_ptr+4].copy_from_slice(&next_val_arr);

                let output = input * dry_mix_vec + delayed * mix_vec;
                *chunk = output.to_array();

                w_ptr += 4;
                r_ptr += 4;
            }

            for ((sample, &fb), &mix) in remainder.iter_mut().zip(fb_rem).zip(mix_rem) {
                let input = *sample;
                let delayed = self.buffer[r_ptr];

                let next_val = input + delayed * fb;
                self.buffer[w_ptr] = next_val;

                *sample = input * (1.0 - mix) + delayed * mix;

                w_ptr += 1;
                r_ptr += 1;
            }

            self.write_ptr = (self.write_ptr + block_size) % len;

        } else {
            for (i, sample) in buffer.iter_mut().enumerate() {
                let input = *sample;
                let fb = self.feedback_buffer[i];
                let mix = self.mix_buffer[i];

                let read_ptr = (self.write_ptr + len - self.delay_samples) % len;
                let delayed = self.buffer[read_ptr];

                let next_val = input + delayed * fb;
                self.buffer[self.write_ptr] = next_val;

                *sample = input * (1.0 - mix) + delayed * mix;

                self.write_ptr = (self.write_ptr + 1) % len;
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate as usize;
        self.delay_time.set_sample_rate(sample_rate);
        self.feedback.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);
        let new_size = (self.max_delay_seconds * sample_rate) as usize;
        if new_size > self.buffer.len() {
            self.buffer.resize(new_size, 0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay() {
        let mut delay = Delay::new(1.0, AudioParam::seconds(0.01), AudioParam::linear(0.5), AudioParam::linear(0.5));
        delay.set_sample_rate(100.0);

        let mut buffer = [1.0, 0.0, 0.0];
        delay.process(&mut buffer, 0);

        assert_eq!(buffer[0], 0.5);
        assert_eq!(buffer[1], 0.5);
        assert_eq!(buffer[2], 0.25);
    }
}
