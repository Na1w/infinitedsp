use crate::core::audio_param::AudioParam;
use crate::core::channels::ChannelConfig;
use crate::core::frame_processor::FrameProcessor;
use alloc::boxed::Box;
#[cfg(feature = "debug_visualize")]
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;
use wide::f32x4;

/// Sums multiple audio signals together, with optional gain and soft clipping.
///
/// Useful for mixing multiple voices or signals.
pub struct SummingMixer<
    C: ChannelConfig,
    T: FrameProcessor<C> + Send = Box<dyn FrameProcessor<C> + Send>,
> {
    inputs: Vec<T>,
    gain: AudioParam,
    soft_clip: bool,
    temp_buffer: Vec<f32>,
    gain_buffer: Vec<f32>,
    _marker: PhantomData<C>,
}

impl<C: ChannelConfig, T: FrameProcessor<C> + Send> SummingMixer<C, T> {
    /// Creates a new SummingMixer with the given inputs.
    pub fn new(inputs: Vec<T>) -> Self {
        SummingMixer {
            inputs,
            gain: AudioParam::Static(1.0),
            soft_clip: false,
            temp_buffer: Vec::new(),
            gain_buffer: Vec::new(),
            _marker: PhantomData,
        }
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

        self.inputs[0].process(buffer, sample_index);

        if self.inputs.len() > 1 {
            if self.temp_buffer.len() < buffer.len() {
                self.temp_buffer.resize(buffer.len(), 0.0);
            }

            let len = buffer.len();
            let temp_slice = &mut self.temp_buffer[0..len];

            for input in &mut self.inputs[1..] {
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

            // Apply gain (and soft clip)
            // We need to iterate samples and map them to the correct gain frame

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
    }

    fn latency_samples(&self) -> u32 {
        self.inputs
            .iter()
            .map(|input| input.latency_samples())
            .max()
            .unwrap_or(0)
    }

    fn name(&self) -> &str {
        "SummingMixer"
    }

    fn visualize(&self, indent: usize) -> String {
        #[cfg(feature = "debug_visualize")]
        {
            let mut output = String::new();
            let spaces = " ".repeat(indent);
            let child_indent = indent + 2;

            output.push_str(&format!("{}SummingMixer\n", spaces));

            for (i, input) in self.inputs.iter().enumerate() {
                output.push_str(&format!("{}Input {}:\n", " ".repeat(child_indent), i + 1));
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
