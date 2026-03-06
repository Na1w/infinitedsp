use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec::Vec;
use core::f32::consts::PI;
use wide::f32x4;

/// The waveform shape for the oscillator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    /// Sine wave.
    Sine,
    /// Triangle wave.
    Triangle,
    /// Sawtooth wave.
    Saw,
    /// Sawtooth wave (Non bandwidth limited).
    NaiveSaw,
    /// Square wave.
    Square,
    /// White noise.
    WhiteNoise,
}

/// A band-limited oscillator.
///
/// Generates standard waveforms using PolyBLEP for anti-aliasing.
pub struct Oscillator {
    pub phase: f32,
    pub frequency: AudioParam,
    pub waveform: Waveform,
    pub sample_rate: f32,
    freq_buffer: Vec<f32>,
    pub rng_state: u32,
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

    #[inline(always)]
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

    #[inline(always)]
    fn next_random(rng_state: &mut u32) -> f32 {
        *rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let val = (*rng_state >> 16) & 0x7FFF;
        (val as f32 / 32768.0) * 2.0 - 1.0
    }

    /// Sets the frequency of the oscillator.
    pub fn set_frequency(&mut self, frequency: AudioParam) {
        self.frequency = frequency;
    }

    /// Gets the current frequency of the oscillator.
    pub fn get_frequency(&self) -> &AudioParam {
        &self.frequency
    }

    /// Sets the phase of the oscillator (0.0 to 1.0).
    pub fn set_phase(&mut self, phase: f32) {
        self.phase = phase;
    }

    /// Gets the current phase of the oscillator.
    pub fn get_phase(&self) -> f32 {
        self.phase
    }

    /// Processes a single sample from the oscillator.
    #[inline(always)]
    pub fn tick(&mut self, freq_hz: f32) -> f32 {
        let inv_sr = 1.0 / self.sample_rate;
        let inc = freq_hz * inv_sr;

        if self.waveform != Waveform::WhiteNoise {
            self.phase += inc;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            } else if self.phase < 0.0 {
                self.phase += 1.0;
            }
        }

        match self.waveform {
            Waveform::Sine => libm::sinf(self.phase * 2.0 * PI),
            Waveform::Triangle => {
                if self.phase < 0.5 {
                    4.0 * self.phase - 1.0
                } else {
                    4.0 * (1.0 - self.phase) - 1.0
                }
            }
            Waveform::Saw => {
                let naive = 2.0 * self.phase - 1.0;
                naive - Self::poly_blep(self.phase, inc.abs())
            }
            Waveform::NaiveSaw => 2.0 * self.phase - 1.0,
            Waveform::Square => {
                let naive = if self.phase < 0.5 { 1.0 } else { -1.0 };
                let dt = inc.abs();
                let core =
                    Self::poly_blep(self.phase, dt) - Self::poly_blep((self.phase + 0.5) % 1.0, dt);
                naive + core
            }
            Waveform::WhiteNoise => Self::next_random(&mut self.rng_state),
        }
    }
}

impl FrameProcessor<Mono> for Oscillator {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        if self.freq_buffer.len() != buffer.len() {
            self.freq_buffer.resize(buffer.len(), 0.0);
        }

        self.frequency.process(&mut self.freq_buffer, sample_index);

        let sample_rate = self.sample_rate;
        let mut phase = self.phase;
        let inv_sr = 1.0 / sample_rate;
        let inv_sr_vec = f32x4::splat(inv_sr);

        let (chunks, remainder) = buffer.as_chunks_mut::<4>();
        let (freq_chunks, _freq_rem) = self.freq_buffer.as_chunks::<4>();

        match self.waveform {
            Waveform::Sine => {
                for (out_chunk, freq_chunk) in chunks.iter_mut().zip(freq_chunks.iter()) {
                    for i in 0..4 {
                        let freq = freq_chunk[i];
                        let inc = freq * inv_sr;
                        phase += inc;
                        if phase >= 1.0 {
                            phase -= 1.0;
                        } else if phase < 0.0 {
                            phase += 1.0;
                        }
                        out_chunk[i] = libm::sinf(phase * 2.0 * PI);
                    }
                }
            }
            Waveform::Triangle => {
                for (out_chunk, freq_chunk) in chunks.iter_mut().zip(freq_chunks.iter()) {
                    let freq = f32x4::from(*freq_chunk);
                    let inc = freq * inv_sr_vec;
                    let inc_arr = inc.to_array();
                    for i in 0..4 {
                        phase += inc_arr[i];
                        if phase >= 1.0 {
                            phase -= 1.0;
                        } else if phase < 0.0 {
                            phase += 1.0;
                        }
                        let x = phase;
                        out_chunk[i] = if x < 0.5 {
                            4.0 * x - 1.0
                        } else {
                            4.0 * (1.0 - x) - 1.0
                        };
                    }
                }
            }
            Waveform::Saw => {
                for (out_chunk, freq_chunk) in chunks.iter_mut().zip(freq_chunks.iter()) {
                    let freq = f32x4::from(*freq_chunk);
                    let inc = freq * inv_sr_vec;
                    let inc_arr = inc.to_array();
                    for i in 0..4 {
                        phase += inc_arr[i];
                        if phase >= 1.0 {
                            phase -= 1.0;
                        } else if phase < 0.0 {
                            phase += 1.0;
                        }
                        let naive = 2.0 * phase - 1.0;
                        out_chunk[i] = naive - Self::poly_blep(phase, inc_arr[i].abs());
                    }
                }
            }
            Waveform::NaiveSaw => {
                for (out_chunk, freq_chunk) in chunks.iter_mut().zip(freq_chunks.iter()) {
                    let freq = f32x4::from(*freq_chunk);
                    let inc = freq * inv_sr_vec;
                    let inc_arr = inc.to_array();
                    for i in 0..4 {
                        phase += inc_arr[i];
                        if phase >= 1.0 {
                            phase -= 1.0;
                        } else if phase < 0.0 {
                            phase += 1.0;
                        }
                        out_chunk[i] = 2.0 * phase - 1.0;
                    }
                }
            }
            Waveform::Square => {
                for (out_chunk, freq_chunk) in chunks.iter_mut().zip(freq_chunks.iter()) {
                    let freq = f32x4::from(*freq_chunk);
                    let inc = freq * inv_sr_vec;
                    let inc_arr = inc.to_array();
                    for i in 0..4 {
                        phase += inc_arr[i];
                        if phase >= 1.0 {
                            phase -= 1.0;
                        } else if phase < 0.0 {
                            phase += 1.0;
                        }
                        let naive = if phase < 0.5 { 1.0 } else { -1.0 };
                        let abs_inc = inc_arr[i].abs();
                        let corr = Self::poly_blep(phase, abs_inc)
                            - Self::poly_blep((phase + 0.5) % 1.0, abs_inc);
                        out_chunk[i] = naive + corr;
                    }
                }
            }
            Waveform::WhiteNoise => {
                let mut rng = self.rng_state;
                for out_chunk in chunks.iter_mut() {
                    for sample in out_chunk.iter_mut() {
                        *sample = Self::next_random(&mut rng);
                    }
                }
                self.rng_state = rng;
            }
        }

        for (i, sample) in remainder.iter_mut().enumerate() {
            let freq_idx = chunks.len() * 4 + i;
            let freq = self.freq_buffer[freq_idx];
            let inc = freq * inv_sr;

            if !matches!(self.waveform, Waveform::WhiteNoise) {
                phase += inc;
                if phase >= 1.0 {
                    phase -= 1.0;
                } else if phase < 0.0 {
                    phase += 1.0;
                }
            }

            let val = match self.waveform {
                Waveform::Sine => libm::sinf(phase * 2.0 * PI),
                Waveform::Triangle => {
                    let x = phase;
                    if x < 0.5 {
                        4.0 * x - 1.0
                    } else {
                        4.0 * (1.0 - x) - 1.0
                    }
                }
                Waveform::Saw => {
                    let naive = 2.0 * phase - 1.0;
                    naive - Self::poly_blep(phase, inc.abs())
                }
                Waveform::NaiveSaw => 2.0 * phase - 1.0,
                Waveform::Square => {
                    let naive = if phase < 0.5 { 1.0 } else { -1.0 };
                    let dt = inc.abs();
                    let corr =
                        Self::poly_blep(phase, dt) - Self::poly_blep((phase + 0.5) % 1.0, dt);
                    naive + corr
                }
                Waveform::WhiteNoise => {
                    let mut rng = self.rng_state;
                    let v = Self::next_random(&mut rng);
                    self.rng_state = rng;
                    v
                }
            };
            *sample = val;
        }

        self.phase = phase;
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.frequency.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        match self.waveform {
            Waveform::Sine => "Oscillator (Sine)",
            Waveform::Triangle => "Oscillator (Triangle)",
            Waveform::Saw => "Oscillator (Saw)",
            Waveform::NaiveSaw => "Oscillator (NaiveSaw)",
            Waveform::Square => "Oscillator (Square)",
            Waveform::WhiteNoise => "Oscillator (WhiteNoise)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parameter::Parameter;

    #[test]
    fn test_oscillator_sine() {
        let param = Parameter::new(441.0);
        let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::Sine);
        let mut buffer = [0.0; 100];
        osc.process(&mut buffer, 0);

        // First sample at 44100Hz, 441Hz increment is 0.01.
        // Phase after first sample is 0.01. sin(0.01 * 2 * PI)
        assert!((buffer[0] - libm::sinf(0.01 * 2.0 * PI)).abs() < 1e-5);
    }
}
