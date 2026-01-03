use crate::core::channels::ChannelConfig;
use alloc::boxed::Box;
#[cfg(feature = "debug_visualize")]
use alloc::format;
use alloc::string::String;

/// The core trait for all audio processors.
///
/// Implementors must handle processing a block of audio samples.
/// The generic parameter `C` defines the channel configuration (Mono/Stereo).
pub trait FrameProcessor<C: ChannelConfig> {
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
    fn latency_samples(&self) -> u32 {
        0
    }

    /// Returns the name of the processor.
    fn name(&self) -> &str {
        #[cfg(feature = "debug_visualize")]
        {
            "Node"
        }
        #[cfg(not(feature = "debug_visualize"))]
        {
            ""
        }
    }

    /// Returns an ASCII visualization of the processor structure.
    fn visualize(&self, indent: usize) -> String {
        #[cfg(feature = "debug_visualize")]
        {
            let spaces = " ".repeat(indent);
            format!("{}{}\n", spaces, self.name())
        }
        #[cfg(not(feature = "debug_visualize"))]
        {
            let _ = indent;
            String::new()
        }
    }
}

impl<C: ChannelConfig, T: FrameProcessor<C> + ?Sized> FrameProcessor<C> for Box<T> {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        (**self).process(buffer, sample_index);
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        (**self).set_sample_rate(sample_rate);
    }

    fn latency_samples(&self) -> u32 {
        (**self).latency_samples()
    }

    fn name(&self) -> &str {
        (**self).name()
    }

    fn visualize(&self, indent: usize) -> String {
        (**self).visualize(indent)
    }
}
