use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use core::f32::consts::PI;
use alloc::vec::Vec;

struct Allpass {
    zm1: f32,
}

impl Allpass {
    fn new() -> Self {
        Allpass { zm1: 0.0 }
    }

    fn process(&mut self, input: f32, a1: f32) -> f32 {
        let y = input * -a1 + self.zm1;
        self.zm1 = input + y * a1;
        y
    }
}

/// A 6-stage phaser effect.
///
/// Creates sweeping notch filters by mixing the input with a phase-shifted version of itself.
pub struct Phaser {
    filters: [Allpass; 6],
    lfo_phase: f32,
    lfo_inc: f32,
    min_freq: AudioParam,
    max_freq: AudioParam,
    feedback: AudioParam,
    mix: AudioParam,
    sample_rate: f32,
    last_sample: f32,

    min_freq_buffer: Vec<f32>,
    max_freq_buffer: Vec<f32>,
    feedback_buffer: Vec<f32>,
    mix_buffer: Vec<f32>,
}

impl Phaser {
    /// Creates a new Phaser.
    ///
    /// # Arguments
    /// * `min_freq` - Minimum frequency of the sweep (Hz).
    /// * `max_freq` - Maximum frequency of the sweep (Hz).
    /// * `feedback` - Feedback amount (0.0 - 1.0).
    /// * `mix` - Dry/Wet mix (0.0 - 1.0).
    pub fn new(min_freq: AudioParam, max_freq: AudioParam, feedback: AudioParam, mix: AudioParam) -> Self {
        let filters = [
            Allpass::new(), Allpass::new(), Allpass::new(),
            Allpass::new(), Allpass::new(), Allpass::new()
        ];
        let sample_rate = 44100.0;

        Phaser {
            filters,
            lfo_phase: 0.0,
            lfo_inc: 2.0 * PI * 0.5 / sample_rate,
            min_freq,
            max_freq,
            feedback,
            mix,
            sample_rate,
            last_sample: 0.0,
            min_freq_buffer: Vec::new(),
            max_freq_buffer: Vec::new(),
            feedback_buffer: Vec::new(),
            mix_buffer: Vec::new(),
        }
    }

    /// Sets the minimum frequency parameter.
    pub fn set_min_freq(&mut self, min_freq: AudioParam) {
        self.min_freq = min_freq;
    }

    /// Sets the maximum frequency parameter.
    pub fn set_max_freq(&mut self, max_freq: AudioParam) {
        self.max_freq = max_freq;
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

impl FrameProcessor for Phaser {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.min_freq_buffer.len() < len { self.min_freq_buffer.resize(len, 0.0); }
        if self.max_freq_buffer.len() < len { self.max_freq_buffer.resize(len, 0.0); }
        if self.feedback_buffer.len() < len { self.feedback_buffer.resize(len, 0.0); }
        if self.mix_buffer.len() < len { self.mix_buffer.resize(len, 0.0); }

        self.min_freq.process(&mut self.min_freq_buffer[0..len], sample_index);
        self.max_freq.process(&mut self.max_freq_buffer[0..len], sample_index);
        self.feedback.process(&mut self.feedback_buffer[0..len], sample_index);
        self.mix.process(&mut self.mix_buffer[0..len], sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let min_freq = self.min_freq_buffer[i];
            let max_freq = self.max_freq_buffer[i];
            let feedback = self.feedback_buffer[i];
            let mix = self.mix_buffer[i];

            let input = *sample + self.last_sample * feedback;

            self.lfo_phase += self.lfo_inc;
            if self.lfo_phase > 2.0 * PI { self.lfo_phase -= 2.0 * PI; }

            let lfo = (libm::sinf(self.lfo_phase) + 1.0) * 0.5;
            let freq = min_freq + lfo * (max_freq - min_freq);

            let w = 2.0 * PI * freq / self.sample_rate;
            let tan = libm::tanf(w * 0.5);

            let a1 = (1.0 - tan) / (1.0 + tan);

            let mut out = input;
            for filter in &mut self.filters {
                out = filter.process(out, a1);
            }

            self.last_sample = out;
            *sample = *sample * (1.0 - mix) + out * mix;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        let old_sr = self.sample_rate;
        self.sample_rate = sample_rate;
        self.min_freq.set_sample_rate(sample_rate);
        self.max_freq.set_sample_rate(sample_rate);
        self.feedback.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);
        self.lfo_inc = self.lfo_inc * old_sr / sample_rate;
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Phaser"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phaser() {
        let mut phaser = Phaser::new(
            AudioParam::Static(200.0),
            AudioParam::Static(2000.0),
            AudioParam::Static(0.5),
            AudioParam::Static(0.5)
        );
        let mut buffer = [1.0; 100];
        phaser.process(&mut buffer, 0);

        assert!(buffer[0].is_finite());
        assert!((buffer[99] - 1.0).abs() > 0.0001);
    }
}
