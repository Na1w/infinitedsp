/// The core trait for all audio processors.
///
/// Implementors must handle processing a block of audio samples.
pub trait FrameProcessor {
    /// Processes a block of audio samples.
    ///
    /// # Arguments
    /// * `buffer` - The audio buffer to process (in-place).
    /// * `sample_index` - The global sample index of the start of the block.
    fn process(&mut self, buffer: &mut [f32], sample_index: u64);

    /// Sets the sample rate.
    ///
    /// Should be called before processing starts or when sample rate changes.
    fn set_sample_rate(&mut self, _sample_rate: f32) {}

    /// Returns the latency of the processor in samples.
    ///
    /// Used for delay compensation.
    fn latency_samples(&self) -> u32 { 0 }
}
