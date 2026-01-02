use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use alloc::vec::Vec;

/// The type of curve to use for mapping.
#[derive(Clone, Copy)]
pub enum CurveType {
    /// Linear interpolation.
    Linear,
    /// Exponential interpolation (suitable for frequency).
    Exponential,
}

/// Maps an input signal (0.0 - 1.0) to a range [min, max].
pub struct MapRange {
    input: AudioParam,
    min: AudioParam,
    max: AudioParam,
    curve: CurveType,

    input_buffer: Vec<f32>,
    min_buffer: Vec<f32>,
    max_buffer: Vec<f32>,
}

impl MapRange {
    /// Creates a new MapRange processor.
    ///
    /// # Arguments
    /// * `input` - The input signal (expected 0.0 - 1.0).
    /// * `min` - The minimum output value.
    /// * `max` - The maximum output value.
    /// * `curve` - The interpolation curve.
    pub fn new(input: AudioParam, min: AudioParam, max: AudioParam, curve: CurveType) -> Self {
        MapRange {
            input,
            min,
            max,
            curve,
            input_buffer: Vec::new(),
            min_buffer: Vec::new(),
            max_buffer: Vec::new(),
        }
    }
}

impl FrameProcessor for MapRange {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.input_buffer.len() < len { self.input_buffer.resize(len, 0.0); }
        if self.min_buffer.len() < len { self.min_buffer.resize(len, 0.0); }
        if self.max_buffer.len() < len { self.max_buffer.resize(len, 0.0); }

        self.input.process(&mut self.input_buffer, sample_index);
        self.min.process(&mut self.min_buffer, sample_index);
        self.max.process(&mut self.max_buffer, sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let t = self.input_buffer[i].clamp(0.0, 1.0);
            let min_val = self.min_buffer[i];
            let max_val = self.max_buffer[i];

            *sample = match self.curve {
                CurveType::Linear => min_val + (max_val - min_val) * t,
                CurveType::Exponential => {
                    if min_val.abs() < 1e-6 || (min_val < 0.0 && max_val > 0.0) {
                        min_val + (max_val - min_val) * t
                    } else {
                        let ratio = max_val / min_val;
                        min_val * libm::powf(ratio, t)
                    }
                }
            };
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.input.set_sample_rate(sample_rate);
        self.min.set_sample_rate(sample_rate);
        self.max.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "MapRange"
    }
}
