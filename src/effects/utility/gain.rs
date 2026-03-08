use crate::core::audio_param::AudioParam;
use crate::core::channels::ChannelConfig;
use crate::FrameProcessor;
use alloc::vec::Vec;
use wide::f32x4;

/// A simple gain processor.
///
/// Multiplies the signal by a gain factor.
pub struct Gain {
    gain: AudioParam,
    gain_buffer: Vec<f32>,
}

impl Gain {
    /// Creates a new Gain processor.
    ///
    /// # Arguments
    /// * `gain` - The gain factor (linear).
    pub fn new(gain: AudioParam) -> Self {
        Gain {
            gain,
            gain_buffer: Vec::new(),
        }
    }

    /// Creates a new Gain processor with a fixed linear gain.
    pub fn new_fixed(gain: f32) -> Self {
        Gain {
            gain: AudioParam::Static(gain),
            gain_buffer: Vec::new(),
        }
    }

    /// Creates a new Gain processor from a decibel value.
    pub fn new_db(db: f32) -> Self {
        // libm::powf
        let val = libm::powf(10.0, db / 20.0);
        Gain {
            gain: AudioParam::Static(val),
            gain_buffer: Vec::new(),
        }
    }
}

impl<C: ChannelConfig> FrameProcessor<C> for Gain {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let channels = C::num_channels();
        let frames = buffer.len() / channels;

        if self.gain_buffer.len() < frames {
            self.gain_buffer.resize(frames, 0.0);
        }

        let gain_slice = &mut self.gain_buffer[0..frames];
        self.gain.process(gain_slice, sample_index);

        if channels == 1 {
            let (in_chunks, in_rem) = buffer.as_chunks_mut::<4>();
            let (gain_chunks, gain_rem) = gain_slice.as_chunks::<4>();

            for (in_c, gain_c) in in_chunks.iter_mut().zip(gain_chunks.iter()) {
                let in_v = f32x4::from(*in_c);
                let gain_v = f32x4::from(*gain_c);
                let res = in_v * gain_v;
                *in_c = res.to_array();
            }

            for (in_s, gain_s) in in_rem.iter_mut().zip(gain_rem.iter()) {
                *in_s *= *gain_s;
            }
        } else {
            for (i, sample) in buffer.iter_mut().enumerate() {
                let frame_idx = i / channels;
                *sample *= gain_slice[frame_idx];
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.gain.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Gain"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::channels::Mono;

    #[test]
    fn test_gain() {
        let mut gain = Gain::new_fixed(0.5);
        let mut buffer = [1.0, -1.0, 0.0, 0.5];
        FrameProcessor::<Mono>::process(&mut gain, &mut buffer, 0);

        assert_eq!(buffer, [0.5, -0.5, 0.0, 0.25]);
    }

    #[test]
    fn test_gain_db() {
        let mut gain = Gain::new_db(-6.0);
        let mut buffer = [1.0];
        FrameProcessor::<Mono>::process(&mut gain, &mut buffer, 0);

        assert!((buffer[0] - 0.501187).abs() < 0.001);
    }
}
