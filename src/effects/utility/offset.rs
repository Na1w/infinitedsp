use crate::core::audio_param::AudioParam;
use crate::core::channels::ChannelConfig;
use crate::FrameProcessor;
use alloc::vec::Vec;
use wide::f32x4;

/// Adds a DC offset to the signal.
pub struct Offset {
    offset: AudioParam,
    offset_buffer: Vec<f32>,
}

impl Offset {
    /// Creates a new Offset processor with a fixed value.
    pub fn new(offset: f32) -> Self {
        Offset {
            offset: AudioParam::Static(offset),
            offset_buffer: Vec::new(),
        }
    }

    /// Creates a new Offset processor with a parameter.
    pub fn new_param(offset: AudioParam) -> Self {
        Offset {
            offset,
            offset_buffer: Vec::new(),
        }
    }
}

impl<C: ChannelConfig> FrameProcessor<C> for Offset {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let channels = C::num_channels();
        let frames = buffer.len() / channels;

        if self.offset_buffer.len() < frames {
            self.offset_buffer.resize(frames, 0.0);
        }

        self.offset
            .process(&mut self.offset_buffer[0..frames], sample_index);

        if let Some(constant_offset) = self.offset.get_constant() {
            let offset_vec = f32x4::splat(constant_offset);
            let (chunks, remainder) = buffer.as_chunks_mut::<4>();

            for chunk in chunks {
                let input = f32x4::from(*chunk);
                *chunk = (input + offset_vec).to_array();
            }

            for sample in remainder {
                *sample += constant_offset;
            }
        } else if channels == 1 {
            let (chunks, remainder) = buffer.as_chunks_mut::<4>();
            let (offset_chunks, offset_rem) = self.offset_buffer[0..frames].as_chunks::<4>();

            for (chunk, offset_chunk) in chunks.iter_mut().zip(offset_chunks) {
                let input = f32x4::from(*chunk);
                let offset = f32x4::from(*offset_chunk);
                *chunk = (input + offset).to_array();
            }

            for (sample, offset) in remainder.iter_mut().zip(offset_rem) {
                *sample += offset;
            }
        } else {
            for (i, sample) in buffer.iter_mut().enumerate() {
                let frame_idx = i / channels;
                *sample += self.offset_buffer[frame_idx];
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.offset.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Offset"
    }
}
