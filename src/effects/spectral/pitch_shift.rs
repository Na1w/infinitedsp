use crate::core::ola::SpectralProcessor;
use crate::core::audio_param::AudioParam;
use num_complex::{Complex32, ComplexFloat};
use alloc::vec::Vec;

/// A spectral pitch shifter using FFT.
///
/// Shifts the pitch of the input signal by a specified number of semitones.
/// Uses spectral resampling (interpolation) to avoid gaps.
pub struct FftPitchShift<const N: usize> {
    fft_buffer: [Complex32; N],
    scratch: [Complex32; N],
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
            fft_buffer: [Complex32::new(0.0, 0.0); N],
            scratch: [Complex32::new(0.0, 0.0); N],
            semitones,
            factor: 1.0,
            semitones_buffer: Vec::new(),
        }
    }

    /// Sets the pitch shift amount in semitones.
    pub fn set_semitones(&mut self, semitones: AudioParam) {
        self.semitones = semitones;
    }

    fn pitch_shift(&mut self) {
        self.scratch.fill(Complex32::new(0.0, 0.0));

        let half_n = N / 2;

        for k in 0..half_n {
            let src_k_float = k as f32 / self.factor;

            if src_k_float < (half_n as f32 - 1.0) {
                let idx_a = src_k_float as usize;
                let idx_b = idx_a + 1;
                let frac = src_k_float - idx_a as f32;

                let val_a = self.fft_buffer[idx_a];
                let val_b = self.fft_buffer[idx_b];

                let re = val_a.re * (1.0 - frac) + val_b.re * frac;
                let im = val_a.im * (1.0 - frac) + val_b.im * frac;

                let val = Complex32::new(re, im);

                self.scratch[k] = val;

                if k > 0 {
                    self.scratch[N - k] = val.conj();
                }
            }
        }
        self.fft_buffer = self.scratch;
    }
}

impl<const N: usize> SpectralProcessor for FftPitchShift<N> {
    fn process_spectral(&mut self, bins: &mut [Complex32], sample_index: u64) {
        if bins.len() != N { return; }

        if self.semitones_buffer.is_empty() {
            self.semitones_buffer.resize(1, 0.0);
        }

        self.semitones.process(&mut self.semitones_buffer[0..1], sample_index);
        let semitones_val = self.semitones_buffer[0];

        self.factor = libm::powf(2.0, semitones_val / 12.0);

        self.fft_buffer.copy_from_slice(bins);
        self.pitch_shift();
        bins.copy_from_slice(&self.fft_buffer);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "FftPitchShift"
    }
}
