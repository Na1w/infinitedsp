use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use wide::f32x4;
use alloc::vec::Vec;

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

impl FrameProcessor for Gain {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        match &mut self.gain {
            AudioParam::Static(val) => {
                let gain_vec = f32x4::splat(*val);
                let (chunks, remainder) = buffer.as_chunks_mut::<4>();

                for chunk in chunks {
                    let vec = f32x4::from(*chunk);
                    let result = vec * gain_vec;
                    *chunk = result.to_array();
                }

                for sample in remainder {
                    *sample *= *val;
                }
            },
            _ => {
                if self.gain_buffer.len() < buffer.len() {
                    self.gain_buffer.resize(buffer.len(), 0.0);
                }

                let len = buffer.len();
                let gain_slice = &mut self.gain_buffer[0..len];
                self.gain.process(gain_slice, sample_index);

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
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.gain.set_sample_rate(sample_rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gain() {
        let mut gain = Gain::new_fixed(0.5);
        let mut buffer = [1.0, -1.0, 0.0, 0.5];
        gain.process(&mut buffer, 0);

        assert_eq!(buffer, [0.5, -0.5, 0.0, 0.25]);
    }

    #[test]
    fn test_gain_db() {
        let mut gain = Gain::new_db(-6.0);
        let mut buffer = [1.0];
        gain.process(&mut buffer, 0);

        assert!((buffer[0] - 0.501187).abs() < 0.001);
    }
}
