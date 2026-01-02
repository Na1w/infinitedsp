use crate::FrameProcessor;

/// A gate signal generator that stays high for a specific duration.
pub struct TimedGate {
    duration_samples: u64,
    sample_rate: f32,
}

impl TimedGate {
    /// Creates a new TimedGate.
    ///
    /// # Arguments
    /// * `duration_seconds` - The duration to hold the gate high (1.0).
    pub fn new(duration_seconds: f32, sample_rate: f32) -> Self {
        TimedGate {
            duration_samples: (duration_seconds * sample_rate) as u64,
            sample_rate,
        }
    }
}

impl FrameProcessor for TimedGate {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        for (i, sample) in buffer.iter_mut().enumerate() {
            let current_sample = sample_index + i as u64;
            if current_sample < self.duration_samples {
                *sample = 1.0;
            } else {
                *sample = 0.0;
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        let old_sr = self.sample_rate;
        self.sample_rate = sample_rate;
        self.duration_samples = (self.duration_samples as f32 * sample_rate / old_sr) as u64;
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "TimedGate"
    }
}
