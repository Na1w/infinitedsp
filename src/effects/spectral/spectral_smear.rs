use crate::core::audio_param::AudioParam;
use crate::core::ola::SpectralProcessor;
use crate::core::utils::FastRng;
use alloc::vec::Vec;
use core::f32::consts::PI;
use num_complex::{Complex32, ComplexFloat};

/// A spectral smear effect.
///
/// Smears audio in the frequency domain by smoothing magnitudes over time
/// and randomizing phases. This creates a lush, diffuse, and ethereal sustain.
pub struct SpectralSmear<const N: usize> {
    smear: AudioParam,
    prev_magnitudes: [f32; N],
    smear_buffer: Vec<f32>,
    rng: FastRng,
}

impl<const N: usize> SpectralSmear<N> {
    /// Creates a new SpectralSmear.
    ///
    /// # Arguments
    /// * `smear` - The amount of temporal smoothing (0.0 to 1.0).
    pub fn new(smear: AudioParam) -> Self {
        SpectralSmear {
            smear,
            prev_magnitudes: [0.0; N],
            smear_buffer: Vec::with_capacity(128),
            rng: FastRng::new(12345),
        }
    }

    /// Sets the smear amount.
    pub fn set_smear(&mut self, smear: AudioParam) {
        self.smear = smear;
    }
}

impl<const N: usize> SpectralProcessor for SpectralSmear<N> {
    fn process_spectral(&mut self, bins: &mut [Complex32], sample_index: u64) {
        if bins.len() != N {
            return;
        }

        let hop_size = N / 2;
        if self.smear_buffer.len() != hop_size {
            self.smear_buffer.resize(hop_size, 0.0);
        }

        self.smear.process(&mut self.smear_buffer, sample_index);
        let s = self.smear_buffer[0].clamp(0.0, 0.999);
        let one_minus_s = 1.0 - s;

        let half_n = N / 2;
        for i in 0..=half_n {
            let (mag, _) = bins[i].to_polar();

            let smoothed_mag = s * self.prev_magnitudes[i] + one_minus_s * mag;
            self.prev_magnitudes[i] = smoothed_mag;

            let phase = self.rng.next_f32_unipolar() * 2.0 * PI;
            let new_bin = Complex32::from_polar(smoothed_mag, phase);
            bins[i] = new_bin;

            if i > 0 && i < half_n {
                bins[N - i] = new_bin.conj();
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.smear.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.prev_magnitudes.fill(0.0);
        self.smear.reset();
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "SpectralSmear"
    }
}
