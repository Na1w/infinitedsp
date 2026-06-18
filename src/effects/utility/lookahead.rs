use crate::core::channels::ChannelConfig;
use crate::core::frame_processor::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;

/// A simple lookahead processor that introduces a fixed latency.
///
/// This is used to delay a signal so that other processors can "see" into the future,
/// or to test latency compensation systems.
pub struct Lookahead<C: ChannelConfig> {
    delay_samples: u32,
    buffer: Vec<f32>,
    write_ptr: usize,
    _marker: PhantomData<C>,
}

impl<C: ChannelConfig> Lookahead<C> {
    /// Creates a new Lookahead processor.
    ///
    /// # Arguments
    /// * `delay_samples` - The amount of latency to introduce.
    pub fn new(delay_samples: u32) -> Self {
        let channels = C::num_channels();
        let buffer_size = delay_samples as usize * channels;

        Lookahead {
            delay_samples,
            buffer: vec![0.0; buffer_size],
            write_ptr: 0,
            _marker: PhantomData,
        }
    }
}

impl<C: ChannelConfig> FrameProcessor<C> for Lookahead<C> {
    fn process(&mut self, buffer: &mut [f32], _sample_index: u64) {
        if self.delay_samples == 0 {
            return;
        }

        let len = self.buffer.len();
        for sample in buffer.iter_mut() {
            let out = self.buffer[self.write_ptr];
            self.buffer[self.write_ptr] = *sample;
            self.write_ptr += 1;
            if self.write_ptr >= len {
                self.write_ptr = 0;
            }
            *sample = out;
        }
    }

    fn set_sample_rate(&mut self, _sample_rate: f32) {}

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_ptr = 0;
    }

    fn latency_samples(&self) -> u32 {
        self.delay_samples
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Lookahead"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> alloc::string::String {
        use core::fmt::Write;
        let mut s = alloc::string::String::new();
        let spaces = " ".repeat(indent);
        let _ = writeln!(s, "{}Lookahead ({} samples)", spaces, self.delay_samples);
        s
    }
}
