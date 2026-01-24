use crate::core::audio_param::AudioParam;
use crate::core::channels::Stereo;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;
use wide::f32x4;

const I16_SCALE: f32 = 32767.0;
const I16_SCALE_INV: f32 = 1.0 / 32767.0;

/// 4 parallel Comb filters using i16 storage and 2x downsampling.
struct Comb4LowMem {
    buffers: [Vec<i16>; 4],
    pos: [usize; 4],

    feedback: f32x4,
    damp: f32x4,
    damp_inv: f32x4,
    filter_state: f32x4,
}

impl Comb4LowMem {
    fn new(sizes: [usize; 4], feedback: f32, damp: f32) -> Self {
        assert!(
            !sizes.iter().any(|&s| s < 1),
            "Comb4LowMem: All filters must be at least 1 unit long."
        );

        let sizes_downsampled = [sizes[0] / 2, sizes[1] / 2, sizes[2] / 2, sizes[3] / 2];

        Comb4LowMem {
            buffers: [
                vec![0; sizes_downsampled[0]],
                vec![0; sizes_downsampled[1]],
                vec![0; sizes_downsampled[2]],
                vec![0; sizes_downsampled[3]],
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

    fn process_downsampled(&mut self, input: f32x4) -> f32x4 {
        let d0 = self.buffers[0][self.pos[0]] as f32 * I16_SCALE_INV;
        let d1 = self.buffers[1][self.pos[1]] as f32 * I16_SCALE_INV;
        let d2 = self.buffers[2][self.pos[2]] as f32 * I16_SCALE_INV;
        let d3 = self.buffers[3][self.pos[3]] as f32 * I16_SCALE_INV;
        let delayed = f32x4::new([d0, d1, d2, d3]);

        let new_input = input + self.filter_state * self.feedback;
        self.filter_state = delayed * self.damp_inv + self.filter_state * self.damp;

        let to_write = new_input.to_array();

        self.buffers[0][self.pos[0]] = (to_write[0].clamp(-1.0, 1.0) * I16_SCALE) as i16;
        self.buffers[1][self.pos[1]] = (to_write[1].clamp(-1.0, 1.0) * I16_SCALE) as i16;
        self.buffers[2][self.pos[2]] = (to_write[2].clamp(-1.0, 1.0) * I16_SCALE) as i16;
        self.buffers[3][self.pos[3]] = (to_write[3].clamp(-1.0, 1.0) * I16_SCALE) as i16;

        for i in 0..4 {
            self.pos[i] += 1;
            if self.pos[i] >= self.buffers[i].len() {
                self.pos[i] = 0;
            }
        }

        delayed
    }

    fn reset(&mut self) {
        for buf in &mut self.buffers {
            buf.fill(0);
        }
        self.pos = [0; 4];
        self.filter_state = f32x4::ZERO;
    }
}

struct AllpassLowMem {
    buffer: Vec<i16>,
    pos: usize,
    feedback: f32,
}

impl AllpassLowMem {
    fn new(size: usize) -> Self {
        assert!(size > 0, "AllpassLowMem: Length must be at least one unit.");
        AllpassLowMem {
            buffer: vec![0; size / 2],
            pos: 0,
            feedback: 0.5,
        }
    }

    fn process_downsampled(&mut self, input: f32) -> f32 {
        let len = self.buffer.len();
        let delayed = self.buffer[self.pos] as f32 * I16_SCALE_INV;

        let output = -input + delayed;
        let to_store = input + output * self.feedback;

        self.buffer[self.pos] = (to_store.clamp(-1.0, 1.0) * I16_SCALE) as i16;

        self.pos += 1;
        if self.pos >= len {
            self.pos = 0;
        }
        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0);
        self.pos = 0;
    }
}

pub struct ReverbLowMem {
    combs_l: [Comb4LowMem; 2],
    combs_r: [Comb4LowMem; 2],
    allpasses_l: Vec<AllpassLowMem>,
    allpasses_r: Vec<AllpassLowMem>,
    room_size: AudioParam,
    damping: AudioParam,
    sample_rate: f32,

    phase: usize,
    downsample_acc_l: f32,
    downsample_acc_r: f32,

    last_out_l: f32,
    last_out_r: f32,
}

impl ReverbLowMem {
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

        let combs_l = [
            Comb4LowMem::new(c1_l, 0.8, 0.2),
            Comb4LowMem::new(c2_l, 0.8, 0.2),
        ];
        let combs_r = [
            Comb4LowMem::new(c1_r, 0.8, 0.2),
            Comb4LowMem::new(c2_r, 0.8, 0.2),
        ];

        let mut allpasses_l = Vec::new();
        let mut allpasses_r = Vec::new();

        for t in allpass_tuning {
            allpasses_l.push(AllpassLowMem::new(t + seed));
            allpasses_r.push(AllpassLowMem::new(t + stereo_spread + seed));
        }

        ReverbLowMem {
            combs_l,
            combs_r,
            allpasses_l,
            allpasses_r,
            room_size,
            damping,
            sample_rate: 44100.0,
            phase: 0,
            downsample_acc_l: 0.0,
            downsample_acc_r: 0.0,
            last_out_l: 0.0,
            last_out_r: 0.0,
        }
    }

    pub fn set_room_size(&mut self, room_size: AudioParam) {
        self.room_size = room_size;
    }

    pub fn set_damping(&mut self, damping: AudioParam) {
        self.damping = damping;
    }
}

impl FrameProcessor<Stereo> for ReverbLowMem {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let mut param_scratch = [0.0; 1];

        self.room_size.process(&mut param_scratch, sample_index);
        let raw_rs = param_scratch[0] * 0.28 + 0.7;
        let rs = (raw_rs * 1.02).min(0.995);

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
            let input_l = frame[0] * 0.015;
            let input_r = frame[1] * 0.015;
            let input_mix = (input_l + input_r) * 0.5;

            if self.phase == 0 {
                self.downsample_acc_l = input_mix;
                self.downsample_acc_r = input_mix;
                self.phase = 1;

                frame[0] = self.last_out_l;
                frame[1] = self.last_out_r;
            } else {
                let in_down = (self.downsample_acc_l + input_mix) * 0.5;
                let in_vec = f32x4::splat(in_down);

                let mut out_l_vec = self.combs_l[0].process_downsampled(in_vec);
                out_l_vec += self.combs_l[1].process_downsampled(in_vec);
                let mut out_l = out_l_vec.reduce_add();

                let mut out_r_vec = self.combs_r[0].process_downsampled(in_vec);
                out_r_vec += self.combs_r[1].process_downsampled(in_vec);
                let mut out_r = out_r_vec.reduce_add();

                for ap in &mut self.allpasses_l {
                    out_l = ap.process_downsampled(out_l);
                }

                for ap in &mut self.allpasses_r {
                    out_r = ap.process_downsampled(out_r);
                }

                let current_out_l = out_l;
                let current_out_r = out_r;

                frame[0] = (self.last_out_l + current_out_l) * 0.5;
                frame[1] = (self.last_out_r + current_out_r) * 0.5;

                self.last_out_l = current_out_l;
                self.last_out_r = current_out_r;

                self.phase = 0;
            }
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
        self.phase = 0;
        self.downsample_acc_l = 0.0;
        self.downsample_acc_r = 0.0;
        self.last_out_l = 0.0;
        self.last_out_r = 0.0;
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Reverb (Low Mem)"
    }
}

impl Default for ReverbLowMem {
    fn default() -> Self {
        Self::new()
    }
}
