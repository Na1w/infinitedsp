use super::frame_processor::FrameProcessor;
use super::channels::{ChannelConfig, Mono, Stereo, MonoToStereo, StereoToMono};
use super::parallel_mixer::ParallelMixer;
use crate::core::audio_param::AudioParam;
use core::marker::PhantomData;
use alloc::string::String;
#[cfg(feature = "debug_visualize")]
use alloc::format;

/// A processor that runs two processors in series.
///
/// This is the building block for `StaticDspChain`.
pub struct SerialProcessor<P1, P2> {
    pub first: P1,
    pub second: P2,
}

impl<P1, P2> SerialProcessor<P1, P2> {
    pub fn new(first: P1, second: P2) -> Self {
        Self { first, second }
    }
}

impl<C, P1, P2> FrameProcessor<C> for SerialProcessor<P1, P2>
where
    C: ChannelConfig,
    P1: FrameProcessor<C>,
    P2: FrameProcessor<C>,
{
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        self.first.process(buffer, sample_index);
        self.second.process(buffer, sample_index);
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.first.set_sample_rate(sample_rate);
        self.second.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.first.reset();
        self.second.reset();
    }

    fn latency_samples(&self) -> u32 {
        self.first.latency_samples() + self.second.latency_samples()
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "SerialProcessor"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> String {
        let spaces = " ".repeat(indent);
        let mut output = String::new();

        output.push_str(&self.first.visualize(indent));
        output.push_str(&format!("{}|\n", spaces));
        output.push_str(&format!("{}v\n", spaces));
        output.push_str(&self.second.visualize(indent));

        output
    }
}

/// A statically typed chain of DSP processors.
///
/// Unlike `DspChain`, which uses `Vec<Box<dyn FrameProcessor>>` and dynamic dispatch,
/// `StaticDspChain` composes processors using generics (`SerialProcessor`).
/// This allows the compiler to inline and optimize the entire processing chain ("kernel fusion").
pub struct StaticDspChain<C: ChannelConfig, P> {
    pub processor: P,
    sample_rate: f32,
    _marker: PhantomData<C>,
}

impl<C: ChannelConfig, P: FrameProcessor<C>> StaticDspChain<C, P> {
    /// Creates a new StaticDspChain starting with the given processor.
    pub fn new(mut processor: P, sample_rate: f32) -> Self {
        processor.set_sample_rate(sample_rate);
        Self {
            processor,
            sample_rate,
            _marker: PhantomData,
        }
    }

    /// Appends a processor to the chain.
    pub fn and<P2>(self, mut next: P2) -> StaticDspChain<C, SerialProcessor<P, P2>>
    where
        P2: FrameProcessor<C>,
    {
        next.set_sample_rate(self.sample_rate);
        let serial = SerialProcessor::new(self.processor, next);
        StaticDspChain {
            processor: serial,
            sample_rate: self.sample_rate,
            _marker: PhantomData,
        }
    }

    /// Appends a processor to the chain with a dry/wet mix.
    pub fn and_mix<P2>(self, mix: f32, mut next: P2) -> StaticDspChain<C, SerialProcessor<P, ParallelMixer<P2, C>>>
    where
        P2: FrameProcessor<C>,
    {
        next.set_sample_rate(self.sample_rate);
        let mixer = ParallelMixer::new(mix, next);
        let serial = SerialProcessor::new(self.processor, mixer);
        StaticDspChain {
            processor: serial,
            sample_rate: self.sample_rate,
            _marker: PhantomData,
        }
    }

    /// Appends a processor to the chain with a modulatable dry/wet mix.
    pub fn and_mix_param<P2>(self, mix: AudioParam, mut next: P2) -> StaticDspChain<C, SerialProcessor<P, ParallelMixer<P2, C>>>
    where
        P2: FrameProcessor<C>,
    {
        next.set_sample_rate(self.sample_rate);
        let mut mixer = ParallelMixer::new(0.0, next);
        mixer.set_mix(mix);

        let serial = SerialProcessor::new(self.processor, mixer);
        StaticDspChain {
            processor: serial,
            sample_rate: self.sample_rate,
            _marker: PhantomData,
        }
    }

    /// Returns a graph visualization of the entire chain.
    pub fn get_graph(&self) -> String {
        #[cfg(feature = "debug_visualize")]
        {
            self.visualize(0)
        }
        #[cfg(not(feature = "debug_visualize"))]
        {
            String::new()
        }
    }
}

impl<P: FrameProcessor<Mono> + Send> StaticDspChain<Mono, P> {
    /// Converts the Mono chain into a Stereo chain.
    pub fn to_stereo(self) -> StaticDspChain<Stereo, MonoToStereo<P>> {
        let converted = MonoToStereo::new(self.processor);
        StaticDspChain::new(converted, self.sample_rate)
    }
}

impl<P: FrameProcessor<Stereo> + Send> StaticDspChain<Stereo, P> {
    /// Converts the Stereo chain into a Mono chain.
    pub fn to_mono(self) -> StaticDspChain<Mono, StereoToMono<P>> {
        let converted = StereoToMono::new(self.processor);
        StaticDspChain::new(converted, self.sample_rate)
    }
}

impl<C: ChannelConfig, P: FrameProcessor<C>> FrameProcessor<C> for StaticDspChain<C, P> {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        self.processor.process(buffer, sample_index);
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.processor.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.processor.reset();
    }

    fn latency_samples(&self) -> u32 {
        self.processor.latency_samples()
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "StaticDspChain"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> String {
        let spaces = " ".repeat(indent);
        let arrow_spaces = " ".repeat(indent + 2);

        let channel_type = if C::num_channels() == 1 {
            "Mono"
        } else {
            "Stereo"
        };

        let mut output = String::new();
        output.push_str(&format!("{}StaticDspChain ({})\n", spaces, channel_type));
        output.push_str(&format!("{}|\n", arrow_spaces));
        output.push_str(&format!("{}v\n", arrow_spaces));

        output.push_str(&self.processor.visualize(indent));

        output.push_str(&format!("{}|\n", arrow_spaces));
        output.push_str(&format!("{}v\n", arrow_spaces));
        output.push_str(&format!("{}Output\n", spaces));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::utility::gain::Gain;

    #[test]
    fn test_static_chain_build_and_process() {
        // Create a chain: Gain(0.5) -> Gain(0.5)
        // Input 1.0 -> 0.5 -> 0.25

        let g1 = Gain::new(AudioParam::Static(0.5));
        let g2 = Gain::new(AudioParam::Static(0.5));

        let mut chain = StaticDspChain::<Mono, _>::new(g1, 44100.0)
            .and(g2);

        let mut buffer = [1.0; 4];
        chain.process(&mut buffer, 0);

        for &s in buffer.iter() {
            assert!((s - 0.25).abs() < 1e-6);
        }
    }

    #[test]
    fn test_static_chain_stereo_conversion() {
        let g1 = Gain::new(AudioParam::Static(0.5));
        let mut chain = StaticDspChain::<Mono, _>::new(g1, 44100.0)
            .to_stereo();

        // Input stereo buffer (interleaved)
        let mut buffer = [1.0; 4]; // 2 frames stereo

        chain.process(&mut buffer, 0);

        for &s in buffer.iter() {
            assert!((s - 0.5).abs() < 1e-6);
        }
    }
}
