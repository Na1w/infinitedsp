use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;
use wide::f32x4;

const PARAM_CHUNK_SIZE: usize = 64;
const I16_SCALE: f32 = 32767.0;
const I16_SCALE_INV: f32 = 1.0 / 32767.0;

/// A memory-efficient digital delay effect using 16-bit integer storage and 2x downsampling.
///
/// Saves 75% memory compared to standard Delay by storing samples as i16 and running the delay line
/// at half the sample rate.
/// Uses SIMD-accelerated processing and Cubic (Hermite) interpolation to restore high-end.
pub struct DelayLowMem {
    buffer: Vec<i16>,
    write_ptr: usize,
    /// 0 or 1, tracking the downsampling phase
    phase: usize,
    /// Accumulator for downsampling filter
    downsample_acc: f32,
    delay_time: AudioParam,
    feedback: AudioParam,
    mix: AudioParam,
    max_delay_seconds: f32,
    sample_rate: f32,
    delay_buffer: [f32; PARAM_CHUNK_SIZE],
    feedback_buffer: [f32; PARAM_CHUNK_SIZE],
    mix_buffer: [f32; PARAM_CHUNK_SIZE],
}

impl DelayLowMem {
    /// Creates a new DelayLowMem.
    ///
    /// # Arguments
    /// * `max_delay_seconds`: Maximum buffer size in seconds.
    /// * `delay_time`: Delay time in seconds.
    /// * `feedback`: Feedback amount (0.0 - 1.0).
    /// * `mix`: Dry/Wet mix (0.0 - 1.0).
    pub fn new(
        max_delay_seconds: f32,
        delay_time: AudioParam,
        feedback: AudioParam,
        mix: AudioParam,
    ) -> Self {
        let sample_rate = 44100.0;
        let size = (max_delay_seconds * sample_rate * 0.5) as usize;

        DelayLowMem {
            buffer: vec![0; size],
            write_ptr: 0,
            phase: 0,
            downsample_acc: 0.0,
            delay_time,
            feedback,
            mix,
            max_delay_seconds,
            sample_rate,
            delay_buffer: [0.0; PARAM_CHUNK_SIZE],
            feedback_buffer: [0.0; PARAM_CHUNK_SIZE],
            mix_buffer: [0.0; PARAM_CHUNK_SIZE],
        }
    }

    /// Sets the delay time parameter.
    pub fn set_delay_time(&mut self, delay_time: AudioParam) {
        self.delay_time = delay_time;
    }

    /// Sets the feedback parameter.
    pub fn set_feedback(&mut self, feedback: AudioParam) {
        self.feedback = feedback;
    }

    /// Sets the mix parameter.
    pub fn set_mix(&mut self, mix: AudioParam) {
        self.mix = mix;
    }
}

impl FrameProcessor<Mono> for DelayLowMem {
    fn process(&mut self, buffer: &mut [f32], start_sample_index: u64) {
        let len = self.buffer.len();
        if len == 0 {
            return;
        }
        let len_f = len as f32;
        let len_f_vec = f32x4::splat(len_f);
        let delay_sr = self.sample_rate * 0.5;
        let delay_sr_vec = f32x4::splat(delay_sr);
        let i16_scale_inv_vec = f32x4::splat(I16_SCALE_INV);

        let mut current_sample_index = start_sample_index;

        for chunk in buffer.chunks_mut(PARAM_CHUNK_SIZE) {
            let chunk_len = chunk.len();

            self.delay_time
                .process(&mut self.delay_buffer[0..chunk_len], current_sample_index);
            self.feedback.process(
                &mut self.feedback_buffer[0..chunk_len],
                current_sample_index,
            );
            self.mix
                .process(&mut self.mix_buffer[0..chunk_len], current_sample_index);

            let mut i = 0;

            if self.phase == 1 && i < chunk_len {
                let input = chunk[i];
                let delay_seconds = self.delay_buffer[i];
                let fb = self.feedback_buffer[i];
                let mix = self.mix_buffer[i];

                let current_pos = self.write_ptr as f32 + 0.5;
                let delay_samples = delay_seconds * delay_sr;
                let mut read_ptr_norm = current_pos - delay_samples;

                if read_ptr_norm < 0.0 { read_ptr_norm += len_f; }
                if read_ptr_norm >= len_f { read_ptr_norm -= len_f; }

                let idx_a = read_ptr_norm as usize;
                let idx_b = if idx_a + 1 == len { 0 } else { idx_a + 1 };
                let idx_prev = if idx_a == 0 { len - 1 } else { idx_a - 1 };
                let idx_next = if idx_b + 1 == len { 0 } else { idx_b + 1 };

                let frac = read_ptr_norm - idx_a as f32;

                let val_prev = self.buffer[idx_prev] as f32 * I16_SCALE_INV;
                let val_a = self.buffer[idx_a] as f32 * I16_SCALE_INV;
                let val_b = self.buffer[idx_b] as f32 * I16_SCALE_INV;
                let val_next = self.buffer[idx_next] as f32 * I16_SCALE_INV;

                let c0 = val_a;
                let c1 = 0.5 * (val_b - val_prev);
                let c2 = val_prev - 2.5 * val_a + 2.0 * val_b - 0.5 * val_next;
                let c3 = 0.5 * (val_next - val_prev) + 1.5 * (val_a - val_b);
                let delayed = ((c3 * frac + c2) * frac + c1) * frac + c0;

                let next_val = input + delayed * fb;

                let avg_val = (self.downsample_acc + next_val) * 0.5;
                let next_val_clamped = avg_val.clamp(-1.0, 1.0);
                self.buffer[self.write_ptr] = (next_val_clamped * I16_SCALE) as i16;
                self.write_ptr += 1;
                if self.write_ptr == len { self.write_ptr = 0; }
                self.phase = 0;

                chunk[i] = input * (1.0 - mix) + delayed * mix;
                i += 1;
            }

            while i + 4 <= chunk_len && self.write_ptr + 2 <= len {
                let delay_seconds = f32x4::new([
                    self.delay_buffer[i], self.delay_buffer[i+1],
                    self.delay_buffer[i+2], self.delay_buffer[i+3]
                ]);
                let fb = f32x4::new([
                    self.feedback_buffer[i], self.feedback_buffer[i+1],
                    self.feedback_buffer[i+2], self.feedback_buffer[i+3]
                ]);
                let mix = f32x4::new([
                    self.mix_buffer[i], self.mix_buffer[i+1],
                    self.mix_buffer[i+2], self.mix_buffer[i+3]
                ]);
                let input = f32x4::new([
                    chunk[i], chunk[i+1], chunk[i+2], chunk[i+3]
                ]);

                let write_base = self.write_ptr as f32;
                let current_pos = f32x4::splat(write_base) + f32x4::new([0.0, 0.5, 1.0, 1.5]);

                let delay_samples = delay_seconds * delay_sr_vec;
                let read_ptr_f = current_pos - delay_samples;

                let mask_under = read_ptr_f.sign_bit();
                let read_ptr_1 = mask_under.blend(read_ptr_f + len_f_vec, read_ptr_f);
                let mask_in_range = (read_ptr_1 - len_f_vec).sign_bit();
                let read_ptr_norm = mask_in_range.blend(read_ptr_1, read_ptr_1 - len_f_vec);

                let idx_f: [f32; 4] = read_ptr_norm.into();
                let idx_a = [
                    idx_f[0] as usize,
                    idx_f[1] as usize,
                    idx_f[2] as usize,
                    idx_f[3] as usize,
                ];

                let idx_prev = [
                    if idx_a[0] == 0 { len - 1 } else { idx_a[0] - 1 },
                    if idx_a[1] == 0 { len - 1 } else { idx_a[1] - 1 },
                    if idx_a[2] == 0 { len - 1 } else { idx_a[2] - 1 },
                    if idx_a[3] == 0 { len - 1 } else { idx_a[3] - 1 },
                ];

                let idx_b = [
                    if idx_a[0] + 1 == len { 0 } else { idx_a[0] + 1 },
                    if idx_a[1] + 1 == len { 0 } else { idx_a[1] + 1 },
                    if idx_a[2] + 1 == len { 0 } else { idx_a[2] + 1 },
                    if idx_a[3] + 1 == len { 0 } else { idx_a[3] + 1 },
                ];

                let idx_next = [
                    if idx_b[0] + 1 == len { 0 } else { idx_b[0] + 1 },
                    if idx_b[1] + 1 == len { 0 } else { idx_b[1] + 1 },
                    if idx_b[2] + 1 == len { 0 } else { idx_b[2] + 1 },
                    if idx_b[3] + 1 == len { 0 } else { idx_b[3] + 1 },
                ];

                let val_prev = f32x4::new([
                    self.buffer[idx_prev[0]] as f32, self.buffer[idx_prev[1]] as f32,
                    self.buffer[idx_prev[2]] as f32, self.buffer[idx_prev[3]] as f32,
                ]) * i16_scale_inv_vec;

                let val_a = f32x4::new([
                    self.buffer[idx_a[0]] as f32, self.buffer[idx_a[1]] as f32,
                    self.buffer[idx_a[2]] as f32, self.buffer[idx_a[3]] as f32,
                ]) * i16_scale_inv_vec;

                let val_b = f32x4::new([
                    self.buffer[idx_b[0]] as f32, self.buffer[idx_b[1]] as f32,
                    self.buffer[idx_b[2]] as f32, self.buffer[idx_b[3]] as f32,
                ]) * i16_scale_inv_vec;

                let val_next = f32x4::new([
                    self.buffer[idx_next[0]] as f32, self.buffer[idx_next[1]] as f32,
                    self.buffer[idx_next[2]] as f32, self.buffer[idx_next[3]] as f32,
                ]) * i16_scale_inv_vec;

                let idx_a_f = f32x4::new([
                    idx_a[0] as f32, idx_a[1] as f32, idx_a[2] as f32, idx_a[3] as f32
                ]);
                let frac = read_ptr_norm - idx_a_f;

                let c0 = val_a;
                let c1 = f32x4::splat(0.5) * (val_b - val_prev);
                let c2 = val_prev - f32x4::splat(2.5) * val_a + f32x4::splat(2.0) * val_b - f32x4::splat(0.5) * val_next;
                let c3 = f32x4::splat(0.5) * (val_next - val_prev) + f32x4::splat(1.5) * (val_a - val_b);

                let delayed = ((c3 * frac + c2) * frac + c1) * frac + c0;

                let next_val = input + delayed * fb;

                let next_val_arr: [f32; 4] = next_val.into();
                let avg0 = (next_val_arr[0] + next_val_arr[1]) * 0.5;
                let avg1 = (next_val_arr[2] + next_val_arr[3]) * 0.5;

                let avg0_clamped = avg0.clamp(-1.0, 1.0);
                let avg1_clamped = avg1.clamp(-1.0, 1.0);

                self.buffer[self.write_ptr] = (avg0_clamped * I16_SCALE) as i16;
                self.buffer[self.write_ptr + 1] = (avg1_clamped * I16_SCALE) as i16;
                self.write_ptr += 2;
                if self.write_ptr == len { self.write_ptr = 0; }

                let out = input * (f32x4::ONE - mix) + delayed * mix;
                let out_arr: [f32; 4] = out.into();
                chunk[i] = out_arr[0];
                chunk[i+1] = out_arr[1];
                chunk[i+2] = out_arr[2];
                chunk[i+3] = out_arr[3];

                i += 4;
            }

            while i < chunk_len {
                let input = chunk[i];
                let delay_seconds = self.delay_buffer[i];
                let fb = self.feedback_buffer[i];
                let mix = self.mix_buffer[i];

                let current_pos = self.write_ptr as f32 + (self.phase as f32 * 0.5);
                let delay_samples = delay_seconds * delay_sr;
                let mut read_ptr_norm = current_pos - delay_samples;

                if read_ptr_norm < 0.0 { read_ptr_norm += len_f; }
                if read_ptr_norm >= len_f { read_ptr_norm -= len_f; }

                let idx_a = read_ptr_norm as usize;
                let idx_b = if idx_a + 1 == len { 0 } else { idx_a + 1 };
                let idx_prev = if idx_a == 0 { len - 1 } else { idx_a - 1 };
                let idx_next = if idx_b + 1 == len { 0 } else { idx_b + 1 };

                let frac = read_ptr_norm - idx_a as f32;

                let val_prev = self.buffer[idx_prev] as f32 * I16_SCALE_INV;
                let val_a = self.buffer[idx_a] as f32 * I16_SCALE_INV;
                let val_b = self.buffer[idx_b] as f32 * I16_SCALE_INV;
                let val_next = self.buffer[idx_next] as f32 * I16_SCALE_INV;

                let c0 = val_a;
                let c1 = 0.5 * (val_b - val_prev);
                let c2 = val_prev - 2.5 * val_a + 2.0 * val_b - 0.5 * val_next;
                let c3 = 0.5 * (val_next - val_prev) + 1.5 * (val_a - val_b);
                let delayed = ((c3 * frac + c2) * frac + c1) * frac + c0;

                let next_val = input + delayed * fb;

                if self.phase == 0 {
                    self.downsample_acc = next_val;
                    self.phase = 1;
                } else {
                    let avg_val = (self.downsample_acc + next_val) * 0.5;
                    let next_val_clamped = avg_val.clamp(-1.0, 1.0);
                    self.buffer[self.write_ptr] = (next_val_clamped * I16_SCALE) as i16;
                    self.write_ptr += 1;
                    if self.write_ptr == len { self.write_ptr = 0; }
                    self.phase = 0;
                }

                chunk[i] = input * (1.0 - mix) + delayed * mix;
                i += 1;
            }

            current_sample_index += chunk_len as u64;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.delay_time.set_sample_rate(sample_rate);
        self.feedback.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);

        let new_size = (self.max_delay_seconds * sample_rate * 0.5) as usize;
        if new_size > self.buffer.len() {
            self.buffer.resize(new_size, 0);
        }
    }

    fn reset(&mut self) {
        self.buffer.fill(0);
        self.write_ptr = 0;
        self.phase = 0;
        self.downsample_acc = 0.0;
        self.delay_time.reset();
        self.feedback.reset();
        self.mix.reset();
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Delay (Low Mem)"
    }
}
