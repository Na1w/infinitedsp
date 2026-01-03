use crate::core::audio_param::AudioParam;
use crate::core::channels::ChannelConfig;
use crate::FrameProcessor;
use alloc::vec::Vec;

/// The type of curve to use for mapping.
#[derive(Clone, Copy)]
pub enum CurveType {
    /// Linear mapping.
    Linear,
    /// Exponential mapping.
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
    /// * `input` - Input signal (expected 0.0 - 1.0).
    /// * `min` - Minimum output value.
    /// * `max` - Maximum output value.
    /// * `curve` - Mapping curve.
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

impl<C: ChannelConfig> FrameProcessor<C> for MapRange {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let channels = C::num_channels();
        let frames = buffer.len() / channels;

        if self.input_buffer.len() < frames {
            self.input_buffer.resize(frames, 0.0);
        }
        if self.min_buffer.len() < frames {
            self.min_buffer.resize(frames, 0.0);
        }
        if self.max_buffer.len() < frames {
            self.max_buffer.resize(frames, 0.0);
        }

        self.input
            .process(&mut self.input_buffer[0..frames], sample_index);
        self.min
            .process(&mut self.min_buffer[0..frames], sample_index);
        self.max
            .process(&mut self.max_buffer[0..frames], sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let frame_idx = i / channels;
            let input_val = self.input_buffer[frame_idx].clamp(0.0, 1.0);
            let min_val = self.min_buffer[frame_idx];
            let max_val = self.max_buffer[frame_idx];

            let t = match self.curve {
                CurveType::Linear => input_val,
                CurveType::Exponential => input_val * input_val,
            };

            *sample = min_val + t * (max_val - min_val);
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
