use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use core::f32::consts::PI;
use alloc::vec::Vec;

/// A tremolo effect.
///
/// Modulates the amplitude of the signal using an LFO.
pub struct Tremolo {
    phase: f32,
    inc: f32,
    depth: AudioParam,
    rate: AudioParam,
    sample_rate: f32,

    depth_buffer: Vec<f32>,
    rate_buffer: Vec<f32>,
}

impl Tremolo {
    /// Creates a new Tremolo effect.
    ///
    /// # Arguments
    /// * `rate` - LFO rate in Hz.
    /// * `depth` - Modulation depth (0.0 - 1.0).
    pub fn new(rate: AudioParam, depth: AudioParam) -> Self {
        let sample_rate = 44100.0;
        Tremolo {
            phase: 0.0,
            inc: 0.0, // Will be updated in process
            depth,
            rate,
            sample_rate,
            depth_buffer: Vec::new(),
            rate_buffer: Vec::new(),
        }
    }

    /// Sets the depth parameter.
    pub fn set_depth(&mut self, depth: AudioParam) {
        self.depth = depth;
    }

    /// Sets the rate parameter.
    pub fn set_rate(&mut self, rate: AudioParam) {
        self.rate = rate;
    }
}

impl FrameProcessor for Tremolo {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.depth_buffer.len() < len { self.depth_buffer.resize(len, 0.0); }
        if self.rate_buffer.len() < len { self.rate_buffer.resize(len, 0.0); }

        self.depth.process(&mut self.depth_buffer[0..len], sample_index);
        self.rate.process(&mut self.rate_buffer[0..len], sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let depth = self.depth_buffer[i];
            let rate = self.rate_buffer[i];

            // Update inc based on current rate
            self.inc = 2.0 * PI * rate / self.sample_rate;

            let current_phase = self.phase;

            self.phase += self.inc;
            if self.phase > 2.0 * PI { self.phase -= 2.0 * PI; }

            let lfo = (libm::sinf(current_phase) + 1.0) * 0.5;
            let gain = 1.0 - depth * lfo;

            *sample *= gain;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.depth.set_sample_rate(sample_rate);
        self.rate.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Tremolo"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tremolo() {
        let mut trem = Tremolo::new(AudioParam::Static(10.0), AudioParam::Static(1.0));
        trem.set_sample_rate(100.0);

        let mut buffer = [1.0; 10];
        trem.process(&mut buffer, 0);

        let min = buffer.iter().fold(1.0f32, |a, &b| a.min(b));
        let max = buffer.iter().fold(0.0f32, |a, &b| a.max(b));

        assert!(min < 0.1);
        assert!(max > 0.9);
    }
}
