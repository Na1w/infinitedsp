use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use core::f32::consts::PI;
use alloc::vec::Vec;
use alloc::vec;

/// A tape delay simulation with saturation, wow/flutter, and low-pass filtering.
pub struct TapeDelay {
    buffer: Vec<f32>,
    write_ptr: usize,

    base_delay: AudioParam,

    lfo_phase: f32,
    lfo_inc: f32,
    depth: f32,

    feedback: AudioParam,
    mix: AudioParam,
    drive: AudioParam,
    lowpass_coeff: f32,

    filter_state: f32,
    sample_rate: f32,

    max_delay_s: f32,
    flutter_amount: f32,

    delay_buffer: Vec<f32>,
    feedback_buffer: Vec<f32>,
    mix_buffer: Vec<f32>,
    drive_buffer: Vec<f32>,
}

impl TapeDelay {
    /// Creates a new TapeDelay.
    ///
    /// # Arguments
    /// * `max_delay_s` - Maximum buffer size in seconds.
    /// * `delay_time` - Delay time in seconds.
    /// * `feedback` - Feedback amount (0.0 - 1.0).
    /// * `mix` - Dry/Wet mix (0.0 - 1.0).
    pub fn new(max_delay_s: f32, delay_time: AudioParam, feedback: AudioParam, mix: AudioParam) -> Self {
        let sample_rate = 44100.0;
        let buffer_size = (sample_rate * (max_delay_s + 0.1)) as usize;

        TapeDelay {
            buffer: vec![0.0; buffer_size],
            write_ptr: 0,
            base_delay: delay_time,

            lfo_phase: 0.0,
            lfo_inc: 2.0 * PI * 0.5 / sample_rate,
            depth: 0.002 * sample_rate,

            feedback,
            mix,
            drive: AudioParam::Static(1.2),
            lowpass_coeff: 0.0,

            filter_state: 0.0,
            sample_rate,

            max_delay_s,
            flutter_amount: 0.5,

            delay_buffer: Vec::new(),
            feedback_buffer: Vec::new(),
            mix_buffer: Vec::new(),
            drive_buffer: Vec::new(),
        }
    }

    /// Sets the delay time parameter.
    pub fn set_delay_time(&mut self, delay_time: AudioParam) {
        self.base_delay = delay_time;
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

    fn recalc_filter(&mut self) {
        let cutoff = 2000.0;
        let dt = 1.0 / self.sample_rate;
        let rc = 1.0 / (2.0 * PI * cutoff);
        self.lowpass_coeff = dt / (rc + dt);
    }
}

impl FrameProcessor for TapeDelay {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        if self.lowpass_coeff == 0.0 { self.recalc_filter(); }

        let len = self.buffer.len();
        let len_f = len as f32;
        let block_size = buffer.len();

        if self.delay_buffer.len() < block_size { self.delay_buffer.resize(block_size, 0.0); }
        if self.feedback_buffer.len() < block_size { self.feedback_buffer.resize(block_size, 0.0); }
        if self.mix_buffer.len() < block_size { self.mix_buffer.resize(block_size, 0.0); }
        if self.drive_buffer.len() < block_size { self.drive_buffer.resize(block_size, 0.0); }

        self.base_delay.process(&mut self.delay_buffer[0..block_size], sample_index);
        self.feedback.process(&mut self.feedback_buffer[0..block_size], sample_index);
        self.mix.process(&mut self.mix_buffer[0..block_size], sample_index);
        self.drive.process(&mut self.drive_buffer[0..block_size], sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let input = *sample;
            let delay_s = self.delay_buffer[i];
            let fb = self.feedback_buffer[i];
            let mix = self.mix_buffer[i];
            let drive = self.drive_buffer[i];

            let base_delay_samples = delay_s * self.sample_rate;

            self.lfo_phase += self.lfo_inc;
            if self.lfo_phase > 2.0 * PI { self.lfo_phase -= 2.0 * PI; }

            let lfo = libm::sinf(self.lfo_phase);
            let current_delay = base_delay_samples + lfo * self.depth * self.flutter_amount;

            let read_pos = (self.write_ptr as f32 - current_delay + len_f) % len_f;
            let idx_a = read_pos as usize;
            let idx_b = (idx_a + 1) % len;
            let frac = read_pos - idx_a as f32;

            let raw_delayed = self.buffer[idx_a] * (1.0 - frac) + self.buffer[idx_b] * frac;

            self.filter_state += self.lowpass_coeff * (raw_delayed - self.filter_state);
            let filtered = self.filter_state;

            let saturated = libm::tanhf(filtered * drive);

            let feedback_signal = saturated * fb;
            let tape_input = libm::tanhf(input + feedback_signal);

            self.buffer[self.write_ptr] = tape_input;

            *sample = input * (1.0 - mix) + raw_delayed * mix;

            self.write_ptr = (self.write_ptr + 1) % len;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        let old_sr = self.sample_rate;
        self.sample_rate = sample_rate;
        self.base_delay.set_sample_rate(sample_rate);
        self.feedback.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);
        self.drive.set_sample_rate(sample_rate);

        self.lfo_inc = self.lfo_inc * old_sr / sample_rate;
        self.depth = self.depth * sample_rate / old_sr;
        self.recalc_filter();

        let needed = (sample_rate * (self.max_delay_s + 0.1)) as usize;
        if needed > self.buffer.len() {
            self.buffer.resize(needed, 0.0);
        }
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "TapeDelay"
    }
}
