use crate::core::audio_param::AudioParam;
use crate::core::channels::Stereo;
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
        let inv_damp = 1.0 - damp;

        if self.index + block_size <= len {
            let (in_chunks, in_rem) = input.as_chunks::<4>();
            let (out_chunks, out_rem) = output.as_chunks_mut::<4>();

            let mut buf_ptr = self.index;
            let mut current_store = *filter_store;

            for (in_chunk, out_chunk) in in_chunks.iter().zip(out_chunks.iter_mut()) {
                let in_vals = *in_chunk;
                let mut out_vals = [0.0; 4];

                let buf_slice = &mut self.buffer[buf_ptr..buf_ptr + 4];
                let mut buf_vals = [0.0; 4];
                buf_vals.copy_from_slice(buf_slice);

                for i in 0..4 {
                    let buf_val = buf_vals[i];
                    out_vals[i] = buf_val;

                    current_store = buf_val * inv_damp + current_store * damp;

                    let to_write = in_vals[i] + current_store * feedback;
                    buf_vals[i] = to_write;
                }

                buf_slice.copy_from_slice(&buf_vals);

                let out_vec = f32x4::from(*out_chunk);
                let res_vec = f32x4::from(out_vals);
                *out_chunk = (out_vec + res_vec).to_array();

                buf_ptr += 4;
            }

            *filter_store = current_store;

            self.index += block_size;
            if self.index == len {
                self.index = 0;
            }

            for (in_val, out_val) in in_rem.iter().zip(out_rem.iter_mut()) {
                let buf_val = self.buffer[self.index];
                *out_val += buf_val;

                *filter_store = buf_val * inv_damp + *filter_store * damp;

                self.buffer[self.index] = *in_val + *filter_store * feedback;
                self.index += 1;
            }
        } else {
            for (in_val, out_val) in input.iter().zip(output.iter_mut()) {
                let buf_val = self.buffer[self.index];
                *out_val += buf_val;

                *filter_store = buf_val * inv_damp + *filter_store * damp;

                self.buffer[self.index] = *in_val + *filter_store * feedback;

                self.index += 1;
                if self.index >= len {
                    self.index = 0;
                }
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

                let buf_slice = &mut self.buffer[buf_ptr..buf_ptr + 4];
                let mut buf_arr = [0.0; 4];
                buf_arr.copy_from_slice(buf_slice);
                let buf_out = f32x4::from(buf_arr);

                let new_buf = input + buf_out * feedback_vec;
                let output = buf_out - new_buf * feedback_vec;

                let to_write_arr = new_buf.to_array();
                buf_slice.copy_from_slice(&to_write_arr);

                *chunk = output.to_array();
                buf_ptr += 4;
            }

            self.index += block_size;
            if self.index == len {
                self.index = 0;
            }

            for sample in remainder {
                let input = *sample;
                let buf_out = self.buffer[self.index];

                let new_buf = input + buf_out * feedback;
                *sample = buf_out - new_buf * feedback;
                self.buffer[self.index] = new_buf;

                self.index += 1;
            }
        } else {
            for sample in buffer.iter_mut() {
                let input = *sample;
                let buf_out = self.buffer[self.index];

                let new_buf = input + buf_out * feedback;
                *sample = buf_out - new_buf * feedback;
                self.buffer[self.index] = new_buf;

                self.index += 1;
                if self.index >= len {
                    self.index = 0;
                }
            }
        }
    }
}

struct Comb {
    delay: DelayLine,
    filter_store: f32,
}

impl Comb {
    fn new(size: usize) -> Self {
        Comb {
            delay: DelayLine::new(size),
            filter_store: 0.0,
        }
    }

    fn process_block(&mut self, input: &[f32], output: &mut [f32], feedback: f32, damp: f32) {
        self.delay
            .process_comb_block(input, output, feedback, damp, &mut self.filter_store);
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
/// This is a Stereo effect.
///
/// Note: This processor outputs 100% Wet signal. Use `ParallelMixer` or `and_mix` to blend with dry signal.
pub struct Reverb {
    combs: [Comb; 8],
    allpasses: [Allpass; 8],
    room_size: AudioParam,
    damping: AudioParam,
    sample_rate: f32,
    mono_input: Vec<f32>,
    reverb_out: Vec<f32>,
    room_size_buffer: Vec<f32>,
    damping_buffer: Vec<f32>,
    seed: usize,
}

impl Reverb {
    /// Creates a new Reverb with default seed.
    pub fn new() -> Self {
        Self::new_with_seed(0)
    }

    /// Creates a new Reverb with a specific seed for randomizing filter lengths.
    ///
    /// # Arguments
    /// * `seed` - Seed for filter length randomization.
    pub fn new_with_seed(seed: usize) -> Self {
        Self::new_with_params(AudioParam::linear(0.8), AudioParam::linear(0.2), seed)
    }

    /// Creates a new Reverb with configurable parameters.
    ///
    /// # Arguments
    /// * `room_size` - Room size (feedback amount for comb filters).
    /// * `damping` - Damping amount (lowpass filter for comb filters).
    /// * `seed` - Seed for filter length randomization.
    pub fn new_with_params(room_size: AudioParam, damping: AudioParam, seed: usize) -> Self {
        let sample_rate = 44100.0;
        let (combs, allpasses) = Self::create_filters(sample_rate, seed);

        Reverb {
            combs,
            allpasses,
            room_size,
            damping,
            sample_rate,
            mono_input: Vec::new(),
            reverb_out: Vec::new(),
            room_size_buffer: Vec::new(),
            damping_buffer: Vec::new(),
            seed,
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

    fn create_filters(sample_rate: f32, seed: usize) -> ([Comb; 8], [Allpass; 8]) {
        let sr_scale = sample_rate / 44100.0;
        let offset = seed * 23;

        let comb_lengths = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        let allpass_lengths = [225, 341, 441, 561, 689, 832, 971, 1083];

        let combs = [
            Comb::new(((comb_lengths[0] + offset) as f32 * sr_scale) as usize),
            Comb::new(((comb_lengths[1] + offset) as f32 * sr_scale) as usize),
            Comb::new(((comb_lengths[2] + offset) as f32 * sr_scale) as usize),
            Comb::new(((comb_lengths[3] + offset) as f32 * sr_scale) as usize),
            Comb::new(((comb_lengths[4] + offset) as f32 * sr_scale) as usize),
            Comb::new(((comb_lengths[5] + offset) as f32 * sr_scale) as usize),
            Comb::new(((comb_lengths[6] + offset) as f32 * sr_scale) as usize),
            Comb::new(((comb_lengths[7] + offset) as f32 * sr_scale) as usize),
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

impl FrameProcessor<Stereo> for Reverb {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let frames = buffer.len() / 2;

        if self.mono_input.len() < frames {
            self.mono_input.resize(frames, 0.0);
        }
        if self.reverb_out.len() < frames {
            self.reverb_out.resize(frames, 0.0);
        }
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

        let room_size_val = self.room_size_buffer[0].clamp(0.0, 0.98);
        let damping_val = self.damping_buffer[0].clamp(0.0, 1.0);

        for (i, frame) in buffer.chunks(2).enumerate() {
            if frame.len() == 2 {
                self.mono_input[i] = (frame[0] + frame[1]) * 0.5 * 0.015; // Scale down
            }
        }

        self.reverb_out.fill(0.0);
        let input_slice = &self.mono_input[0..frames];
        let output_slice = &mut self.reverb_out[0..frames];

        for comb in &mut self.combs {
            comb.process_block(input_slice, output_slice, room_size_val, damping_val);
        }

        for allpass in &mut self.allpasses {
            allpass.process_block(output_slice);
        }

        for (i, frame) in buffer.chunks_mut(2).enumerate() {
            if frame.len() == 2 {
                let wet = self.reverb_out[i];
                frame[0] = wet;
                frame[1] = wet;
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        if (self.sample_rate - sample_rate).abs() > 1.0 {
            self.sample_rate = sample_rate;
            self.room_size.set_sample_rate(sample_rate);
            self.damping.set_sample_rate(sample_rate);
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

impl Default for Reverb {
    fn default() -> Self {
        Self::new()
    }
}
