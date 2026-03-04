use crate::core::channels::ChannelConfig;
use crate::FrameProcessor;
use core::marker::PhantomData;

/// A wrapper that allows bypassing any FrameProcessor.
///
/// When enabled, the processor is executed normally.
/// When disabled, the input is passed through untouched.
pub struct Bypass<T, C: ChannelConfig> {
    processor: T,
    enabled: bool,
    _marker: PhantomData<C>,
}

impl<T, C: ChannelConfig> Bypass<T, C> {
    /// Creates a new Bypass wrapper.
    pub fn new(processor: T, enabled: bool) -> Self {
        Self {
            processor,
            enabled,
            _marker: PhantomData,
        }
    }

    /// Sets whether the effect is enabled or bypassed.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns true if the effect is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns a reference to the inner processor.
    pub fn processor(&self) -> &T {
        &self.processor
    }

    /// Returns a mutable reference to the inner processor.
    pub fn processor_mut(&mut self) -> &mut T {
        &mut self.processor
    }
}

impl<T, C: ChannelConfig> FrameProcessor<C> for Bypass<T, C>
where
    T: FrameProcessor<C>,
{
    fn process(&mut self, buffer: &mut [f32], frame_index: u64) {
        if self.enabled {
            self.processor.process(buffer, frame_index);
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.processor.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.processor.reset();
    }

    fn latency_samples(&self) -> u32 {
        if self.enabled {
            self.processor.latency_samples()
        } else {
            0
        }
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        if self.enabled {
            self.processor.name()
        } else {
            "Bypass"
        }
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> alloc::string::String {
        if self.enabled {
            self.processor.visualize(indent)
        } else {
            let mut s = alloc::string::String::new();
            for _ in 0..indent {
                s.push_str("  ");
            }
            s.push_str("[Bypassed] ");
            s.push_str(self.processor.name());
            s
        }
    }
}
