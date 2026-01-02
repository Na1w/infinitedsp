use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use alloc::vec::Vec;

/// A simple Mid/Side stereo widener.
///
/// Increases or decreases the stereo width by adjusting the Side channel level.
/// Width > 1.0 increases width, < 1.0 decreases width (0.0 = mono).
pub struct StereoWidener {
    width: AudioParam,
    width_buffer: Vec<f32>,
}

impl StereoWidener {
    /// Creates a new StereoWidener.
    ///
    /// # Arguments
    /// * `width` - Stereo width factor (1.0 = normal, 0.0 = mono, >1.0 = wide).
    pub fn new(width: AudioParam) -> Self {
        StereoWidener {
            width,
            width_buffer: Vec::new(),
        }
    }
}

impl FrameProcessor for StereoWidener {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        // Buffer is interleaved stereo: [L, R, L, R, ...]
        let frames = buffer.len() / 2;
        if self.width_buffer.len() < frames {
            self.width_buffer.resize(frames, 0.0);
        }

        self.width.process(&mut self.width_buffer[0..frames], sample_index);

        for (i, frame) in buffer.chunks_mut(2).enumerate() {
            if frame.len() < 2 { break; }
            let width = self.width_buffer[i];

            let l = frame[0];
            let r = frame[1];

            let mid = (l + r) * 0.5;
            let side = (l - r) * 0.5;

            let side_boosted = side * width;

            frame[0] = mid + side_boosted;
            frame[1] = mid - side_boosted;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.width.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "StereoWidener"
    }
}
