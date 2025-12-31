use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use core::f32::consts::PI;
use alloc::vec::Vec;

/// The output type of the State Variable Filter.
#[derive(Clone, Copy)]
pub enum SvfType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
    Peak,
}

/// A State Variable Filter (SVF).
///
/// A stable and versatile filter that provides simultaneous low-pass, high-pass, band-pass and notch outputs.
/// This implementation uses the TPT (Topology Preserving Transform) / ZDF (Zero Delay Feedback) method
/// for excellent stability and response across the frequency range.
pub struct StateVariableFilter {
    filter_type: SvfType,
    cutoff: AudioParam,
    resonance: AudioParam,
    sample_rate: f32,

    s1: f32,
    s2: f32,

    cutoff_buffer: Vec<f32>,
    res_buffer: Vec<f32>,
}

impl StateVariableFilter {
    /// Creates a new StateVariableFilter.
    ///
    /// # Arguments
    /// * `filter_type` - The output type.
    /// * `cutoff` - Cutoff frequency in Hz.
    /// * `resonance` - Resonance (Q). 0.0 to 1.0 (or higher for self-oscillation).
    pub fn new(filter_type: SvfType, cutoff: AudioParam, resonance: AudioParam) -> Self {
        StateVariableFilter {
            filter_type,
            cutoff,
            resonance,
            sample_rate: 44100.0,
            s1: 0.0,
            s2: 0.0,
            cutoff_buffer: Vec::new(),
            res_buffer: Vec::new(),
        }
    }

    /// Sets the filter type.
    pub fn set_type(&mut self, filter_type: SvfType) {
        self.filter_type = filter_type;
    }
}

impl FrameProcessor for StateVariableFilter {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.cutoff_buffer.len() < len { self.cutoff_buffer.resize(len, 0.0); }
        if self.res_buffer.len() < len { self.res_buffer.resize(len, 0.0); }

        self.cutoff.process(&mut self.cutoff_buffer[0..len], sample_index);
        self.resonance.process(&mut self.res_buffer[0..len], sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let cutoff_hz = self.cutoff_buffer[i];
            let res = self.res_buffer[i];

            let g = libm::tanf(PI * cutoff_hz / self.sample_rate);
            let k = 1.0 / res.max(0.1);

            let input = *sample;

            let denom = 1.0 + g * (g + k);
            let hp = (input - self.s1 * (g + k) - self.s2) / denom;
            let bp = g * hp + self.s1;
            let lp = g * bp + self.s2;

            self.s1 += 2.0 * g * hp;
            self.s2 += 2.0 * g * bp;

            let out = match self.filter_type {
                SvfType::LowPass => lp,
                SvfType::HighPass => hp,
                SvfType::BandPass => bp,
                SvfType::Notch => hp + lp,
                SvfType::Peak => lp - hp,
            };

            *sample = out;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.cutoff.set_sample_rate(sample_rate);
        self.resonance.set_sample_rate(sample_rate);
    }
}
