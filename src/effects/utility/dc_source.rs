use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;

/// Generates a constant DC signal.
///
/// Useful for control signals or testing.
pub struct DcSource {
    value: AudioParam,
}

impl DcSource {
    /// Creates a new DC source.
    ///
    /// # Arguments
    /// * `value` - The value to output.
    pub fn new(value: AudioParam) -> Self {
        DcSource { value }
    }
}

impl FrameProcessor<Mono> for DcSource {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        self.value.process(buffer, sample_index);
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.value.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "DcSource"
    }
}
