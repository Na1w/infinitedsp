use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec::Vec;

/// Generates a constant DC signal.
///
/// Useful for control signals or testing.
pub struct DcSource {
    value: AudioParam,
    buffer: Vec<f32>,
}

impl DcSource {
    /// Creates a new DcSource.
    ///
    /// # Arguments
    /// * `value` - The DC value to output.
    pub fn new(value: AudioParam) -> Self {
        DcSource {
            value,
            buffer: Vec::new(),
        }
    }
}

impl FrameProcessor<Mono> for DcSource {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.buffer.len() < len {
            self.buffer.resize(len, 0.0);
        }

        self.value.process(&mut self.buffer[0..len], sample_index);

        buffer.copy_from_slice(&self.buffer[0..len]);
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.value.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "DcSource"
    }
}
