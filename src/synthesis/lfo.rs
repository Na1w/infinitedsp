use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
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
    /// Sample and Hold (Random steps).
    SampleAndHold,
}

/// A Low Frequency Oscillator (LFO).
///
/// Generates control signals for modulation.
pub struct Lfo {
    phase: f32,
    frequency: AudioParam,
    waveform: LfoWaveform,
    min: f32,
    max: f32,
    sample_rate: f32,
    freq_buffer: Vec<f32>,
    rng_state: u32,
    last_sh_value: f32,
    sh_triggered: bool,
}

impl Lfo {
    /// Creates a new LFO.
    ///
    /// # Arguments
    /// * `frequency` - Frequency in Hz.
    /// * `waveform` - Waveform shape.
    pub fn new(frequency: AudioParam, waveform: LfoWaveform) -> Self {
        Lfo {
            phase: 0.0,
            frequency,
            waveform,
            min: -1.0,
            max: 1.0,
            sample_rate: 44100.0,
            freq_buffer: Vec::with_capacity(128),
            rng_state: 12345,
            last_sh_value: 0.0,
            sh_triggered: false,
        }
    }

    /// Sets the output range of the LFO.
    pub fn set_range(&mut self, min: f32, max: f32) {
        self.min = min;
        self.max = max;
    }

    /// Sets whether the output is unipolar (0.0 to 1.0) or bipolar (-1.0 to 1.0).
    /// This is a convenience wrapper around `set_range`.
    pub fn set_unipolar(&mut self, unipolar: bool) {
        if unipolar {
            self.set_range(0.0, 1.0);
        } else {
            self.set_range(-1.0, 1.0);
        }
    }

    fn next_random(&mut self) -> f32 {
        crate::core::utils::FastRng::next_f32_bipolar_stateless(&mut self.rng_state)
    }
}

impl FrameProcessor<Mono> for Lfo {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        if self.freq_buffer.len() != buffer.len() {
            self.freq_buffer.resize(buffer.len(), 0.0);
        }

        self.frequency.process(&mut self.freq_buffer, sample_index);

        let range = self.max - self.min;
        let offset = self.min;

        for (i, sample) in buffer.iter_mut().enumerate() {
            let freq = self.freq_buffer[i];
            let inc = freq / self.sample_rate;

            let current_phase = self.phase;
            self.phase += inc;

            if self.phase >= 1.0 {
                self.phase -= 1.0;
                self.sh_triggered = false;
            } else if self.phase < 0.0 {
                self.phase += 1.0;
            }

            let raw = match self.waveform {
                LfoWaveform::Sine => {
                    let mut t = current_phase * 2.0 - 1.0;
                    t = 2.0 * libm::fabsf(t) - 1.0;
                    t * (1.5 - 0.5 * t * t)
                }
                LfoWaveform::Triangle => {
                    let t = current_phase * 2.0 - 1.0;
                    2.0 * libm::fabsf(t) - 1.0
                }
                LfoWaveform::Saw => 2.0 * current_phase - 1.0,
                LfoWaveform::Square => {
                    if current_phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                LfoWaveform::SampleAndHold => {
                    if !self.sh_triggered {
                        self.last_sh_value = self.next_random();
                        self.sh_triggered = true;
                    }
                    self.last_sh_value
                }
            };

            let normalized = (raw + 1.0) * 0.5;
            *sample = offset + normalized * range;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.frequency.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.sh_triggered = false;
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        match self.waveform {
            LfoWaveform::Sine => "Lfo (Sine)",
            LfoWaveform::Triangle => "Lfo (Triangle)",
            LfoWaveform::Saw => "Lfo (Saw)",
            LfoWaveform::Square => "Lfo (Square)",
            LfoWaveform::SampleAndHold => "Lfo (S&H)",
        }
    }
}
