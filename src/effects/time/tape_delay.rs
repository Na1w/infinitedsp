use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;
use core::f32::consts::PI;

/// A tape delay simulation with saturation, wow/flutter, and low-pass filtering.
pub struct TapeDelay {
    buffer: Vec<f32>,
    write_ptr: usize,
    delay_time: AudioParam,
    feedback: AudioParam,
    mix: AudioParam,
    drive: AudioParam,
    max_delay_seconds: f32,
    sample_rate: f32,

    lfo_phase: f32,
    lfo_inc: f32,
    filter_state: f32,

    delay_buffer: Vec<f32>,
    feedback_buffer: Vec<f32>,
    mix_buffer: Vec<f32>,
    drive_buffer: Vec<f32>,
}

impl TapeDelay {
    /// Creates a new TapeDelay.
    ///
    /// # Arguments
    /// * `max_delay_s`: Maximum delay time in seconds.
    /// * `delay_time`: Delay time in seconds.
    /// * `feedback`: Feedback amount (0.0 - 1.0+).
    /// * `mix`: Dry/Wet mix (0.0 - 1.0).
    pub fn new(
        max_delay_s: f32,
        delay_time: AudioParam,
        feedback: AudioParam,
        mix: AudioParam,
    ) -> Self {
        let sample_rate = 44100.0;
        let size = (max_delay_s * sample_rate) as usize;

        TapeDelay {
            buffer: vec![0.0; size],
            write_ptr: 0,
            delay_time,
            feedback,
            mix,
            drive: AudioParam::Static(0.0),
            max_delay_seconds: max_delay_s,
            sample_rate,
            lfo_phase: 0.0,
            lfo_inc: 2.0 * PI * 0.5 / sample_rate,
            filter_state: 0.0,
            delay_buffer: Vec::new(),
            feedback_buffer: Vec::new(),
            mix_buffer: Vec::new(),
            drive_buffer: Vec::new(),
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

    /// Sets the drive (saturation) parameter.
    pub fn set_drive(&mut self, drive: AudioParam) {
        self.drive = drive;
    }
}

impl FrameProcessor<Mono> for TapeDelay {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = self.buffer.len();
        let len_f = len as f32;
        let block_size = buffer.len();

        if self.delay_buffer.len() < block_size {
            self.delay_buffer.resize(block_size, 0.0);
        }
        if self.feedback_buffer.len() < block_size {
            self.feedback_buffer.resize(block_size, 0.0);
        }
        if self.mix_buffer.len() < block_size {
            self.mix_buffer.resize(block_size, 0.0);
        }
        if self.drive_buffer.len() < block_size {
            self.drive_buffer.resize(block_size, 0.0);
        }

        self.delay_time
            .process(&mut self.delay_buffer[0..block_size], sample_index);
        self.feedback
            .process(&mut self.feedback_buffer[0..block_size], sample_index);
        self.mix
            .process(&mut self.mix_buffer[0..block_size], sample_index);
        self.drive
            .process(&mut self.drive_buffer[0..block_size], sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let input = *sample;
            let delay_s = self.delay_buffer[i];
            let fb = self.feedback_buffer[i];
            let mix = self.mix_buffer[i];
            let drive = self.drive_buffer[i];

            self.lfo_phase += self.lfo_inc;
            if self.lfo_phase > 2.0 * PI {
                self.lfo_phase -= 2.0 * PI;
            }
            let flutter = libm::sinf(self.lfo_phase) * 0.0005; 

            let current_delay_s = delay_s + flutter;
            let delay_samples = current_delay_s * self.sample_rate;

            let read_pos = (self.write_ptr as f32 - delay_samples + len_f) % len_f;
            let idx_a = read_pos as usize;
            let idx_b = (idx_a + 1) % len;
            let frac = read_pos - idx_a as f32;

            let mut delayed = self.buffer[idx_a] * (1.0 - frac) + self.buffer[idx_b] * frac;

            if drive > 0.0 {
                delayed = libm::tanhf(delayed * (1.0 + drive));
            }

            self.filter_state += (delayed - self.filter_state) * 0.3;
            delayed = self.filter_state;

            self.buffer[self.write_ptr] = input + delayed * fb;

            *sample = input * (1.0 - mix) + delayed * mix;

            self.write_ptr = (self.write_ptr + 1) % len;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        let old_sr = self.sample_rate;
        self.sample_rate = sample_rate;
        self.delay_time.set_sample_rate(sample_rate);
        self.feedback.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);
        self.drive.set_sample_rate(sample_rate);

        self.lfo_inc = self.lfo_inc * old_sr / sample_rate;

        let new_size = (self.max_delay_seconds * sample_rate) as usize;
        if new_size > self.buffer.len() {
            self.buffer.resize(new_size, 0.0);
        }
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_ptr = 0;
        self.lfo_phase = 0.0;
        self.filter_state = 0.0;
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "TapeDelay"
    }
}
