use crate::core::audio_param::AudioParam;
use crate::core::channels::Stereo;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;

/// A stereo ping-pong delay effect.
///
/// The feedback from the left channel is sent to the right channel, and vice versa.
pub struct PingPongDelay {
    left_buffer: Vec<f32>,
    right_buffer: Vec<f32>,
    write_ptr: usize,
    delay_time: AudioParam,
    feedback: AudioParam,
    mix: AudioParam,
    max_delay_seconds: f32,
    sample_rate: usize,

    delay_buffer: Vec<f32>,
    feedback_buffer: Vec<f32>,
    mix_buffer: Vec<f32>,
}

impl PingPongDelay {
    /// Creates a new PingPongDelay.
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
        let sample_rate = 44100;
        let size = (max_delay_seconds * sample_rate as f32) as usize;

        PingPongDelay {
            left_buffer: vec![0.0; size],
            right_buffer: vec![0.0; size],
            write_ptr: 0,
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

impl FrameProcessor<Stereo> for PingPongDelay {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = self.left_buffer.len();
        if len == 0 {
            return;
        }

        let frames = buffer.len() / 2;

        if self.delay_buffer.len() < frames {
            self.delay_buffer.resize(frames, 0.0);
        }
        if self.feedback_buffer.len() < frames {
            self.feedback_buffer.resize(frames, 0.0);
        }
        if self.mix_buffer.len() < frames {
            self.mix_buffer.resize(frames, 0.0);
        }

        self.delay_time
            .process(&mut self.delay_buffer[0..frames], sample_index);
        self.feedback
            .process(&mut self.feedback_buffer[0..frames], sample_index);
        self.mix
            .process(&mut self.mix_buffer[0..frames], sample_index);

        let current_delay_s = self.delay_buffer[0];
        let delay_samples = libm::roundf(current_delay_s * self.sample_rate as f32) as usize;
        let delay_samples = if delay_samples >= len {
            if len > 0 {
                len - 1
            } else {
                0
            }
        } else {
            delay_samples
        };

        for (i, frame) in buffer.chunks_mut(2).enumerate() {
            if frame.len() < 2 {
                break;
            }

            let input_l = frame[0];
            let input_r = frame[1];

            let fb = self.feedback_buffer[i];
            let mix = self.mix_buffer[i];

            let read_ptr = (self.write_ptr + len - delay_samples) % len;

            let delayed_l = self.left_buffer[read_ptr];
            let delayed_r = self.right_buffer[read_ptr];

            let next_l = input_l + delayed_r * fb;
            let next_r = input_r + delayed_l * fb;

            self.left_buffer[self.write_ptr] = next_l;
            self.right_buffer[self.write_ptr] = next_r;

            frame[0] = input_l * (1.0 - mix) + delayed_l * mix;
            frame[1] = input_r * (1.0 - mix) + delayed_r * mix;

            self.write_ptr = (self.write_ptr + 1) % len;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate as usize;
        self.delay_time.set_sample_rate(sample_rate);
        self.feedback.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);
        let new_size = (self.max_delay_seconds * sample_rate) as usize;
        if new_size > self.left_buffer.len() {
            self.left_buffer.resize(new_size, 0.0);
            self.right_buffer.resize(new_size, 0.0);
        }
    }

    fn reset(&mut self) {
        self.left_buffer.fill(0.0);
        self.right_buffer.fill(0.0);
        self.write_ptr = 0;
        self.delay_time.reset();
        self.feedback.reset();
        self.mix.reset();
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "PingPongDelay"
    }
}
