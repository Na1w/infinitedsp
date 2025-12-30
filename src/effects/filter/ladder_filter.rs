use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use core::f32::consts::PI;
use alloc::vec::Vec;

pub struct LadderFilter {
    cutoff: AudioParam,
    resonance: AudioParam,
    sample_rate: f32,
    s: [f32; 4],

    cutoff_buffer: Vec<f32>,
    res_buffer: Vec<f32>,
}

impl LadderFilter {
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
}

impl FrameProcessor for LadderFilter {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();

        if self.cutoff_buffer.len() < len { self.cutoff_buffer.resize(len, 0.0); }
        if self.res_buffer.len() < len { self.res_buffer.resize(len, 0.0); }

        let cutoff_slice = &mut self.cutoff_buffer[0..len];
        let res_slice = &mut self.res_buffer[0..len];

        self.cutoff.process(cutoff_slice, sample_index);
        self.resonance.process(res_slice, sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let cutoff_val = cutoff_slice[i];
            let res_val = res_slice[i];

            let fc = cutoff_val.clamp(10.0, self.sample_rate * 0.49);
            let g = libm::tanf(PI * fc / self.sample_rate);

            let k = res_val * 4.0;

            let g1 = g / (1.0 + g);
            let g2 = g1 * g1;
            let g3 = g2 * g1;
            let g4 = g3 * g1;

            let beta = 1.0 / (1.0 + g);
            let x = *sample;

            let s1_term = self.s[0] * beta;
            let s2_term = self.s[1] * beta;
            let s3_term = self.s[2] * beta;
            let s4_term = self.s[3] * beta;

            let sigma = g3 * s1_term + g2 * s2_term + g1 * s3_term + s4_term;

            let mut y4 = self.s[3];

            for _ in 0..5 {
                let tanh_y4 = libm::tanhf(y4);
                let u = x - k * tanh_y4;

                let f_y = y4 - (g4 * u + sigma);
                let df_y = 1.0 + g4 * k * (1.0 - tanh_y4 * tanh_y4);

                y4 -= f_y / df_y;
            }

            let tanh_y4 = libm::tanhf(y4);
            let u = x - k * tanh_y4;

            let y1 = (g * u + self.s[0]) * beta;
            let y2 = (g * y1 + self.s[1]) * beta;
            let y3 = (g * y2 + self.s[2]) * beta;

            self.s[0] = 2.0 * y1 - self.s[0];
            self.s[1] = 2.0 * y2 - self.s[1];
            self.s[2] = 2.0 * y3 - self.s[2];
            self.s[3] = 2.0 * y4 - self.s[3];

            *sample = y4;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.cutoff.set_sample_rate(sample_rate);
        self.resonance.set_sample_rate(sample_rate);
    }
}
