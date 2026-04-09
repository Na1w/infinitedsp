use crate::core::channels::ChannelConfig;
use crate::core::frame_processor::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;

/// A wrapper that ensures a processor has a specific total latency.
///
/// If the wrapped processor has less latency than the target, this wrapper
/// adds a delay line to compensate.
pub struct LatencyCompensator<C: ChannelConfig, P: FrameProcessor<C>> {
    processor: P,
    target_latency: u32,
    delay_line: Vec<f32>,
    write_ptr: usize,
    _marker: PhantomData<C>,
}

impl<C: ChannelConfig, P: FrameProcessor<C>> LatencyCompensator<C, P> {
    /// Creates a new LatencyCompensator.
    ///
    /// # Arguments
    /// * `processor` - The processor to wrap.
    /// * `target_latency` - The total latency this wrapper should report and maintain.
    pub fn new(processor: P, target_latency: u32) -> Self {
        let inner_latency = processor.latency_samples();
        assert!(
            target_latency >= inner_latency,
            "Target latency ({}) must be greater than or equal to inner latency ({})",
            target_latency,
            inner_latency
        );

        let delay_needed = (target_latency - inner_latency) as usize;
        let channels = C::num_channels();
        let buffer_size = delay_needed * channels;

        LatencyCompensator {
            processor,
            target_latency,
            delay_line: vec![0.0; buffer_size],
            write_ptr: 0,
            _marker: PhantomData,
        }
    }

    /// Returns the number of samples added by this compensator (excluding the inner processor).
    pub fn added_latency(&self) -> u32 {
        self.target_latency - self.processor.latency_samples()
    }
}

impl<C: ChannelConfig, P: FrameProcessor<C>> FrameProcessor<C> for LatencyCompensator<C, P> {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        self.processor.process(buffer, sample_index);

        let added_delay = self.added_latency() as usize;
        if added_delay == 0 {
            return;
        }

        let len = self.delay_line.len();
        if len == 0 {
            return;
        }

        for sample in buffer.iter_mut() {
            let out = self.delay_line[self.write_ptr];
            self.delay_line[self.write_ptr] = *sample;
            self.write_ptr += 1;
            if self.write_ptr >= len {
                self.write_ptr = 0;
            }
            *sample = out;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.processor.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.processor.reset();
        self.delay_line.fill(0.0);
        self.write_ptr = 0;
    }

    fn latency_samples(&self) -> u32 {
        self.target_latency
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "LatencyCompensator"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> alloc::string::String {
        use core::fmt::Write;
        let mut output = alloc::string::String::new();
        let spaces = " ".repeat(indent);
        let _ = writeln!(
            output,
            "{}LatencyCompensator (Total: {}, Added: {})",
            spaces,
            self.target_latency,
            self.added_latency()
        );
        output.push_str(&self.processor.visualize(indent + 2));
        output
    }
}
