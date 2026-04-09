use crate::core::audio_param::AudioParam;
use crate::core::channels::ChannelConfig;
use crate::core::frame_processor::FrameProcessor;
use crate::core::latency_compensator::LatencyCompensator;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;
use wide::f32x4;

/// Sums multiple audio signals together, with optional gain and soft clipping.
///
/// Automatically synchronizes input latencies by adding delay to inputs with lower latency.
pub struct SummingMixer<
    C: ChannelConfig,
    T: FrameProcessor<C> + Send = Box<dyn FrameProcessor<C> + Send>,
> {
    inputs: Vec<T>,
    gain: AudioParam,
    soft_clip: bool,
    input_buffer: Vec<f32>,
    temp_buffer: Vec<f32>,
    gain_buffer: Vec<f32>,
    _marker: PhantomData<C>,
}

impl<C: ChannelConfig + 'static, T: FrameProcessor<C> + Send + 'static> SummingMixer<C, T> {
    /// Creates a new SummingMixer with the given inputs.
    pub fn new(inputs: Vec<T>) -> Self {
        SummingMixer {
            inputs,
            gain: AudioParam::Static(1.0),
            soft_clip: false,
            input_buffer: Vec::with_capacity(128),
            temp_buffer: Vec::with_capacity(128),
            gain_buffer: Vec::with_capacity(128),
            _marker: PhantomData,
        }
    }

    /// Creates a new SummingMixer and synchronizes latencies.
    /// 
    /// This is specifically for Boxed processors.
    pub fn new_sync(inputs: Vec<Box<dyn FrameProcessor<C> + Send>>) -> SummingMixer<C, Box<dyn FrameProcessor<C> + Send>> {
        let max_latency = inputs
            .iter()
            .map(|input| input.latency_samples())
            .max()
            .unwrap_or_default();

        let sync_inputs = inputs
            .into_iter()
            .map(|input| {
                if input.latency_samples() < max_latency {
                    let wrapped: Box<dyn FrameProcessor<C> + Send> = Box::new(LatencyCompensator::new(input, max_latency));
                    wrapped
                } else {
                    input
                }
            })
            .collect();

        SummingMixer::new(sync_inputs)
    }

    /// Sets the output gain.
    pub fn set_gain(&mut self, gain: AudioParam) {
        self.gain = gain;
    }

    /// Enables or disables soft clipping (tanh) on the output.
    pub fn set_soft_clip(&mut self, enabled: bool) {
        self.soft_clip = enabled;
    }

    /// Builder method to set gain.
    pub fn with_gain(mut self, gain: AudioParam) -> Self {
        self.gain = gain;
        self
    }

    /// Builder method to enable soft clipping.
    pub fn with_soft_clip(mut self, enabled: bool) -> Self {
        self.soft_clip = enabled;
        self
    }
}

impl<C: ChannelConfig, T: FrameProcessor<C> + Send> FrameProcessor<C> for SummingMixer<C, T> {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        if self.inputs.is_empty() {
            buffer.fill(0.0);
            return;
        }

        if self.inputs.len() == 1 {
            self.inputs[0].process(buffer, sample_index);
        } else {
            let len = buffer.len();
            if self.input_buffer.len() < len {
                self.input_buffer.resize(len, 0.0);
            }
            if self.temp_buffer.len() < len {
                self.temp_buffer.resize(len, 0.0);
            }

            self.input_buffer[0..len].copy_from_slice(buffer);
            
            self.inputs[0].process(buffer, sample_index);

            for input in &mut self.inputs[1..] {
                let temp_slice = &mut self.temp_buffer[0..len];
                temp_slice.copy_from_slice(&self.input_buffer[0..len]);
                
                input.process(temp_slice, sample_index);

                let (buf_chunks, buf_rem) = buffer.as_chunks_mut::<4>();
                let (temp_chunks, temp_rem) = temp_slice.as_chunks::<4>();

                for (buf_c, temp_c) in buf_chunks.iter_mut().zip(temp_chunks.iter()) {
                    let buf_v = f32x4::from(*buf_c);
                    let temp_v = f32x4::from(*temp_c);
                    let res = buf_v + temp_v;
                    *buf_c = res.to_array();
                }

                for (buf_s, temp_s) in buf_rem.iter_mut().zip(temp_rem.iter()) {
                    *buf_s += *temp_s;
                }
            }
        }

        let constant_gain = self.gain.get_constant();
        let skip_processing = !self.soft_clip && constant_gain == Some(1.0);

        if !skip_processing {
            let channels = C::num_channels();
            let frames = buffer.len() / channels;

            if self.gain_buffer.len() < frames {
                self.gain_buffer.resize(frames, 0.0);
            }

            let gain_slice = &mut self.gain_buffer[0..frames];
            self.gain.process(gain_slice, sample_index);

            for (i, sample) in buffer.iter_mut().enumerate() {
                let frame_idx = i / channels;
                let g = gain_slice[frame_idx];

                let mut val = *sample * g;

                if self.soft_clip {
                    val = libm::tanhf(val);
                }
                *sample = val;
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        for input in &mut self.inputs {
            input.set_sample_rate(sample_rate);
        }
        self.gain.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        for input in &mut self.inputs {
            input.reset();
        }
        self.input_buffer.fill(0.0);
        self.temp_buffer.fill(0.0);
    }

    fn latency_samples(&self) -> u32 {
        self.inputs
            .iter()
            .map(|input| input.latency_samples())
            .max()
            .unwrap_or_default()
    }

    fn name(&self) -> &str {
        "SummingMixer"
    }

    fn visualize(&self, indent: usize) -> String {
        #[cfg(feature = "debug_visualize")]
        {
            use core::fmt::Write;
            let mut output = String::new();
            let spaces = " ".repeat(indent);
            let child_indent = indent + 2;

            let _ = writeln!(output, "{}SummingMixer", spaces);

            for (i, input) in self.inputs.iter().enumerate() {
                let _ = writeln!(output, "{}Input {}:", " ".repeat(child_indent), i + 1);
                output.push_str(&input.visualize(child_indent + 2));
            }

            output
        }
        #[cfg(not(feature = "debug_visualize"))]
        {
            let _ = indent;
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::channels::Mono;
    use crate::effects::utility::passthrough::Passthrough;
    use crate::effects::utility::lookahead::Lookahead;
    use alloc::vec;

    #[test]
    fn test_summing_mixer_latency_compensation() {
        // Input signal is a single 1.0 at sample 0.
        // Input 1: Lookahead (latency: 5 samples)
        // Input 2: Passthrough (latency: 0 samples)
        // SummingMixer should sync both to 5 samples.
        // Result: 1.0 + 1.0 = 2.0 at sample 5.

        let input1: Box<dyn FrameProcessor<Mono> + Send> = Box::new(Lookahead::new(5));
        let input2: Box<dyn FrameProcessor<Mono> + Send> = Box::new(Passthrough::new());

        let mut mixer = SummingMixer::<Mono, Box<dyn FrameProcessor<Mono> + Send>>::new_sync(vec![input1, input2]);
        assert_eq!(mixer.latency_samples(), 5);

        let mut buffer = [0.0; 10];
        buffer[0] = 1.0;

        mixer.process(&mut buffer, 0);

        // Sample 0-4 should be 0.0 (due to 5 sample latency)
        for i in 0..5 {
            assert_eq!(buffer[i], 0.0);
        }

        // Sample 5 should be 2.0 (1.0 from each input, both delayed by 5 samples)
        assert_eq!(buffer[5], 2.0);
    }
}
