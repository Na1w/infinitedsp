use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec::Vec;
use core::f32::consts::PI;

struct LadderCoeffs {
    g: f32,
    k: f32,
    g1: f32,
    g2: f32,
    g3: f32,
    g4: f32,
    beta: f32,
}

/// A 4-pole lowpass ladder filter using Newton-Raphson ZDF.
///
/// This implementation uses an iterative solver to handle the non-linear feedback loop,
/// providing high accuracy and stability even at high resonance.
pub struct LadderFilter {
    cutoff: AudioParam,
    resonance: AudioParam,
    sample_rate: f32,
    s: [f32; 4],

    cutoff_buffer: Vec<f32>,
    res_buffer: Vec<f32>,
}

impl LadderFilter {
    /// Creates a new LadderFilter.
    ///
    /// # Arguments
    /// * `cutoff` - Cutoff frequency in Hz.
    /// * `resonance` - Resonance (0.0 - 1.0+). Self-oscillates at high values.
    pub fn new(cutoff: AudioParam, resonance: AudioParam) -> Self {
        LadderFilter {
            cutoff,
            resonance,
            sample_rate: 44100.0,
            s: [0.0; 4],
            cutoff_buffer: Vec::new(),
            res_buffer: Vec::new(),
        }
    }

    #[inline(always)]
    fn calc_coeffs(cutoff_val: f32, res_val: f32, sample_rate: f32) -> LadderCoeffs {
        let fc = cutoff_val.clamp(10.0, sample_rate * 0.49);
        let g = libm::tanf(PI * fc / sample_rate);
        let k = res_val * 4.0;

        let g1 = g / (1.0 + g);
        let g2 = g1 * g1;
        let g3 = g2 * g1;
        let g4 = g3 * g1;

        let beta = 1.0 / (1.0 + g);

        LadderCoeffs {
            g,
            k,
            g1,
            g2,
            g3,
            g4,
            beta,
        }
    }

    #[inline(always)]
    fn step(s: &mut [f32; 4], sample: &mut f32, coeffs: &LadderCoeffs) {
        let x = *sample;
        let c = coeffs;

        let s1_term = s[0] * c.beta;
        let s2_term = s[1] * c.beta;
        let s3_term = s[2] * c.beta;
        let s4_term = s[3] * c.beta;

        let sigma = c.g3 * s1_term + c.g2 * s2_term + c.g1 * s3_term + s4_term;

        let mut y4 = s[3];

        for _ in 0..5 {
            let tanh_y4 = libm::tanhf(y4);
            let u = x - c.k * tanh_y4;

            let f_y = y4 - (c.g4 * u + sigma);
            let df_y = 1.0 + c.g4 * c.k * (1.0 - tanh_y4 * tanh_y4);

            y4 -= f_y / df_y;
        }

        let tanh_y4 = libm::tanhf(y4);
        let u = x - c.k * tanh_y4;

        let y1 = (c.g * u + s[0]) * c.beta;
        let y2 = (c.g * y1 + s[1]) * c.beta;
        let y3 = (c.g * y2 + s[2]) * c.beta;

        s[0] = 2.0 * y1 - s[0];
        s[1] = 2.0 * y2 - s[1];
        s[2] = 2.0 * y3 - s[2];
        s[3] = 2.0 * y4 - s[3];

        *sample = y4;
    }
}

impl FrameProcessor<Mono> for LadderFilter {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();

        let cutoff_is_dynamic = matches!(self.cutoff, AudioParam::Dynamic(_));
        let res_is_dynamic = matches!(self.resonance, AudioParam::Dynamic(_));

        if cutoff_is_dynamic {
            if self.cutoff_buffer.len() < len {
                self.cutoff_buffer.resize(len, 0.0);
            }
            self.cutoff
                .process(&mut self.cutoff_buffer[0..len], sample_index);
        }
        if res_is_dynamic {
            if self.res_buffer.len() < len {
                self.res_buffer.resize(len, 0.0);
            }
            self.resonance
                .process(&mut self.res_buffer[0..len], sample_index);
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
        let sample_rate = self.sample_rate;

        if !cutoff_is_dynamic && !res_is_dynamic {
            let coeffs = Self::calc_coeffs(cutoff_static, res_static, sample_rate);
            for sample in buffer.iter_mut() {
                Self::step(s, sample, &coeffs);
            }
        } else {
            for (i, sample) in buffer.iter_mut().enumerate() {
                let c = if cutoff_is_dynamic {
                    self.cutoff_buffer[i]
                } else {
                    cutoff_static
                };
                let r = if res_is_dynamic {
                    self.res_buffer[i]
                } else {
                    res_static
                };

                let coeffs = Self::calc_coeffs(c, r, sample_rate);
                Self::step(s, sample, &coeffs);
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.cutoff.set_sample_rate(sample_rate);
        self.resonance.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.s = [0.0; 4];
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "LadderFilter (Moog)"
    }
}
