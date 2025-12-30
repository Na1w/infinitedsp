use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use core::f32::consts::PI;
use alloc::vec::Vec;

/// The waveform shape for the LFO.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LfoWaveform {
    /// Sine wave.
    Sine,
    /// Triangle wave.
    Triangle,
    /// Sawtooth wave.
    Saw,
    /// Square wave.
    Square,
    /// Random sample and hold (noise).
    SampleAndHold,
}

/// A Low Frequency Oscillator (LFO).
///
/// Generates control signals for modulation.
pub struct Lfo {
    frequency: AudioParam,
    waveform: LfoWaveform,
    unipolar: bool,

    phase: f32,
    sample_rate: f32,
    freq_buffer: Vec<f32>,

    rng_state: u32,
    last_random: f32,
}

impl Lfo {
    /// Creates a new LFO.
    ///
    /// # Arguments
    /// * `frequency` - The frequency of the LFO in Hz.
    /// * `waveform` - The shape of the waveform.
    pub fn new(frequency: AudioParam, waveform: LfoWaveform) -> Self {
        Lfo {
            frequency,
            waveform,
            unipolar: false,
            phase: 0.0,
            sample_rate: 44100.0,
            freq_buffer: Vec::new(),
            rng_state: 12345,
            last_random: 0.0,
        }
    }

    /// Sets whether the output should be unipolar (0.0 to 1.0) or bipolar (-1.0 to 1.0).
    pub fn set_unipolar(&mut self, unipolar: bool) {
        self.unipolar = unipolar;
    }

    fn next_random(rng_state: &mut u32) -> f32 {
        *rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let val = (*rng_state >> 16) & 0x7FFF;
        (val as f32 / 32768.0) * 2.0 - 1.0
    }
}

impl FrameProcessor for Lfo {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        if self.freq_buffer.len() < buffer.len() {
            self.freq_buffer.resize(buffer.len(), 0.0);
        }

        let len = buffer.len();
        let freq_slice = &mut self.freq_buffer[0..len];
        self.frequency.process(freq_slice, sample_index);

        let inv_sr = 1.0 / self.sample_rate;

        let mut phase = self.phase;
        let mut rng_state = self.rng_state;
        let mut last_random = self.last_random;
        let waveform = self.waveform;
        let unipolar = self.unipolar;

        for (i, sample) in buffer.iter_mut().enumerate() {
            let freq = freq_slice[i];
            let phase_inc = freq * inv_sr;

            phase += phase_inc;

            let mut wrapped = false;
            if phase >= 1.0 {
                phase -= 1.0;
                wrapped = true;
            } else if phase < 0.0 {
                phase += 1.0;
                wrapped = true;
            }

            let mut out = match waveform {
                // libm::sinf
                LfoWaveform::Sine => libm::sinf(phase * 2.0 * PI),
                LfoWaveform::Saw => 2.0 * phase - 1.0,
                LfoWaveform::Square => if phase < 0.5 { 1.0 } else { -1.0 },
                LfoWaveform::Triangle => {
                    let x = phase * 2.0 - 1.0;
                    2.0 * x.abs() - 1.0
                },
                LfoWaveform::SampleAndHold => {
                    if wrapped {
                        last_random = Self::next_random(&mut rng_state);
                    }
                    last_random
                }
            };

            if unipolar {
                out = out * 0.5 + 0.5;
            }

            *sample = out;
        }

        self.phase = phase;
        self.rng_state = rng_state;
        self.last_random = last_random;
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.frequency.set_sample_rate(sample_rate);
    }
}
