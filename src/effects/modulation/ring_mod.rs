use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use core::f32::consts::PI;
use alloc::vec::Vec;

/// A ring modulator effect.
///
/// Multiplies the input signal with a carrier sine wave.
pub struct RingMod {
    phase: f32,
    inc: f32,
    freq: AudioParam,
    mix: AudioParam,
    sample_rate: f32,

    freq_buffer: Vec<f32>,
    mix_buffer: Vec<f32>,
}

impl RingMod {
    /// Creates a new RingMod effect.
    ///
    /// # Arguments
    /// * `freq` - Carrier frequency in Hz.
    /// * `mix` - Dry/Wet mix (0.0 - 1.0).
    pub fn new(freq: AudioParam, mix: AudioParam) -> Self {
        let sample_rate = 44100.0;
        RingMod {
            phase: 0.0,
            inc: 0.0, // Will be updated in process
            freq,
            mix,
            sample_rate,
            freq_buffer: Vec::new(),
            mix_buffer: Vec::new(),
        }
    }

    /// Sets the carrier frequency parameter.
    pub fn set_freq(&mut self, freq: AudioParam) {
        self.freq = freq;
    }

    /// Sets the mix parameter.
    pub fn set_mix(&mut self, mix: AudioParam) {
        self.mix = mix;
    }
}

impl FrameProcessor for RingMod {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.freq_buffer.len() < len { self.freq_buffer.resize(len, 0.0); }
        if self.mix_buffer.len() < len { self.mix_buffer.resize(len, 0.0); }

        self.freq.process(&mut self.freq_buffer[0..len], sample_index);
        self.mix.process(&mut self.mix_buffer[0..len], sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let freq = self.freq_buffer[i];
            let mix = self.mix_buffer[i];

            self.inc = 2.0 * PI * freq / self.sample_rate;

            let current_phase = self.phase;

            self.phase += self.inc;
            if self.phase > 2.0 * PI { self.phase -= 2.0 * PI; }

            let carrier = libm::sinf(current_phase);
            let wet = *sample * carrier;

            *sample = *sample * (1.0 - mix) + wet * mix;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.freq.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_mod() {
        let mut rm = RingMod::new(AudioParam::Static(10.0), AudioParam::Static(1.0)); // 10Hz
        rm.set_sample_rate(100.0);

        let mut buffer = [1.0; 10];
        rm.process(&mut buffer, 0);

        let min = buffer.iter().fold(1.0f32, |a, &b| a.min(b));
        let max = buffer.iter().fold(-1.0f32, |a, &b| a.max(b));

        assert!(min < -0.5);
        assert!(max > 0.5);
    }
}
