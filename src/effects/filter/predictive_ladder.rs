use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use alloc::vec::Vec;
use core::f32::consts::PI;
use wide::f32x4;

/// A 4-pole lowpass ladder filter using Linear Prediction ZDF.
///
/// This implementation is significantly faster than the Newton-Raphson solver used in `LadderFilter`,
/// while retaining comparable audio fidelity.
pub struct PredictiveLadderFilter {
    cutoff: AudioParam,
    resonance: AudioParam,
    sample_rate: f32,
    s: [f32; 4],
    cutoff_buffer: Vec<f32>,
    res_buffer: Vec<f32>,
}

impl PredictiveLadderFilter {
    /// Creates a new PredictiveLadderFilter.
    ///
    /// # Arguments
    /// * `cutoff` - Cutoff frequency in Hz.
    /// * `resonance` - Resonance (0.0 - 1.0+). Self-oscillates at high values.
    pub fn new(cutoff: AudioParam, resonance: AudioParam) -> Self {
        Self {
            cutoff,
            resonance,
            sample_rate: 44100.0,
            s: [0.0; 4],
            cutoff_buffer: Vec::new(),
            res_buffer: Vec::new(),
        }
    }

    #[inline(always)]
    fn calc_coeffs(c: f32, r: f32, sample_rate: f32) -> (f32, f32, f32) {
        let max_f = sample_rate * 0.49;
        let fc = c.clamp(10.0, max_f);
        let g = fast_tan(PI * fc / sample_rate);
        let k = r * 4.0;
        let beta = 1.0 / (1.0 + g);
        (g, k, beta)
    }

    #[inline(always)]
    fn calc_coeffs_simd(c: f32x4, r: f32x4, sample_rate: f32) -> ([f32; 4], [f32; 4], [f32; 4]) {
        let max_f = sample_rate * 0.49;
        let fc = c.max(f32x4::splat(10.0)).min(f32x4::splat(max_f));
        let g_vec = fast_tan_simd(fc * f32x4::splat(PI / sample_rate));
        let k_vec = r * f32x4::splat(4.0);
        let beta_vec = (f32x4::splat(1.0) + g_vec).recip();
        (g_vec.into(), k_vec.into(), beta_vec.into())
    }

    #[inline(always)]
    fn step(s: &mut [f32; 4], sample: &mut f32, g: f32, k: f32, beta: f32) {
        let x = *sample;

        let g_val = g * beta;
        let s0 = s[0] * beta;
        let s1 = s[1] * beta;
        let s2 = s[2] * beta;
        let s3 = s[3] * beta;

        let g2 = g_val * g_val;
        let gamma = g2 * g2;

        let sigma = s3 + g_val * (s2 + g_val * (s1 + g_val * s0));

        let y_est = (gamma * x + sigma) / (1.0 + k * gamma);

        let u = x - k * fast_tanh(y_est);

        let v1 = g_val * u + s0;
        let v2 = g_val * v1 + s1;
        let v3 = g_val * v2 + s2;
        let v4 = g_val * v3 + s3;

        s[0] = 2.0 * v1 - s[0];
        s[1] = 2.0 * v2 - s[1];
        s[2] = 2.0 * v3 - s[2];
        s[3] = 2.0 * v4 - s[3];

        *sample = v4;
    }
}

#[inline(always)]
fn fast_tan(x: f32) -> f32 {
    let x2 = x * x;
    x * (1.0 + 0.333333 * x2)
}

#[inline(always)]
fn fast_tan_simd(x: f32x4) -> f32x4 {
    let x2 = x * x;
    x * (f32x4::splat(1.0) + f32x4::splat(0.333333) * x2)
}

#[inline(always)]
fn fast_tanh(x: f32) -> f32 {
    let x_clamped = x.clamp(-3.0, 3.0);
    let x2_c = x_clamped * x_clamped;
    x_clamped * (27.0 + x2_c) / (27.0 + 9.0 * x2_c)
}

impl FrameProcessor for PredictiveLadderFilter {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();

        let cutoff_is_dynamic = matches!(self.cutoff, AudioParam::Dynamic(_));
        let res_is_dynamic = matches!(self.resonance, AudioParam::Dynamic(_));

        if cutoff_is_dynamic {
            if self.cutoff_buffer.len() < len { self.cutoff_buffer.resize(len, 0.0); }
            self.cutoff.process(&mut self.cutoff_buffer[0..len], sample_index);
        }
        if res_is_dynamic {
            if self.res_buffer.len() < len { self.res_buffer.resize(len, 0.0); }
            self.resonance.process(&mut self.res_buffer[0..len], sample_index);
        }

        let cutoff_static = match &self.cutoff {
            AudioParam::Static(v) => *v,
            AudioParam::Linked(p) => p.get(),
            _ => 0.0,
        };
        let res_static = match &self.resonance {
            AudioParam::Static(v) => *v,
            AudioParam::Linked(p) => p.get(),
            _ => 0.0,
        };

        let s = &mut self.s;
        let cutoff_buf = &self.cutoff_buffer;
        let res_buf = &self.res_buffer;
        let sample_rate = self.sample_rate;

        if !cutoff_is_dynamic && !res_is_dynamic {
            let (g, k, beta) = Self::calc_coeffs(cutoff_static, res_static, sample_rate);
            for sample in buffer.iter_mut() {
                Self::step(s, sample, g, k, beta);
            }
        } else {
            let (chunks, remainder) = buffer.as_chunks_mut::<4>();

            let mut i = 0;
            for chunk in chunks {
                let c_vec = if cutoff_is_dynamic {
                    let arr: [f32; 4] = cutoff_buf[i..i+4].try_into().unwrap();
                    f32x4::from(arr)
                } else {
                    f32x4::splat(cutoff_static)
                };

                let r_vec = if res_is_dynamic {
                    let arr: [f32; 4] = res_buf[i..i+4].try_into().unwrap();
                    f32x4::from(arr)
                } else {
                    f32x4::splat(res_static)
                };

                let (g_arr, k_arr, beta_arr) = Self::calc_coeffs_simd(c_vec, r_vec, sample_rate);

                for j in 0..4 {
                    Self::step(s, &mut chunk[j], g_arr[j], k_arr[j], beta_arr[j]);
                }

                i += 4;
            }

            for (j, sample) in remainder.iter_mut().enumerate() {
                let idx = i + j;
                let c = if cutoff_is_dynamic { cutoff_buf[idx] } else { cutoff_static };
                let r = if res_is_dynamic { res_buf[idx] } else { res_static };

                let (g, k, beta) = Self::calc_coeffs(c, r, sample_rate);
                Self::step(s, sample, g, k, beta);
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.cutoff.set_sample_rate(sample_rate);
        self.resonance.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "PredictiveLadderFilter (Moog)"
    }
}
