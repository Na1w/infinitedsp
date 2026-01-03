use crate::core::channels::ChannelConfig;
use crate::FrameProcessor;

/// A gate signal generator that stays high for a specific duration.
pub struct TimedGate {
    duration_samples: u64,
    current_sample: u64,
    active: bool,
    sample_rate: f32,
    duration_seconds: f32,
}

impl TimedGate {
    /// Creates a new TimedGate.
    ///
    /// # Arguments
    /// * `duration_seconds` - Duration of the gate in seconds.
    /// * `sample_rate` - Sample rate in Hz.
    pub fn new(duration_seconds: f32, sample_rate: f32) -> Self {
        TimedGate {
            duration_samples: (duration_seconds * sample_rate) as u64,
            current_sample: 0,
            active: false,
            sample_rate,
            duration_seconds,
        }
    }

    /// Triggers the gate.
    pub fn trigger(&mut self) {
        self.current_sample = 0;
        self.active = true;
    }
}

impl<C: ChannelConfig> FrameProcessor<C> for TimedGate {
    fn process(&mut self, buffer: &mut [f32], _sample_index: u64) {
        for sample in buffer.iter_mut() {
            if self.active {
                *sample = 1.0;
                self.current_sample += 1;
                if self.current_sample >= self.duration_samples {
                    self.active = false;
                }
            } else {
                *sample = 0.0;
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.duration_samples = (self.duration_seconds * sample_rate) as u64;
    }

    fn reset(&mut self) {
        self.active = false;
        self.current_sample = 0;
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "TimedGate"
    }
}
