use super::frame_processor::FrameProcessor;
use super::parallel_mixer::ParallelMixer;
use crate::core::audio_param::AudioParam;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::vec;
use alloc::string::String;
#[cfg(feature = "debug_visualize")]
use alloc::format;

/// A chain of DSP processors.
///
/// Processes audio sequentially through a list of processors.
pub struct DspChain {
    processors: Vec<Box<dyn FrameProcessor + Send>>,
    sample_rate: f32,
}

impl DspChain {
    /// Creates a new DspChain starting with the given processor.
    pub fn new(mut first: impl FrameProcessor + Send + 'static, sample_rate: f32) -> Self {
        first.set_sample_rate(sample_rate);
        DspChain {
            processors: vec![Box::new(first)],
            sample_rate,
        }
    }

    /// Appends a processor to the chain.
    pub fn and(mut self, mut processor: impl FrameProcessor + Send + 'static) -> Self {
        processor.set_sample_rate(self.sample_rate);
        self.processors.push(Box::new(processor));
        self
    }

    /// Appends a processor to the chain with a dry/wet mix.
    pub fn and_mix(mut self, mix: f32, mut processor: impl FrameProcessor + Send + 'static) -> Self {
        processor.set_sample_rate(self.sample_rate);
        let mixed = ParallelMixer::new(mix, processor);
        self.processors.push(Box::new(mixed));
        self
    }

    /// Appends a processor to the chain with a modulatable dry/wet mix.
    pub fn and_mix_param(mut self, mix: AudioParam, mut processor: impl FrameProcessor + Send + 'static) -> Self {
        processor.set_sample_rate(self.sample_rate);
        let mut mixed = ParallelMixer::new(0.0, processor);
        mixed.set_mix(mix);
        self.processors.push(Box::new(mixed));
        self
    }

    /// Returns a graph visualization of the entire chain.
    pub fn get_graph(&self) -> String {
        self.visualize(0)
    }
}

impl FrameProcessor for DspChain {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        for p in &mut self.processors {
            p.process(buffer, sample_index);
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        for p in &mut self.processors {
            p.set_sample_rate(sample_rate);
        }
    }

    fn latency_samples(&self) -> u32 {
        self.processors.iter().map(|p| p.latency_samples()).sum()
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "DspChain"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> String {
        let mut output = String::new();
        let spaces = " ".repeat(indent);
        let arrow_spaces = " ".repeat(indent + 2); // Indent arrows slightly

        output.push_str(&format!("{}DspChain Start\n", spaces));
        output.push_str(&format!("{}|\n", arrow_spaces));
        output.push_str(&format!("{}v\n", arrow_spaces));

        for (i, p) in self.processors.iter().enumerate() {
            output.push_str(&p.visualize(indent));
            if i < self.processors.len() - 1 {
                output.push_str(&format!("{}|\n", arrow_spaces));
                output.push_str(&format!("{}v\n", arrow_spaces));
            }
        }

        output.push_str(&format!("{}|\n", arrow_spaces));
        output.push_str(&format!("{}v\n", arrow_spaces));
        output.push_str(&format!("{}Output\n", spaces));

        output
    }
}
