use crate::core::audio_param::AudioParam;
use crate::core::channels::Stereo;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;
use wide::f32x4;

/// 4 parallel Comb filters (SIMD friendly... hopefully).
struct Comb4 {
    buffers: [Vec<f32>; 4],
    pos: [usize; 4],

    feedback: f32x4,
    damp: f32x4,
    damp_inv: f32x4,
    filter_state: f32x4,
}

impl Comb4 {
    fn new(sizes: [usize; 4], feedback: f32, damp: f32) -> Self {
        assert!(
            !sizes.iter().any(|&s| s < 1),
            "Comb4: All filters must be at least 1 unit long."
        );
        Comb4 {
            buffers: [
                vec![0.0; sizes[0]],
                vec![0.0; sizes[1]],
                vec![0.0; sizes[2]],
                vec![0.0; sizes[3]],
            ],
            pos: [0; 4],
            feedback: f32x4::splat(feedback),
            damp: f32x4::splat(damp),
            damp_inv: f32x4::splat(1.0 - damp),
            filter_state: f32x4::ZERO,
        }
    }

    fn set_params(&mut self, feedback: f32, damp: f32, damp_inv: f32) {
        self.feedback = f32x4::splat(feedback);
        self.damp = f32x4::splat(damp);
        self.damp_inv = f32x4::splat(damp_inv);
    }

    fn process(&mut self, input: f32) -> f32 {
        let input_vec = f32x4::splat(input);

        // SAFETY: pos can never be outside the bounds.
        let delayed = unsafe {
            let d0 = *self.buffers[0].get_unchecked(self.pos[0]);
            let d1 = *self.buffers[1].get_unchecked(self.pos[1]);
            let d2 = *self.buffers[2].get_unchecked(self.pos[2]);
            let d3 = *self.buffers[3].get_unchecked(self.pos[3]);
            f32x4::new([d0, d1, d2, d3])
        };

        let new_input = input_vec + self.filter_state * self.feedback;
        self.filter_state = delayed * self.damp_inv + self.filter_state * self.damp;

        let to_write = new_input.to_array();

        // SAFETY: pos can never be outside the bounds.
        unsafe {
            *self.buffers[0].get_unchecked_mut(self.pos[0]) = to_write[0];
            *self.buffers[1].get_unchecked_mut(self.pos[1]) = to_write[1];
            *self.buffers[2].get_unchecked_mut(self.pos[2]) = to_write[2];
            *self.buffers[3].get_unchecked_mut(self.pos[3]) = to_write[3];
        };

        for i in 0..4 {
            self.pos[i] += 1;
            if self.pos[i] >= self.buffers[i].len() {
                self.pos[i] = 0;
            }
        }

        delayed.reduce_add()
    }

    fn reset(&mut self) {
        for buf in &mut self.buffers {
            buf.fill(0.0);
        }
        self.pos = [0; 4];
        self.filter_state = f32x4::ZERO;
    }
}

struct Allpass {
    buffer: Vec<f32>,
    pos: usize,
    feedback: f32,
}

impl Allpass {
    fn new(size: usize) -> Self {
        assert!(size > 0, "Allpass: Length must be at least one unit.");
        Allpass {
            buffer: vec![0.0; size],
            pos: 0,
            feedback: 0.5,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let len = self.buffer.len();
        // SAFETY: pos can never be outside the bounds.
        let delayed = unsafe { *self.buffer.get_unchecked(self.pos) };
        let output = -input + delayed;
        let to_store = input + output * self.feedback;
        // SAFETY: pos can never be outside the bounds.
        unsafe { *self.buffer.get_unchecked_mut(self.pos) = to_store };

        self.pos += 1;
        if self.pos >= len {
            self.pos = 0;
        }
        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.pos = 0;
    }
}

pub struct Reverb {
    combs_l: [Comb4; 2],
    combs_r: [Comb4; 2],
    allpasses_l: Vec<Allpass>,
    allpasses_r: Vec<Allpass>,
    room_size: AudioParam,
    damping: AudioParam,
    sample_rate: f32,
}

impl Reverb {
    pub fn new() -> Self {
        Self::new_with_params(AudioParam::Static(0.8), AudioParam::Static(0.2), 0)
    }

    pub fn new_with_seed(seed: usize) -> Self {
        Self::new_with_params(AudioParam::Static(0.8), AudioParam::Static(0.2), seed)
    }

    pub fn new_with_params(room_size: AudioParam, damping: AudioParam, seed: usize) -> Self {
        let comb_tuning = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        let allpass_tuning = [556, 441, 341, 225];
        let stereo_spread = 23;

        let c1_l = [
            comb_tuning[0] + seed,
            comb_tuning[1] + seed,
            comb_tuning[2] + seed,
            comb_tuning[3] + seed,
        ];
        let c2_l = [
            comb_tuning[4] + seed,
            comb_tuning[5] + seed,
            comb_tuning[6] + seed,
            comb_tuning[7] + seed,
        ];

        let c1_r = [
            comb_tuning[0] + stereo_spread + seed,
            comb_tuning[1] + stereo_spread + seed,
            comb_tuning[2] + stereo_spread + seed,
            comb_tuning[3] + stereo_spread + seed,
        ];
        let c2_r = [
            comb_tuning[4] + stereo_spread + seed,
            comb_tuning[5] + stereo_spread + seed,
            comb_tuning[6] + stereo_spread + seed,
            comb_tuning[7] + stereo_spread + seed,
        ];

        let combs_l = [Comb4::new(c1_l, 0.8, 0.2), Comb4::new(c2_l, 0.8, 0.2)];
        let combs_r = [Comb4::new(c1_r, 0.8, 0.2), Comb4::new(c2_r, 0.8, 0.2)];

        let mut allpasses_l = Vec::new();
        let mut allpasses_r = Vec::new();

        for t in allpass_tuning {
            allpasses_l.push(Allpass::new(t + seed));
            allpasses_r.push(Allpass::new(t + stereo_spread + seed));
        }

        Reverb {
            combs_l,
            combs_r,
            allpasses_l,
            allpasses_r,
            room_size,
            damping,
            sample_rate: 44100.0,
        }
    }

    pub fn set_room_size(&mut self, room_size: AudioParam) {
        self.room_size = room_size;
    }

    pub fn set_damping(&mut self, damping: AudioParam) {
        self.damping = damping;
    }
}

impl FrameProcessor<Stereo> for Reverb {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let mut param_scratch = [0.0; 1];

        self.room_size.process(&mut param_scratch, sample_index);
        let rs = param_scratch[0] * 0.28 + 0.7;

        self.damping.process(&mut param_scratch, sample_index);
        let dp = param_scratch[0] * 0.4;
        let dp_inv = 1.0 - dp;

        for c in &mut self.combs_l {
            c.set_params(rs, dp, dp_inv);
        }
        for c in &mut self.combs_r {
            c.set_params(rs, dp, dp_inv);
        }

        for frame in buffer.chunks_mut(2) {
            let input = (frame[0] + frame[1]) * 0.5 * 0.015;

            let mut out_l = self.combs_l[0].process(input);
            out_l += self.combs_l[1].process(input);

            let mut out_r = self.combs_r[0].process(input);
            out_r += self.combs_r[1].process(input);

            for ap in &mut self.allpasses_l {
                out_l = ap.process(out_l);
            }
            for ap in &mut self.allpasses_r {
                out_r = ap.process(out_r);
            }

            frame[0] = out_l;
            frame[1] = out_r;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.room_size.set_sample_rate(sample_rate);
        self.damping.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        for c in &mut self.combs_l {
            c.reset();
        }
        for c in &mut self.combs_r {
            c.reset();
        }
        for ap in &mut self.allpasses_l {
            ap.reset();
        }
        for ap in &mut self.allpasses_r {
            ap.reset();
        }
        self.room_size.reset();
        self.damping.reset();
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Reverb (Schroeder)"
    }
}

impl Default for Reverb {
    fn default() -> Self {
        Self::new()
    }
}
