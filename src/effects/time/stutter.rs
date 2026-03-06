use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;

/// A Stutter effect with windowing and dry/wet control.
///
/// Records the incoming audio into a buffer and repeats it when triggered.
pub struct Stutter {
    buffer: Vec<f32>,
    write_pos: usize,
    sample_rate: f32,

    length: AudioParam,
    repeats: AudioParam,
    trigger: AudioParam,
    mix: AudioParam,

    is_stuttering: bool,
    stutter_read_start_pos: usize,
    stutter_read_pos: f32,
    stutter_len_samples: usize,
    remaining_samples: i32,
    last_trigger: f32,
}

impl Stutter {
    /// Creates a new Stutter effect.
    ///
    /// # Arguments
    /// * `max_delay_ms` - Maximum length of the stutter buffer in milliseconds.
    /// * `length` - Length of the stutter segment (as an [`AudioParam`]).
    /// * `repeats` - Number of times to repeat the segment (as an [`AudioParam`]).
    /// * `trigger` - When this value > 0.5, the stutter effect starts (as an [`AudioParam`]).
    pub fn new(
        max_delay_ms: f32,
        length: AudioParam,
        repeats: AudioParam,
        trigger: AudioParam,
    ) -> Self {
        let sample_rate = 44100.0;
        let buffer_size = (max_delay_ms / 1000.0 * sample_rate) as usize + 1024;
        Stutter {
            buffer: vec![0.0; buffer_size],
            write_pos: 0,
            sample_rate,
            length,
            repeats,
            trigger,
            mix: AudioParam::Static(1.0),
            is_stuttering: false,
            stutter_read_start_pos: 0,
            stutter_read_pos: 0.0,
            stutter_len_samples: 0,
            remaining_samples: 0,
            last_trigger: 0.0,
        }
    }

    /// Sets the dry/wet mix.
    pub fn set_mix(&mut self, mix: AudioParam) {
        self.mix = mix;
    }

    /// Sets the trigger parameter.
    pub fn set_trigger(&mut self, trigger: AudioParam) {
        self.trigger = trigger;
    }

    /// Sets the number of repeats.
    pub fn set_repeats(&mut self, repeats: AudioParam) {
        self.repeats = repeats;
    }

    /// Sets the stutter length.
    pub fn set_length(&mut self, length: AudioParam) {
        self.length = length;
    }
}

impl FrameProcessor<Mono> for Stutter {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let sample_rate = self.sample_rate;
        let buffer_len = self.buffer.len();

        for (i, sample) in buffer.iter_mut().enumerate() {
            let current_idx = sample_index + i as u64;

            let trig = self.trigger.get_value_at(current_idx);
            let target_len_sec = self.length.get_value_at(current_idx);
            let target_reps = self.repeats.get_value_at(current_idx);
            let mix = self.mix.get_value_at(current_idx);

            if trig > 0.5 && self.last_trigger <= 0.5 {
                self.is_stuttering = true;
                self.stutter_len_samples = (target_len_sec * sample_rate) as usize;
                self.stutter_len_samples = self.stutter_len_samples.clamp(10, buffer_len - 1);
                self.stutter_read_start_pos =
                    (self.write_pos + buffer_len - self.stutter_len_samples) % buffer_len;
                self.remaining_samples = (self.stutter_len_samples as f32 * target_reps) as i32;
                self.stutter_read_pos = 0.0;
            }
            self.last_trigger = trig;

            let input = *sample;
            self.buffer[self.write_pos] = input;
            self.write_pos = (self.write_pos + 1) % buffer_len;

            if self.is_stuttering {
                let pos = self.stutter_read_pos as usize;
                let read_idx = (self.stutter_read_start_pos + pos) % buffer_len;

                let fade_samples = (self.stutter_len_samples / 20).max(1);
                let mut envelope = 1.0;

                if pos < fade_samples {
                    envelope = pos as f32 / fade_samples as f32;
                } else if pos > self.stutter_len_samples - fade_samples {
                    envelope = (self.stutter_len_samples - pos) as f32 / fade_samples as f32;
                }

                let stutter_out = self.buffer[read_idx] * envelope;
                *sample = input * (1.0 - mix) + stutter_out * mix;

                self.stutter_read_pos += 1.0;
                if self.stutter_read_pos >= self.stutter_len_samples as f32 {
                    self.stutter_read_pos = 0.0;
                }

                if self.remaining_samples > 0 {
                    self.remaining_samples -= 1;
                    if self.remaining_samples <= 0 {
                        self.is_stuttering = false;
                    }
                }
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        if (self.sample_rate - sample_rate).abs() > 0.1 {
            let old_sr = self.sample_rate;
            self.sample_rate = sample_rate;
            let new_size = (self.buffer.len() as f32 * (sample_rate / old_sr)) as usize;
            self.buffer.resize(new_size, 0.0);
            self.write_pos = 0;
        }
        self.length.set_sample_rate(sample_rate);
        self.repeats.set_sample_rate(sample_rate);
        self.trigger.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.is_stuttering = false;
        self.last_trigger = 0.0;
        self.length.reset();
        self.repeats.reset();
        self.trigger.reset();
        self.mix.reset();
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Stutter"
    }
}
