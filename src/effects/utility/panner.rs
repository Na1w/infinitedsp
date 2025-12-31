use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use core::f32::consts::PI;
use alloc::vec::Vec;

/// A stereo panner.
///
/// Pans a stereo signal (interleaved) or mono signal (if L=R) between left and right channels.
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

impl FrameProcessor for StereoPanner {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        // Buffer is interleaved stereo: [L, R, L, R, ...]
        // We process pairs.
        let frames = buffer.len() / 2;

        if self.pan_buffer.len() < frames {
            self.pan_buffer.resize(frames, 0.0);
        }

        // Process pan param for each frame (not sample)
        // Note: sample_index usually counts frames for stereo signals in some hosts,
        // but here we assume sample_index is the raw sample count?
        // If sample_index is raw samples, then for frame i, index is sample_index + 2*i.
        // But AudioParam expects a buffer of size 'frames'.
        // Let's assume we want one pan value per stereo frame.

        self.pan.process(&mut self.pan_buffer[0..frames], sample_index);

        for (i, frame) in buffer.chunks_mut(2).enumerate() {
            if frame.len() < 2 { break; }

            let pan = self.pan_buffer[i].clamp(-1.0, 1.0);

            // Constant power panning
            // pan = -1 -> angle = 0
            // pan = 0 -> angle = PI/4
            // pan = 1 -> angle = PI/2

            let angle = (pan + 1.0) * PI / 4.0;
            let gain_l = libm::cosf(angle);
            let gain_r = libm::sinf(angle);

            let l = frame[0];
            let r = frame[1];

            // If input is mono (L=R), this pans it.
            // If input is stereo, this balances it.
            // Standard balance control:
            // L_out = L_in * gain_l
            // R_out = R_in * gain_r
            // But wait, for true panning of mono source in stereo buffer:
            // If we assume input is mono duplicated to L and R:
            // L_out = L * gain_l
            // R_out = R * gain_r

            frame[0] = l * gain_l;
            frame[1] = r * gain_r;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.pan.set_sample_rate(sample_rate);
    }
}
