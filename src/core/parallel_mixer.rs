use super::frame_processor::FrameProcessor;
use crate::core::audio_param::AudioParam;
use wide::f32x4;
use alloc::vec::Vec;
#[cfg(feature = "debug_visualize")]
use alloc::string::String;
#[cfg(feature = "debug_visualize")]
use alloc::format;

/// A Parallel Mixer (Dry/Wet).
///
/// Mixes the processed signal (wet) with the original signal (dry).
/// Handles latency compensation if the processor reports latency.
pub struct ParallelMixer<P> {
    processor: P,
    mix: AudioParam,
    dry_buffer: Vec<f32>,
    delay_line: Vec<f32>,
    write_ptr: usize,
    mix_buffer: Vec<f32>,
}

impl<P: FrameProcessor> ParallelMixer<P> {
    /// Creates a new ParallelMixer.
    ///
    /// # Arguments
    /// * `mix` - Initial mix amount (0.0 = dry, 1.0 = wet).
    /// * `processor` - The processor to wrap.
    pub fn new(mix: f32, processor: P) -> Self {
        ParallelMixer {
            processor,
            mix: AudioParam::Static(mix),
            dry_buffer: Vec::new(),
            delay_line: Vec::new(),
            write_ptr: 0,
            mix_buffer: Vec::new(),
        }
    }

    /// Sets the mix parameter.
    pub fn set_mix(&mut self, mix: AudioParam) {
        self.mix = mix;
    }
}

impl<P: FrameProcessor> FrameProcessor for ParallelMixer<P> {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let latency = self.processor.latency_samples() as usize;

        if latency > 0 {
            let needed = latency + 4096;
            if self.delay_line.len() < needed {
                self.delay_line.resize(needed, 0.0);
            }
        }

        if self.dry_buffer.len() != buffer.len() {
            self.dry_buffer.resize(buffer.len(), 0.0);
        }
        self.dry_buffer.copy_from_slice(buffer);

        if self.mix_buffer.len() != buffer.len() {
            self.mix_buffer.resize(buffer.len(), 0.0);
        }
        self.mix.process(&mut self.mix_buffer, sample_index);

        if latency > 0 {
            let len = self.delay_line.len();
            for &sample in buffer.iter() {
                self.delay_line[self.write_ptr] = sample;
                self.write_ptr = (self.write_ptr + 1) % len;
            }
        }

        self.processor.process(buffer, sample_index);

        if latency > 0 {
            let len = self.delay_line.len();
            let start_read = (self.write_ptr + len - buffer.len() - latency) % len;

            for (i, sample) in buffer.iter_mut().enumerate() {
                let read_idx = (start_read + i) % len;
                let dry = self.delay_line[read_idx];
                let wet_gain = self.mix_buffer[i];
                let dry_gain = 1.0 - wet_gain;
                *sample = dry * dry_gain + *sample * wet_gain;
            }
        } else {
            let (dry_chunks, dry_rem) = self.dry_buffer.as_chunks::<4>();
            let (wet_chunks, wet_rem) = buffer.as_chunks_mut::<4>();
            let (mix_chunks, mix_rem) = self.mix_buffer.as_chunks::<4>();

            let one_vec = f32x4::splat(1.0);

            for ((dry_chunk, wet_chunk), mix_chunk) in dry_chunks.iter().zip(wet_chunks.iter_mut()).zip(mix_chunks.iter()) {
                let d = f32x4::from(*dry_chunk);
                let w = f32x4::from(*wet_chunk);
                let wet_gain = f32x4::from(*mix_chunk);
                let dry_gain = one_vec - wet_gain;

                let res = d * dry_gain + w * wet_gain;
                *wet_chunk = res.to_array();
            }

            for ((dry, wet), &wet_gain) in dry_rem.iter().zip(wet_rem.iter_mut()).zip(mix_rem.iter()) {
                let dry_gain = 1.0 - wet_gain;
                *wet = *dry * dry_gain + *wet * wet_gain;
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.processor.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);
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
        let spaces = " ".repeat(indent);
        let mut output = String::new();

        output.push_str(&format!("{}ParallelMixer\n", spaces));
        output.push_str(&format!("{}  |-- Input Signal (Passthrough)\n", spaces));
        output.push_str(&format!("{}  |-- Processed Signal\n", spaces));
        output.push_str(&format!("{}  |    |\n", spaces));
        output.push_str(&format!("{}  |    v\n", spaces));

        let inner_viz = self.processor.visualize(0);

        for line in inner_viz.lines() {
            output.push_str(&format!("{}  |    {}\n", spaces, line));
        }

        output.push_str(&format!("{}  |    |\n", spaces));
        output.push_str(&format!("{}  |    v\n", spaces));
        output.push_str(&format!("{}  |-- Sum\n", spaces));

        output
    }
}
