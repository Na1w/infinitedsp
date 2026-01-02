use crate::core::audio_param::AudioParam;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;
use wide::f32x4;

struct DelayLine {
    buffer: Vec<f32>,
    index: usize,
}

impl DelayLine {
    fn new(size: usize) -> Self {
        DelayLine {
            buffer: vec![0.0; size],
            index: 0,
        }
    }

    fn process_comb_block(
        &mut self,
        input: &[f32],
        output: &mut [f32],
        feedback: f32,
        damp: f32,
        filter_store: &mut f32,
    ) {
        let len = self.buffer.len();
        let block_size = input.len();

        if self.index + block_size <= len {
            let (in_chunks, in_rem) = input.as_chunks::<4>();
            let (out_chunks, out_rem) = output.as_chunks_mut::<4>();

            let mut buf_ptr = self.index;
            let mut current_store = *filter_store;

            for (in_chunk, out_chunk) in in_chunks.iter().zip(out_chunks.iter_mut()) {
                let in_vals = *in_chunk;
                let mut out_vals = [0.0; 4];

                for i in 0..4 {
                    let buf_val = self.buffer[buf_ptr + i];
                    out_vals[i] = buf_val;

                    current_store = buf_val * (1.0 - damp) + current_store * damp;

                    let to_write = in_vals[i] + current_store * feedback;
                    self.buffer[buf_ptr + i] = to_write;
                }

                let out_vec = f32x4::from(*out_chunk);
                let res_vec = f32x4::from(out_vals);
                *out_chunk = (out_vec + res_vec).to_array();

                buf_ptr += 4;
            }

            *filter_store = current_store;
            self.index = (self.index + block_size) % len;

            for (in_val, out_val) in in_rem.iter().zip(out_rem.iter_mut()) {
                let buf_val = self.buffer[self.index];
                *out_val += buf_val;

                *filter_store = buf_val * (1.0 - damp) + *filter_store * damp;

                self.buffer[self.index] = *in_val + *filter_store * feedback;
                self.index = (self.index + 1) % len;
            }
        } else {
            for (in_val, out_val) in input.iter().zip(output.iter_mut()) {
                let buf_val = self.buffer[self.index];
                *out_val += buf_val;

                *filter_store = buf_val * (1.0 - damp) + *filter_store * damp;

                self.buffer[self.index] = *in_val + *filter_store * feedback;
                self.index = (self.index + 1) % len;
            }
        }
    }

    fn process_allpass_block(&mut self, buffer: &mut [f32], feedback: f32) {
        let len = self.buffer.len();
        let block_size = buffer.len();

        if self.index + block_size <= len {
            let feedback_vec = f32x4::splat(feedback);
            let (chunks, remainder) = buffer.as_chunks_mut::<4>();
            let mut buf_ptr = self.index;

            for chunk in chunks {
                let input = f32x4::from(*chunk);

                let buf_slice = &self.buffer[buf_ptr..buf_ptr + 4];
                let buf_out = f32x4::from(unsafe { *(buf_slice.as_ptr() as *const [f32; 4]) });

                let output = buf_out - input;
                let to_write = input + buf_out * feedback_vec;

                let to_write_arr = to_write.to_array();
                self.buffer[buf_ptr..buf_ptr + 4].copy_from_slice(&to_write_arr);

                *chunk = output.to_array();
                buf_ptr += 4;
            }

            self.index = (self.index + block_size) % len;

            for sample in remainder {
                let input = *sample;
                let buf_out = self.buffer[self.index];

                *sample = buf_out - input;
                self.buffer[self.index] = input + buf_out * feedback;

                self.index = (self.index + 1) % len;
            }
        } else {
            for sample in buffer.iter_mut() {
                let input = *sample;
                let buf_out = self.buffer[self.index];

                *sample = buf_out - input;
                self.buffer[self.index] = input + buf_out * feedback;

                self.index = (self.index + 1) % len;
            }
        }
    }
}

struct Comb {
    delay: DelayLine,
    feedback: f32,
    filter_store: f32,
    damp: f32,
}

impl Comb {
    fn new(size: usize, feedback: f32, damp: f32) -> Self {
        Comb {
            delay: DelayLine::new(size),
            feedback,
            filter_store: 0.0,
            damp,
        }
    }

    fn process_block(&mut self, input: &[f32], output: &mut [f32]) {
        self.delay.process_comb_block(
            input,
            output,
            self.feedback,
            self.damp,
            &mut self.filter_store,
        );
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

    fn process_block(&mut self, buffer: &mut [f32]) {
        self.delay.process_allpass_block(buffer, self.feedback);
    }
}

/// A Schroeder-style algorithmic reverb.
///
/// Uses parallel comb filters and series allpass filters to create a dense reverberation tail.
pub struct Reverb {
    combs: [Comb; 8],
    allpasses: [Allpass; 8],
    gain: AudioParam,
    sample_rate: f32,
    input_buffer: Vec<f32>,
    gain_buffer: Vec<f32>,
    seed: usize,
}

impl Reverb {
    /// Creates a new Reverb with default seed.
    ///
    /// # Arguments
    /// * `gain` - Input gain (amount of reverb).
    pub fn new(gain: AudioParam) -> Self {
        Self::new_with_seed(gain, 0)
    }

    /// Creates a new Reverb with a specific seed for randomizing filter lengths.
    ///
    /// # Arguments
    /// * `gain` - Input gain.
    /// * `seed` - Seed for filter length randomization.
    pub fn new_with_seed(gain: AudioParam, seed: usize) -> Self {
        let sample_rate = 44100.0;
        let (combs, allpasses) = Self::create_filters(sample_rate, seed);

        Reverb {
            combs,
            allpasses,
            gain,
            sample_rate,
            input_buffer: Vec::new(),
            gain_buffer: Vec::new(),
            seed,
        }
    }

    /// Sets the input gain parameter.
    pub fn set_gain(&mut self, gain: AudioParam) {
        self.gain = gain;
    }

    fn create_filters(sample_rate: f32, seed: usize) -> ([Comb; 8], [Allpass; 8]) {
        let sr_scale = sample_rate / 44100.0;
        let offset = seed * 23;

        let comb_lengths = [1674, 1782, 1915, 2034, 2133, 2236, 2335, 2425];
        let allpass_lengths = [225, 341, 441, 561, 689, 832, 971, 1083];

        let feedback = 0.8;
        let damp = 0.3;

        let combs = [
            Comb::new(
                ((comb_lengths[0] + offset) as f32 * sr_scale) as usize,
                feedback,
                damp,
            ),
            Comb::new(
                ((comb_lengths[1] + offset) as f32 * sr_scale) as usize,
                feedback,
                damp,
            ),
            Comb::new(
                ((comb_lengths[2] + offset) as f32 * sr_scale) as usize,
                feedback,
                damp,
            ),
            Comb::new(
                ((comb_lengths[3] + offset) as f32 * sr_scale) as usize,
                feedback,
                damp,
            ),
            Comb::new(
                ((comb_lengths[4] + offset) as f32 * sr_scale) as usize,
                feedback,
                damp,
            ),
            Comb::new(
                ((comb_lengths[5] + offset) as f32 * sr_scale) as usize,
                feedback,
                damp,
            ),
            Comb::new(
                ((comb_lengths[6] + offset) as f32 * sr_scale) as usize,
                feedback,
                damp,
            ),
            Comb::new(
                ((comb_lengths[7] + offset) as f32 * sr_scale) as usize,
                feedback,
                damp,
            ),
        ];

        let allpasses = [
            Allpass::new(((allpass_lengths[0] + offset) as f32 * sr_scale) as usize),
            Allpass::new(((allpass_lengths[1] + offset) as f32 * sr_scale) as usize),
            Allpass::new(((allpass_lengths[2] + offset) as f32 * sr_scale) as usize),
            Allpass::new(((allpass_lengths[3] + offset) as f32 * sr_scale) as usize),
            Allpass::new(((allpass_lengths[4] + offset) as f32 * sr_scale) as usize),
            Allpass::new(((allpass_lengths[5] + offset) as f32 * sr_scale) as usize),
            Allpass::new(((allpass_lengths[6] + offset) as f32 * sr_scale) as usize),
            Allpass::new(((allpass_lengths[7] + offset) as f32 * sr_scale) as usize),
        ];

        (combs, allpasses)
    }
}

impl FrameProcessor for Reverb {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        if self.input_buffer.len() < buffer.len() {
            self.input_buffer.resize(buffer.len(), 0.0);
        }
        if self.gain_buffer.len() < buffer.len() {
            self.gain_buffer.resize(buffer.len(), 0.0);
        }

        self.gain
            .process(&mut self.gain_buffer[0..buffer.len()], sample_index);

        let (in_chunks, in_rem) = buffer.as_chunks::<4>();
        let (tmp_chunks, tmp_rem) = self.input_buffer.as_chunks_mut::<4>();
        let (gain_chunks, gain_rem) = self.gain_buffer.as_chunks::<4>();

        for ((in_c, tmp_c), gain_c) in in_chunks
            .iter()
            .zip(tmp_chunks.iter_mut())
            .zip(gain_chunks.iter())
        {
            let v = f32x4::from(*in_c);
            let g = f32x4::from(*gain_c);
            *tmp_c = (v * g).to_array();
        }
        for ((in_s, tmp_s), gain_s) in in_rem.iter().zip(tmp_rem.iter_mut()).zip(gain_rem.iter()) {
            *tmp_s = *in_s * *gain_s;
        }

        buffer.fill(0.0);

        let slice_len = buffer.len();
        let input_slice = &self.input_buffer[0..slice_len];

        for comb in &mut self.combs {
            comb.process_block(input_slice, buffer);
        }

        for allpass in &mut self.allpasses {
            allpass.process_block(buffer);
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        if (self.sample_rate - sample_rate).abs() > 1.0 {
            self.sample_rate = sample_rate;
            self.gain.set_sample_rate(sample_rate);
            let (combs, allpasses) = Self::create_filters(sample_rate, self.seed);
            self.combs = combs;
            self.allpasses = allpasses;
        }
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Reverb (Schroeder)"
    }
}
