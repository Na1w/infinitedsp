use crate::core::audio_param::AudioParam;
use crate::core::ola::SpectralProcessor;
use alloc::vec::Vec;
use core::f32::consts::PI;
use num_complex::{Complex32, ComplexFloat};

/// A spectral pitch shifter using FFT.
///
/// Shifts the pitch of the input signal by a specified number of semitones.
/// Uses spectral resampling (interpolation) to avoid gaps.
pub struct FftPitchShift<const N: usize> {
    prev_analysis_phases: [f32; N],
    synthesis_phases: [f32; N],
    analysis_mags: [f32; N],
    analysis_freqs: [f32; N],
    synthesis_mags: [f32; N],
    synthesis_freqs: [f32; N],
    semitones: AudioParam,
    factor: f32,
    semitones_buffer: Vec<f32>,
}

impl<const N: usize> FftPitchShift<N> {
    /// Creates a new FftPitchShift.
    ///
    /// # Arguments
    /// * `semitones` - Pitch shift amount in semitones.
    pub fn new(semitones: AudioParam) -> Self {
        FftPitchShift {
            prev_analysis_phases: [0.0; N],
            synthesis_phases: [0.0; N],
            analysis_mags: [0.0; N],
            analysis_freqs: [0.0; N],
            synthesis_mags: [0.0; N],
            synthesis_freqs: [0.0; N],
            semitones,
            factor: 1.0,
            semitones_buffer: Vec::with_capacity(128),
        }
    }

    /// Sets the pitch shift amount in semitones.
    pub fn set_semitones(&mut self, semitones: AudioParam) {
        self.semitones = semitones;
    }

    fn process_phase_vocoder(&mut self, bins: &mut [Complex32]) {
        let half_n = N / 2;
        let hop_size = N / 2;
        let expect = 2.0 * PI * hop_size as f32 / N as f32;

        for (k, bin) in bins.iter().enumerate().take(half_n + 1) {
            let mag = bin.abs();
            let phase = bin.arg();

            let mut tmp = phase - self.prev_analysis_phases[k];
            self.prev_analysis_phases[k] = phase;

            tmp -= k as f32 * expect;

            let qpd = libm::floorf(tmp / (2.0 * PI) + 0.5);
            tmp -= qpd * 2.0 * PI;
            tmp /= hop_size as f32;

            let freq = k as f32 * (2.0 * PI / N as f32) + tmp;

            self.analysis_mags[k] = mag;
            self.analysis_freqs[k] = freq;
        }

        self.synthesis_mags.fill(0.0);
        self.synthesis_freqs.fill(0.0);

        for k in 0..=half_n {
            let target_float = k as f32 * self.factor;
            let target_k = (target_float + 0.5) as usize;

            if target_k <= half_n {
                let m = self.analysis_mags[k];
                let f = self.analysis_freqs[k] * self.factor;

                if m > self.synthesis_mags[target_k] {
                    self.synthesis_mags[target_k] = m;
                    self.synthesis_freqs[target_k] = f;
                }
            }
        }

        for k in 0..=half_n {
            let mag = self.synthesis_mags[k];
            let freq = self.synthesis_freqs[k];

            self.synthesis_phases[k] += freq * hop_size as f32;

            let mut p = self.synthesis_phases[k];
            let wraps = libm::floorf(p / (2.0 * PI) + 0.5);
            p -= wraps * 2.0 * PI;
            self.synthesis_phases[k] = p;

            let bin = Complex32::from_polar(mag, p);
            bins[k] = bin;

            if k > 0 && k < half_n {
                bins[N - k] = bin.conj();
            }
        }

        bins[0] = Complex32::new(bins[0].re, 0.0);
        if N.is_multiple_of(2) {
            bins[half_n] = Complex32::new(bins[half_n].re, 0.0);
        }
    }
}

impl<const N: usize> SpectralProcessor for FftPitchShift<N> {
    fn process_spectral(&mut self, bins: &mut [Complex32], sample_index: u64) {
        if bins.len() != N {
            return;
        }

        let hop_size = N / 2;

        if self.semitones_buffer.len() != hop_size {
            self.semitones_buffer.resize(hop_size, 0.0);
        }

        self.semitones
            .process(&mut self.semitones_buffer, sample_index);

        let semitones_val = self.semitones_buffer[0];

        self.factor = libm::powf(2.0, semitones_val / 12.0);

        self.process_phase_vocoder(bins);
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.semitones.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.prev_analysis_phases.fill(0.0);
        self.synthesis_phases.fill(0.0);
        self.analysis_mags.fill(0.0);
        self.analysis_freqs.fill(0.0);
        self.synthesis_mags.fill(0.0);
        self.synthesis_freqs.fill(0.0);
        self.semitones.reset();
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "FftPitchShift"
    }
}
