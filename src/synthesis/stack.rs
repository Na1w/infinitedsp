use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::synthesis::oscillator::{Oscillator, Waveform};
use crate::FrameProcessor;
use alloc::vec::Vec;

/// A stack of multiple oscillators for a thicker sound.
pub struct Stack {
    pub oscillators: Vec<Oscillator>,
    pub detune: AudioParam,
    pub mix: AudioParam,
    detune_buffer: Vec<f32>,
    mix_buffer: Vec<f32>,
    temp_buffer: Vec<f32>,
}

impl Stack {
    /// Creates a new Stack of oscillators.
    ///
    /// # Arguments
    /// * `count` - Number of oscillators in the stack.
    /// * `frequency` - Base frequency in Hz.
    /// * `waveform` - Waveform shape for all oscillators.
    /// * `detune` - Detuning amount (0.0 to 1.0).
    pub fn new(
        count: usize,
        frequency: AudioParam,
        waveform: Waveform,
        detune: AudioParam,
    ) -> Self {
        let mut oscillators = Vec::with_capacity(count);
        for _ in 0..count {
            let base_f = frequency.get_constant().unwrap_or(440.0);
            oscillators.push(Oscillator::new(AudioParam::Static(base_f), waveform));
        }
        Stack {
            oscillators,
            detune,
            mix: AudioParam::Static(1.0 / count.max(1) as f32),
            detune_buffer: Vec::with_capacity(128),
            mix_buffer: Vec::with_capacity(128),
            temp_buffer: Vec::with_capacity(128),
        }
    }

    /// Sets the base frequency for all oscillators.
    pub fn set_frequency(&mut self, frequency: AudioParam) {
        let f = frequency.get_constant().unwrap_or(440.0);
        for osc in &mut self.oscillators {
            osc.set_frequency(AudioParam::Static(f));
        }
    }

    /// Align all oscillators to the same phase.
    pub fn align_phases(&mut self) {
        for osc in &mut self.oscillators {
            osc.set_phase(0.0);
        }
    }
}

impl FrameProcessor<Mono> for Stack {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.detune_buffer.len() < len {
            self.detune_buffer.resize(len, 0.0);
            self.mix_buffer.resize(len, 0.0);
            self.temp_buffer.resize(len, 0.0);
        }

        self.detune
            .process(&mut self.detune_buffer[0..len], sample_index);
        self.mix.process(&mut self.mix_buffer[0..len], sample_index);

        buffer.fill(0.0);

        let count = self.oscillators.len();
        for (i, osc) in self.oscillators.iter_mut().enumerate() {
            let spread = if count > 1 {
                (i as f32 / (count - 1) as f32) * 2.0 - 1.0
            } else {
                0.0
            };

            let base_freq = osc.get_frequency().get_constant().unwrap_or(440.0);
            let detuned_freq = base_freq * (1.0 + spread * 0.01 * self.detune_buffer[0]);
            osc.set_frequency(AudioParam::Static(detuned_freq));

            self.temp_buffer[0..len].fill(0.0);
            osc.process(&mut self.temp_buffer[0..len], sample_index);

            for (j, sample) in buffer.iter_mut().enumerate().take(len) {
                *sample += self.temp_buffer[j] * self.mix_buffer[j];
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        for osc in &mut self.oscillators {
            osc.set_sample_rate(sample_rate);
        }
    }

    fn reset(&mut self) {
        for osc in &mut self.oscillators {
            osc.reset();
        }
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Stack"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_basic() {
        let mut stack = Stack::new(
            3,
            AudioParam::hz(440.0),
            Waveform::Sine,
            AudioParam::Static(0.0),
        );
        stack.set_sample_rate(44100.0);
        let mut buffer = [0.0; 100];
        stack.process(&mut buffer, 0);

        // Should have generated some signal
        assert!((buffer[1]).abs() > 0.0);
    }

    #[test]
    fn test_stack_detune() {
        let mut stack = Stack::new(
            2,
            AudioParam::hz(440.0),
            Waveform::Sine,
            AudioParam::Static(1.0),
        );
        stack.set_sample_rate(44100.0);
        let mut buffer1 = [0.0; 100];
        stack.process(&mut buffer1, 0);

        let mut stack2 = Stack::new(
            2,
            AudioParam::hz(440.0),
            Waveform::Sine,
            AudioParam::Static(0.0),
        );
        stack2.set_sample_rate(44100.0);
        let mut buffer2 = [0.0; 100];
        stack2.process(&mut buffer2, 0);

        // Detuned signal should be different from non-detuned after some time
        // We check a bit further into the buffer
        let mut diff = 0.0;
        for i in 0..100 {
            diff += (buffer1[i] - buffer2[i]).abs();
        }
        assert!(diff > 0.001);
    }
}
