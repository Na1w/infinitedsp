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

    /// Creates a new Offset processor with a modulatable parameter.
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

        match &mut self.offset {
            AudioParam::Static(val) => {
                let offset_vec = f32x4::splat(*val);
                let (chunks, remainder) = buffer.as_chunks_mut::<4>();

                for chunk in chunks {
                    let vec = f32x4::from(*chunk);
                    let result = vec + offset_vec;
                    *chunk = result.to_array();
                }

                for sample in remainder {
                    *sample += *val;
                }
            }
            _ => {
                if self.offset_buffer.len() < frames {
                    self.offset_buffer.resize(frames, 0.0);
                }

                let offset_slice = &mut self.offset_buffer[0..frames];
                self.offset.process(offset_slice, sample_index);

                if channels == 1 {
                    let (in_chunks, in_rem) = buffer.as_chunks_mut::<4>();
                    let (off_chunks, off_rem) = offset_slice.as_chunks::<4>();

                    for (in_c, off_c) in in_chunks.iter_mut().zip(off_chunks.iter()) {
                        let in_v = f32x4::from(*in_c);
                        let off_v = f32x4::from(*off_c);
                        let res = in_v + off_v;
                        *in_c = res.to_array();
                    }

                    for (in_s, off_s) in in_rem.iter_mut().zip(off_rem.iter()) {
                        *in_s += *off_s;
                    }
                } else {
                    for (i, sample) in buffer.iter_mut().enumerate() {
                        let frame_idx = i / channels;
                        *sample += offset_slice[frame_idx];
                    }
                }
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
