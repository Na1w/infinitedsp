use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec::Vec;

/// A dynamic range compressor.
///
/// Reduces the volume of loud sounds or amplifies quiet sounds by narrowing or compressing an audio signal's dynamic range.
pub struct Compressor {
    threshold_db: AudioParam,
    ratio: AudioParam,
    attack_ms: AudioParam,
    release_ms: AudioParam,
    makeup_gain_db: AudioParam,
    sample_rate: f32,

    attack_coeff: f32,
    release_coeff: f32,
    envelope: f32,

    threshold_buffer: Vec<f32>,
    ratio_buffer: Vec<f32>,
    attack_buffer: Vec<f32>,
    release_buffer: Vec<f32>,
    makeup_buffer: Vec<f32>,

    last_attack_bits: u32,
    last_release_bits: u32,
}

impl Compressor {
    /// Creates a new Compressor.
    ///
    /// # Arguments
    /// * `threshold_db` - The level above which compression starts (in dB).
    /// * `ratio` - The amount of gain reduction (e.g., 4.0 for 4:1).
    pub fn new(threshold_db: AudioParam, ratio: AudioParam) -> Self {
        let mut c = Compressor {
            threshold_db,
            ratio,
            attack_ms: AudioParam::Static(10.0),
            release_ms: AudioParam::Static(100.0),
            makeup_gain_db: AudioParam::Static(0.0),
            sample_rate: 44100.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            envelope: 0.0,
            threshold_buffer: Vec::new(),
            ratio_buffer: Vec::new(),
            attack_buffer: Vec::new(),
            release_buffer: Vec::new(),
            makeup_buffer: Vec::new(),
            last_attack_bits: u32::MAX,
            last_release_bits: u32::MAX,
        };
        c.recalc(10.0, 100.0);
        c
    }

    /// Creates a Compressor configured as a Limiter.
    ///
    /// Sets a high ratio and fast attack/release times.
    pub fn new_limiter() -> Self {
        let mut c = Self::new(AudioParam::Static(-0.1), AudioParam::Static(100.0));
        c.attack_ms = AudioParam::Static(1.0);
        c.release_ms = AudioParam::Static(50.0);
        c.recalc(1.0, 50.0);
        c
    }

    /// Sets the threshold parameter.
    pub fn set_threshold(&mut self, threshold: AudioParam) {
        self.threshold_db = threshold;
    }

    /// Sets the ratio parameter.
    pub fn set_ratio(&mut self, ratio: AudioParam) {
        self.ratio = ratio;
    }

    /// Sets the attack time parameter.
    pub fn set_attack(&mut self, attack: AudioParam) {
        self.attack_ms = attack;
    }

    /// Sets the release time parameter.
    pub fn set_release(&mut self, release: AudioParam) {
        self.release_ms = release;
    }

    /// Sets the makeup gain parameter.
    pub fn set_makeup(&mut self, makeup: AudioParam) {
        self.makeup_gain_db = makeup;
    }

    fn recalc(&mut self, attack_ms: f32, release_ms: f32) {
        self.attack_coeff = libm::expf(-1.0 / (attack_ms * self.sample_rate * 0.001));
        self.release_coeff = libm::expf(-1.0 / (release_ms * self.sample_rate * 0.001));
    }
}

impl FrameProcessor<Mono> for Compressor {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();

        if self.threshold_buffer.len() < len {
            self.threshold_buffer.resize(len, 0.0);
        }
        if self.ratio_buffer.len() < len {
            self.ratio_buffer.resize(len, 0.0);
        }
        if self.attack_buffer.len() < len {
            self.attack_buffer.resize(len, 0.0);
        }
        if self.release_buffer.len() < len {
            self.release_buffer.resize(len, 0.0);
        }
        if self.makeup_buffer.len() < len {
            self.makeup_buffer.resize(len, 0.0);
        }

        self.threshold_db
            .process(&mut self.threshold_buffer[0..len], sample_index);
        self.ratio
            .process(&mut self.ratio_buffer[0..len], sample_index);
        self.attack_ms
            .process(&mut self.attack_buffer[0..len], sample_index);
        self.release_ms
            .process(&mut self.release_buffer[0..len], sample_index);
        self.makeup_gain_db
            .process(&mut self.makeup_buffer[0..len], sample_index);

        if let (Some(thresh_db), Some(ratio), Some(att_ms), Some(rel_ms), Some(makeup_db)) = (
            self.threshold_db.get_constant(),
            self.ratio.get_constant(),
            self.attack_ms.get_constant(),
            self.release_ms.get_constant(),
            self.makeup_gain_db.get_constant(),
        ) {
            let att_bits = att_ms.to_bits();
            let rel_bits = rel_ms.to_bits();

            if att_bits != self.last_attack_bits || rel_bits != self.last_release_bits {
                self.recalc(att_ms, rel_ms);
                self.last_attack_bits = att_bits;
                self.last_release_bits = rel_bits;
            }

            let threshold_linear = libm::powf(10.0, thresh_db / 20.0);
            let makeup = libm::powf(10.0, makeup_db / 20.0);
            let inv_ratio_sub_one = 1.0 - 1.0 / ratio;

            for sample in buffer.iter_mut() {
                let input = *sample;
                let abs_input = input.abs();

                if abs_input > self.envelope {
                    self.envelope =
                        self.attack_coeff * self.envelope + (1.0 - self.attack_coeff) * abs_input;
                } else {
                    self.envelope =
                        self.release_coeff * self.envelope + (1.0 - self.release_coeff) * abs_input;
                }

                let mut gain = 1.0;
                if self.envelope > threshold_linear {
                    let env_db = 20.0 * libm::log10f(self.envelope);
                    let over_db = env_db - thresh_db;
                    let gain_db = -over_db * inv_ratio_sub_one;
                    gain = libm::powf(10.0, gain_db / 20.0);
                }

                *sample = input * gain * makeup;
            }
        } else {
            for (i, sample) in buffer.iter_mut().enumerate() {
                let threshold_db = self.threshold_buffer[i];
                let ratio = self.ratio_buffer[i];
                let attack_ms = self.attack_buffer[i];
                let release_ms = self.release_buffer[i];
                let makeup_db = self.makeup_buffer[i];

                let att_bits = attack_ms.to_bits();
                let rel_bits = release_ms.to_bits();

                if att_bits != self.last_attack_bits || rel_bits != self.last_release_bits {
                    self.recalc(attack_ms, release_ms);
                    self.last_attack_bits = att_bits;
                    self.last_release_bits = rel_bits;
                }

                let threshold_linear = libm::powf(10.0, threshold_db / 20.0);
                let makeup = libm::powf(10.0, makeup_db / 20.0);

                let input = *sample;
                let abs_input = input.abs();

                if abs_input > self.envelope {
                    self.envelope =
                        self.attack_coeff * self.envelope + (1.0 - self.attack_coeff) * abs_input;
                } else {
                    self.envelope =
                        self.release_coeff * self.envelope + (1.0 - self.release_coeff) * abs_input;
                }

                let mut gain = 1.0;
                if self.envelope > threshold_linear {
                    let env_db = 20.0 * libm::log10f(self.envelope);
                    let over_db = env_db - threshold_db;
                    let gain_db = -over_db * (1.0 - 1.0 / ratio);
                    gain = libm::powf(10.0, gain_db / 20.0);
                }

                *sample = input * gain * makeup;
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.threshold_db.set_sample_rate(sample_rate);
        self.ratio.set_sample_rate(sample_rate);
        self.attack_ms.set_sample_rate(sample_rate);
        self.release_ms.set_sample_rate(sample_rate);
        self.makeup_gain_db.set_sample_rate(sample_rate);
        self.last_attack_bits = u32::MAX;
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Compressor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter() {
        let mut limiter = Compressor::new_limiter();
        limiter.set_sample_rate(44100.0);

        let mut buffer = [2.0; 100];
        limiter.process(&mut buffer, 0);

        let last = buffer[99];
        assert!(last < 1.5);
        assert!(last > 0.0);
    }
}
