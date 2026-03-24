use crate::core::audio_param::AudioParam;
use crate::core::channels::ChannelConfig;
use crate::core::frame_processor::FrameProcessor;
use crate::effects::utility::lookahead::Lookahead;
use alloc::vec;
use alloc::vec::Vec;

/// A Limiter with lookahead.
///
/// Uses a lookahead buffer to "see" upcoming peaks and apply gain reduction
/// smoothly before the peak reaches the output, ensuring the signal never
/// exceeds the threshold.
pub struct Limiter<C: ChannelConfig> {
    threshold_db: AudioParam,
    release_ms: AudioParam,
    lookahead_samples: u32,
    
    lookahead: Lookahead<C>,
    envelope: f32,
    sample_rate: f32,
    
    threshold_buffer: Vec<f32>,
    release_buffer: Vec<f32>,
}

impl<C: ChannelConfig> Limiter<C> {
    /// Creates a new Limiter.
    ///
    /// # Arguments
    /// * `threshold_db` - The maximum output level in dB (e.g. -0.1).
    /// * `lookahead_ms` - Lookahead time in milliseconds (e.g. 5.0).
    /// * `release_ms` - Release time in milliseconds (e.g. 100.0).
    /// * `sample_rate` - Initial sample rate.
    pub fn new(threshold_db: AudioParam, lookahead_ms: f32, release_ms: AudioParam, sample_rate: f32) -> Self {
        let lookahead_samples = (lookahead_ms * sample_rate / 1000.0) as u32;
        
        Limiter {
            threshold_db,
            release_ms,
            lookahead_samples,
            lookahead: Lookahead::new(lookahead_samples),
            envelope: 0.0,
            sample_rate,
            threshold_buffer: vec![0.0; 128],
            release_buffer: vec![0.0; 128],
        }
    }

    pub fn set_threshold(&mut self, threshold: AudioParam) {
        self.threshold_db = threshold;
    }

    pub fn set_release(&mut self, release: AudioParam) {
        self.release_ms = release;
    }
}

impl<C: ChannelConfig> FrameProcessor<C> for Limiter<C> {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        let channels = C::num_channels();
        let frames = len / channels;

        if self.threshold_buffer.len() < frames {
            self.threshold_buffer.resize(frames, 0.0);
        }
        if self.release_buffer.len() < frames {
            self.release_buffer.resize(frames, 0.0);
        }

        self.threshold_db.process(&mut self.threshold_buffer[0..frames], sample_index);
        self.release_ms.process(&mut self.release_buffer[0..frames], sample_index);

        for i in 0..frames {
            let threshold_db = self.threshold_buffer[i];
            let release_ms = self.release_buffer[i];
            
            let threshold = libm::powf(10.0, threshold_db / 20.0);
            let release_coeff = libm::expf(-1.0 / (release_ms * self.sample_rate * 0.001));

            let mut frame_peak = 0.0f32;
            for c in 0..channels {
                frame_peak = frame_peak.max(buffer[i * channels + c].abs());
            }

            if frame_peak > self.envelope {
                self.envelope = frame_peak;
            } else {
                self.envelope = release_coeff * self.envelope + (1.0 - release_coeff) * frame_peak;
            }

            let mut gain = 1.0;
            if self.envelope > threshold {
                gain = threshold / self.envelope;
            }

            let frame_slice = &mut buffer[i * channels .. (i + 1) * channels];
            self.lookahead.process(frame_slice, sample_index + i as u64);
            
            for sample in frame_slice.iter_mut() {
                *sample *= gain;
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.threshold_db.set_sample_rate(sample_rate);
        self.release_ms.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.lookahead.reset();
        self.envelope = 0.0;
    }

    fn latency_samples(&self) -> u32 {
        self.lookahead_samples
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Limiter (Lookahead)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::channels::Mono;

    #[test]
    fn test_limiter_basics() {
        let threshold = AudioParam::db(-6.0); // approx 0.5
        let release = AudioParam::ms(100.0);
        let mut limiter = Limiter::<Mono>::new(threshold, 5.0, release, 44100.0);
        
        assert_eq!(limiter.latency_samples(), (5.0 * 44100.0 / 1000.0) as u32);

        // Input 2.0 (well above 0.5)
        let mut buffer = [2.0; 1000];
        limiter.process(&mut buffer, 0);

        // The first few samples will be 0.0 due to lookahead delay.
        // But eventually it should limit to approximately 0.5.
        let last_sample = buffer[999];
        assert!(last_sample <= 0.502);
        assert!(last_sample > 0.4);
    }
}
