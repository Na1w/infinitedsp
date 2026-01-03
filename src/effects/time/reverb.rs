use crate::core::audio_param::AudioParam;
use crate::core::channels::Stereo;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;

struct DelayLine {
    buffer: Vec<f32>,
    pos: usize,
}

impl DelayLine {
    fn new(size: usize) -> Self {
        DelayLine {
            buffer: vec![0.0; size],
            pos: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.pos];
        self.buffer[self.pos] = input;
        self.pos = (self.pos + 1) % self.buffer.len();
        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.pos = 0;
    }
}

struct Comb {
    delay: DelayLine,
    feedback: f32,
    damp: f32,
    filter_state: f32,
}

impl Comb {
    fn new(size: usize, feedback: f32, damp: f32) -> Self {
        Comb {
            delay: DelayLine::new(size),
            feedback,
            damp,
            filter_state: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.delay.process(input + self.filter_state * self.feedback);
        self.filter_state = output * (1.0 - self.damp) + self.filter_state * self.damp;
        output
    }

    fn reset(&mut self) {
        self.delay.reset();
        self.filter_state = 0.0;
    }
}

struct Allpass {
    delay: DelayLine,
    feedback: f32,
}

impl Allpass {
    fn new(size: usize) -> Self {
        Allpass {
            delay: DelayLine::new(size),
            feedback: 0.5,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.delay.process(input);
        let output = -input + delayed;

        let buf_len = self.delay.buffer.len();
        let write_pos = (self.delay.pos + buf_len - 1) % buf_len;

        self.delay.buffer[write_pos] += output * self.feedback;

        output
    }

    fn reset(&mut self) {
        self.delay.reset();
    }
}

/// A Schroeder-style algorithmic reverb.
///
/// Uses parallel comb filters and series allpass filters to create a dense reverberation tail.
/// This is a Stereo effect.
///
/// Note: This processor outputs 100% Wet signal. Use `ParallelMixer` or `and_mix` to blend with dry signal.
pub struct Reverb {
    combs_l: Vec<Comb>,
    combs_r: Vec<Comb>,
    allpasses_l: Vec<Allpass>,
    allpasses_r: Vec<Allpass>,
    room_size: AudioParam,
    damping: AudioParam,
    room_size_buffer: Vec<f32>,
    damping_buffer: Vec<f32>,
    sample_rate: f32,
}

impl Reverb {
    /// Creates a new Reverb with default settings.
    pub fn new() -> Self {
        Self::new_with_params(AudioParam::Static(0.8), AudioParam::Static(0.2), 0)
    }

    /// Creates a new Reverb with a specific seed for delay lengths.
    pub fn new_with_seed(seed: usize) -> Self {
        Self::new_with_params(AudioParam::Static(0.8), AudioParam::Static(0.2), seed)
    }

    /// Creates a new Reverb with custom parameters.
    pub fn new_with_params(room_size: AudioParam, damping: AudioParam, seed: usize) -> Self {
        let comb_tuning = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        let allpass_tuning = [556, 441, 341, 225];
        let stereo_spread = 23;

        let mut combs_l = Vec::new();
        let mut combs_r = Vec::new();
        let mut allpasses_l = Vec::new();
        let mut allpasses_r = Vec::new();

        for t in comb_tuning {
            combs_l.push(Comb::new(t + seed, 0.8, 0.2));
            combs_r.push(Comb::new(t + stereo_spread + seed, 0.8, 0.2));
        }

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
            room_size_buffer: Vec::new(),
            damping_buffer: Vec::new(),
            sample_rate: 44100.0,
        }
    }

    /// Sets the room size parameter.
    pub fn set_room_size(&mut self, room_size: AudioParam) {
        self.room_size = room_size;
    }

    /// Sets the damping parameter.
    pub fn set_damping(&mut self, damping: AudioParam) {
        self.damping = damping;
    }
}

impl FrameProcessor<Stereo> for Reverb {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let frames = buffer.len() / 2;
        if self.room_size_buffer.len() < frames {
            self.room_size_buffer.resize(frames, 0.0);
        }
        if self.damping_buffer.len() < frames {
            self.damping_buffer.resize(frames, 0.0);
        }

        self.room_size
            .process(&mut self.room_size_buffer[0..frames], sample_index);
        self.damping
            .process(&mut self.damping_buffer[0..frames], sample_index);

        let rs = self.room_size_buffer[0] * 0.28 + 0.7;
        let dp = self.damping_buffer[0] * 0.4;

        for comb in &mut self.combs_l {
            comb.feedback = rs;
            comb.damp = dp;
        }
        for comb in &mut self.combs_r {
            comb.feedback = rs;
            comb.damp = dp;
        }

        for frame in buffer.chunks_mut(2) {
            let input = (frame[0] + frame[1]) * 0.5 * 0.015;

            let mut out_l = 0.0;
            let mut out_r = 0.0;

            for comb in &mut self.combs_l {
                out_l += comb.process(input);
            }
            for comb in &mut self.combs_r {
                out_r += comb.process(input);
            }

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
        for comb in &mut self.combs_l {
            comb.reset();
        }
        for comb in &mut self.combs_r {
            comb.reset();
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
