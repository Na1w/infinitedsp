use crate::core::audio_param::AudioParam;
use crate::core::channels::Stereo;
use crate::FrameProcessor;
use alloc::vec::Vec;
use core::f32::consts::PI;

/// A stereo panner.
///
/// Pans a stereo signal (interleaved) between left and right channels.
/// Uses constant power panning law.
pub struct StereoPanner {
    pan: AudioParam,
    pan_buffer: Vec<f32>,
}

impl StereoPanner {
    /// Creates a new StereoPanner.
    ///
    /// # Arguments
    /// * `pan` - Pan position (-1.0 = Left, 0.0 = Center, 1.0 = Right).
    pub fn new(pan: AudioParam) -> Self {
        StereoPanner {
            pan,
            pan_buffer: Vec::new(),
        }
    }

    /// Sets the pan parameter.
    pub fn set_pan(&mut self, pan: AudioParam) {
        self.pan = pan;
    }
}

impl FrameProcessor<Stereo> for StereoPanner {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let frames = buffer.len() / 2;

        if self.pan_buffer.len() < frames {
            self.pan_buffer.resize(frames, 0.0);
        }

        self.pan
            .process(&mut self.pan_buffer[0..frames], sample_index);

        for (i, frame) in buffer.chunks_mut(2).enumerate() {
            if frame.len() < 2 {
                break;
            }

            let pan = self.pan_buffer[i].clamp(-1.0, 1.0);

            let angle = (pan + 1.0) * PI / 4.0;
            let gain_l = libm::cosf(angle);
            let gain_r = libm::sinf(angle);

            frame[0] *= gain_l;
            frame[1] *= gain_r;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.pan.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "StereoPanner"
    }
}
