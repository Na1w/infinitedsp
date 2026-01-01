use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use core::f32::consts::PI;
use alloc::vec::Vec;

/// The waveform shape for the oscillator.
#[derive(Clone, Copy)]
pub enum Waveform {
    /// Sine wave.
    Sine,
    /// Triangle wave.
    Triangle,
    /// Sawtooth wave.
    Saw,
    /// Square wave.
    Square,
    /// White noise.
    WhiteNoise,
}

/// A band-limited oscillator.
///
/// Generates standard waveforms using PolyBLEP for anti-aliasing.
pub struct Oscillator {
    phase: f32,
    frequency: AudioParam,
    waveform: Waveform,
    sample_rate: f32,
    freq_buffer: Vec<f32>,
    rng_state: u32,
}

impl Oscillator {
    /// Creates a new Oscillator.
    ///
    /// # Arguments
    /// * `frequency` - Frequency in Hz.
    /// * `waveform` - Waveform shape.
    pub fn new(frequency: AudioParam, waveform: Waveform) -> Self {
        Oscillator {
            phase: 0.0,
            frequency,
            waveform,
            sample_rate: 44100.0,
            freq_buffer: Vec::new(),
            rng_state: 12345,
        }
    }

    fn poly_blep(t: f32, dt: f32) -> f32 {
        if t < dt {
            let t = t / dt;
            return t + t - t * t - 1.0;
        } else if t > 1.0 - dt {
            let t = (t - 1.0) / dt;
            return t * t + t + t + 1.0;
        }
        0.0
    }

    fn next_random(rng_state: &mut u32) -> f32 {
        *rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let val = (*rng_state >> 16) & 0x7FFF;
        (val as f32 / 32768.0) * 2.0 - 1.0
    }
}

impl FrameProcessor for Oscillator {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        if self.freq_buffer.len() != buffer.len() {
            self.freq_buffer.resize(buffer.len(), 0.0);
        }

        self.freq_buffer.fill(0.0);

        self.frequency.process(&mut self.freq_buffer, sample_index);

        let mut rng_state = self.rng_state;

        for (i, sample) in buffer.iter_mut().enumerate() {
            let freq = self.freq_buffer[i];
            let inc = freq / self.sample_rate;

            let current_phase = self.phase;

            self.phase += inc;

            // Handle phase wrapping for both positive and negative frequencies
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            } else if self.phase < 0.0 {
                self.phase += 1.0;
            }

            let val = match self.waveform {
                Waveform::Sine => libm::sinf(current_phase * 2.0 * PI),
                Waveform::Triangle => {
                    let x = current_phase;
                    if x < 0.5 {
                        4.0 * x - 1.0
                    } else {
                        4.0 * (1.0 - x) - 1.0
                    }
                },
                Waveform::Saw => {
                    let naive = 2.0 * current_phase - 1.0;
                    naive - Self::poly_blep(current_phase, inc.abs())
                },
                Waveform::Square => {
                    let naive = if current_phase < 0.5 { 1.0 } else { -1.0 };
                    let abs_inc = inc.abs();
                    let corr = Self::poly_blep(current_phase, abs_inc) - Self::poly_blep((current_phase + 0.5) % 1.0, abs_inc);
                    naive + corr
                },
                Waveform::WhiteNoise => {
                    Self::next_random(&mut rng_state)
                }
            };

            *sample = val;
        }

        self.rng_state = rng_state;
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.frequency.set_sample_rate(sample_rate);
    }
}

#[cfg(test)]
mod tests {
    use crate::core::parameter::Parameter;
    use super::*;

    #[test]
    fn test_oscillator_sine() {
        let param = Parameter::new(441.0);
        let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::Sine);
        let mut buffer = [0.0; 100];
        osc.process(&mut buffer, 0);

        // First sample should be sin(0) = 0
        assert!((buffer[0]).abs() < 1e-5);

        // Sample 25 (at 44100Hz, 441Hz) is 1/4 cycle = PI/2
        // sin(PI/2) = 1.0
        assert!((buffer[25] - 1.0).abs() < 1e-5);
    }
}
