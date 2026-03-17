use super::frame_processor::FrameProcessor;
use crate::core::audio_param::AudioParam;
use crate::core::channels::ChannelConfig;
#[cfg(feature = "debug_visualize")]
#[cfg(feature = "debug_visualize")]
use alloc::string::String;
use alloc::vec::Vec;

/// A Parallel Mixer (Dry/Wet).
///
/// Mixes the processed signal (wet) with the original signal (dry).
/// Handles latency compensation if the processor reports latency.
pub struct ParallelMixer<P, C: ChannelConfig> {
    processor: P,
    mix: AudioParam,
    dry_buffer: Vec<f32>,
    delay_line: Vec<f32>,
    write_ptr: usize,
    mix_buffer: Vec<f32>,
    _marker: core::marker::PhantomData<C>,
}

impl<P: FrameProcessor<C>, C: ChannelConfig> ParallelMixer<P, C> {
    /// Creates a new ParallelMixer.
    ///
    /// # Arguments
    /// * `mix` - Initial mix amount (0.0 = dry, 1.0 = wet).
    /// * `processor` - The processor to wrap.
    pub fn new(mix: f32, processor: P) -> Self {
        ParallelMixer {
            processor,
            mix: AudioParam::Static(mix),
            dry_buffer: Vec::with_capacity(128),
            delay_line: Vec::with_capacity(128),
            write_ptr: 0,
            mix_buffer: Vec::with_capacity(128),
            _marker: core::marker::PhantomData,
        }
    }

    /// Sets the mix parameter.
    pub fn set_mix(&mut self, mix: AudioParam) {
        self.mix = mix;
    }
}

impl<P: FrameProcessor<C>, C: ChannelConfig> FrameProcessor<C> for ParallelMixer<P, C> {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let latency = self.processor.latency_samples() as usize;
        let channels = C::num_channels();

        if latency > 0 {
            let needed = (latency + 4096) * channels;
            if self.delay_line.len() < needed {
                self.delay_line.resize(needed, 0.0);
            }
        }

        if self.dry_buffer.len() != buffer.len() {
            self.dry_buffer.resize(buffer.len(), 0.0);
        }
        self.dry_buffer.copy_from_slice(buffer);

        let frames = buffer.len() / channels;
        if self.mix_buffer.len() < frames {
            self.mix_buffer.resize(frames, 0.0);
        }
        self.mix
            .process(&mut self.mix_buffer[0..frames], sample_index);

        if latency > 0 {
            let len = self.delay_line.len();
            for &sample in buffer.iter() {
                self.delay_line[self.write_ptr] = sample;
                self.write_ptr += 1;
                if self.write_ptr >= len {
                    self.write_ptr -= len;
                }
            }
        }

        self.processor.process(buffer, sample_index);

        if latency > 0 {
            let len = self.delay_line.len();
            let total_latency_samples = latency * channels;

            let mut start_read = self.write_ptr + len - buffer.len() - total_latency_samples;
            while start_read >= len {
                start_read -= len;
            }

            for (i, sample) in buffer.iter_mut().enumerate() {
                let mut read_idx = start_read + i;
                while read_idx >= len {
                    read_idx -= len;
                }
                let dry = self.delay_line[read_idx];

                let frame_idx = i / channels;
                let wet_gain = self.mix_buffer[frame_idx];
                let dry_gain = 1.0 - wet_gain;

                *sample = dry * dry_gain + *sample * wet_gain;
            }
        } else {
            for (i, sample) in buffer.iter_mut().enumerate() {
                let dry = self.dry_buffer[i];
                let frame_idx = i / channels;
                let wet_gain = self.mix_buffer[frame_idx];
                let dry_gain = 1.0 - wet_gain;

                *sample = dry * dry_gain + *sample * wet_gain;
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.processor.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.processor.reset();
        self.delay_line.fill(0.0);
        self.write_ptr = 0;
        self.mix.reset();
    }

    fn latency_samples(&self) -> u32 {
        self.processor.latency_samples()
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "ParallelMixer"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> String {
        use core::fmt::Write;
        let spaces = " ".repeat(indent);
        let mut output = String::new();

        writeln!(output, "{}ParallelMixer", spaces).unwrap();
        writeln!(output, "{}  |-- Input Signal (Passthrough)", spaces).unwrap();
        writeln!(output, "{}  |-- Processed Signal", spaces).unwrap();
        writeln!(output, "{}  |    |", spaces).unwrap();
        writeln!(output, "{}  |    v", spaces).unwrap();

        let inner_viz = self.processor.visualize(0);

        for line in inner_viz.lines() {
            writeln!(output, "{}  |    {}", spaces, line).unwrap();
        }

        writeln!(output, "{}  |    |", spaces).unwrap();
        writeln!(output, "{}  |    v", spaces).unwrap();
        writeln!(output, "{}  |-- Sum", spaces).unwrap();

        output
    }
}
