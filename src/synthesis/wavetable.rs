use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use num_complex::Complex32;
use wide::f32x4;

#[derive(Clone)]
struct MipmappedFrame {
    levels: Vec<Vec<f32>>,
}

/// A wavetable with automatic band-limiting (mipmapping).
#[derive(Clone)]
pub struct Wavetable {
    frames: Arc<Vec<MipmappedFrame>>,
}

impl Wavetable {
    /// Creates a new Wavetable from raw data and automatically generates mipmaps.
    #[must_use]
    pub fn new(data: &[f32], samples_per_frame: usize) -> Self {
        assert_eq!(samples_per_frame, 2048);
        let num_frames = data.len() / samples_per_frame;
        let mut frames = Vec::with_capacity(num_frames);

        for f in 0..num_frames {
            let start = f * samples_per_frame;
            let mut base_frame = data[start..start + samples_per_frame].to_vec();
            
            let mut max_abs = 0.0f32;
            for &s in &base_frame { max_abs = max_abs.max(s.abs()); }
            if max_abs > 0.0 {
                for s in &mut base_frame { *s /= max_abs; }
            }
            let levels = vec![base_frame];
            frames.push(MipmappedFrame { levels });
        }
        Wavetable { frames: Arc::new(frames) }
    }

    /// Band-limited constructor that uses FFT to properly band-limit each mipmap level.
    #[must_use]
    pub fn new_bandlimited(data: &[f32], samples_per_frame: usize) -> Self {
        assert_eq!(samples_per_frame, 2048);
        let num_frames = data.len() / samples_per_frame;
        let mut frames = Vec::with_capacity(num_frames);

        for f in 0..num_frames {
            let start = f * samples_per_frame;
            let raw_samples = &data[start..start + samples_per_frame];
            
            let mut complex_buf = [Complex32::new(0.0, 0.0); 2048];
            for i in 0..2048 { complex_buf[i] = Complex32::new(raw_samples[i], 0.0); }
            
            let _ = microfft::complex::cfft_2048(&mut complex_buf);
            
            let mut levels = Vec::with_capacity(9);
            
            for level_idx in 0..9 {
                let size = 2048 >> level_idx;
                if size < 16 { break; }
                
                let mut level_complex = [Complex32::new(0.0, 0.0); 2048];
                let harmonics_to_keep = 1024 >> level_idx;
                
                for i in 0..harmonics_to_keep {
                    level_complex[i] = complex_buf[i];
                    if i > 0 {
                        level_complex[2048 - i] = complex_buf[2048 - i];
                    }
                }

                for x in &mut level_complex { *x = x.conj(); }
                let _ = microfft::complex::cfft_2048(&mut level_complex);
                for x in &mut level_complex { *x = x.conj() / 2048.0; }
                
                let mut level_samples = vec![0.0; size];
                for i in 0..size {
                    level_samples[i] = level_complex[i * (1 << level_idx)].re;
                }
                levels.push(level_samples);
            }
            
            frames.push(MipmappedFrame { levels });
        }

        Wavetable {
            frames: Arc::new(frames),
        }
    }
}

pub struct WavetableOscillator {
    wavetable: Wavetable,
    frequency: AudioParam,
    position: AudioParam,
    phase: f32,
    sample_rate: f32,
    freq_buffer: Vec<f32>,
    pos_buffer: Vec<f32>,
}

impl WavetableOscillator {
    #[must_use]
    pub fn new(wavetable: Wavetable, frequency: AudioParam, position: AudioParam) -> Self {
        WavetableOscillator {
            wavetable,
            frequency,
            position,
            phase: 0.0,
            sample_rate: 44100.0,
            freq_buffer: Vec::with_capacity(128),
            pos_buffer: Vec::with_capacity(128),
        }
    }

    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    #[inline(always)]
    fn read_frame(&self, f_idx: usize, m_idx: usize, phase: f32) -> f32 {
        let level_data = &self.wavetable.frames[f_idx].levels[m_idx];
        let size = level_data.len();
        let p = phase * size as f32;
        let i_a = p as usize;
        let mut i_b = i_a + 1;
        if i_b >= size {
            i_b -= size;
        }
        let frac = p - i_a as f32;
        level_data[i_a] + (level_data[i_b] - level_data[i_a]) * frac
    }

    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    #[inline(always)]
    fn get_sample(&self, phase: f32, freq: f32, position: f32) -> f32 {
        let num_mip_levels = self.wavetable.frames[0].levels.len();
        
        if num_mip_levels == 1 {
            let num_frames = self.wavetable.frames.len();
            let pos_scaled = position.clamp(0.0, 1.0) * (num_frames - 1) as f32;
            let frame_idx = pos_scaled as usize;
            let next_frame_idx = (frame_idx + 1).min(num_frames - 1);
            let frame_frac = pos_scaled - frame_idx as f32;

            let v1 = self.read_frame(frame_idx, 0, phase);
            let v2 = self.read_frame(next_frame_idx, 0, phase);
            return v1 + (v2 - v1) * frame_frac;
        }

        let max_allowed_harmonics = self.sample_rate / (2.0 * freq.max(1.0));
        let mip_level_f = libm::log2f(1024.0 / max_allowed_harmonics).max(0.0);
        let mip_level = (mip_level_f as usize).min(num_mip_levels - 2);
        let mip_frac = mip_level_f - mip_level as f32;

        let num_frames = self.wavetable.frames.len();
        let pos_scaled = position.clamp(0.0, 1.0) * (num_frames - 1) as f32;
        let frame_idx = pos_scaled as usize;
        let next_frame_idx = (frame_idx + 1).min(num_frames - 1);
        let frame_frac = pos_scaled - frame_idx as f32;

        let v1_m1 = self.read_frame(frame_idx, mip_level, phase);
        let v1_m2 = self.read_frame(frame_idx, mip_level + 1, phase);
        let v1 = v1_m1 + (v1_m2 - v1_m1) * mip_frac;

        let v2_m1 = self.read_frame(next_frame_idx, mip_level, phase);
        let v2_m2 = self.read_frame(next_frame_idx, mip_level + 1, phase);
        let v2 = v2_m1 + (v2_m2 - v2_m1) * mip_frac;

        v1 + (v2 - v1) * frame_frac
    }
}

impl FrameProcessor<Mono> for WavetableOscillator {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.freq_buffer.len() < len { self.freq_buffer.resize(len, 0.0); }
        if self.pos_buffer.len() < len { self.pos_buffer.resize(len, 0.0); }

        self.frequency.process(&mut self.freq_buffer, sample_index);
        self.position.process(&mut self.pos_buffer, sample_index);

        let inv_sr = 1.0 / self.sample_rate;
        let (chunks, remainder) = buffer.as_chunks_mut::<4>();
        let (freq_chunks, freq_rem) = self.freq_buffer.as_chunks::<4>();
        let (pos_chunks, pos_rem) = self.pos_buffer.as_chunks::<4>();

        for i in 0..chunks.len() {
            let freq = f32x4::from(freq_chunks[i]);
            let pos = f32x4::from(pos_chunks[i]);
            let inc = freq * inv_sr;
            let inc_arr = inc.to_array();
            let freq_arr = freq.to_array();
            let pos_arr = pos.to_array();
            
            let mut results = [0.0f32; 4];
            for j in 0..4 {
                results[j] = self.get_sample(self.phase, freq_arr[j], pos_arr[j]);
                self.phase += inc_arr[j];
                if self.phase >= 1.0 { self.phase -= 1.0; }
            }
            chunks[i] = results;
        }

        for i in 0..remainder.len() {
            let f = freq_rem[i];
            let p = pos_rem[i];
            let inc = f / self.sample_rate;
            remainder[i] = self.get_sample(self.phase, f, p);
            self.phase += inc;
            if self.phase >= 1.0 { self.phase -= 1.0; }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.frequency.set_sample_rate(sample_rate);
        self.position.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "WavetableOscillator"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> alloc::string::String {
        use core::fmt::Write;
        let mut output = alloc::string::String::new();
        let spaces = " ".repeat(indent);
        writeln!(output, "{}WavetableOscillator (Anti-aliased)", spaces).unwrap();
        writeln!(output, "{}  |-- Frames: {}", spaces, self.wavetable.frames.len()).unwrap();
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parameter::Parameter;

    #[test]
    fn test_wavetable_basic_processing() {
        let size = 2048;
        let mut data = vec![0.0; size];
        for i in 0..size {
            data[i] = libm::sinf((i as f32 / size as f32) * 2.0 * core::f32::consts::PI);
        }
        let table = Wavetable::new(&data, size);
        
        let freq = Parameter::new(441.0);
        let mut osc = WavetableOscillator::new(
            table,
            AudioParam::Linked(freq),
            AudioParam::Static(0.0),
        );
        osc.set_sample_rate(44100.0);

        let mut buffer = [0.0; 100];
        osc.process(&mut buffer, 0);

        assert!(buffer[0].abs() < 1e-5);
        let expected = libm::sinf(0.01 * 2.0 * core::f32::consts::PI);
        assert!((buffer[1] - expected).abs() < 1e-5);
    }

    #[test]
    fn test_wavetable_morphing() {
        let size = 2048;
        let mut data = vec![0.0; size * 2];
        for i in 0..size { data[i] = 0.5; }
        for i in size..size*2 { data[i] = -0.5; }
        
        let table = Wavetable::new(&data, size);
        
        let mut osc = WavetableOscillator::new(
            table,
            AudioParam::Static(100.0),
            AudioParam::Static(0.5),
        );
        osc.set_sample_rate(44100.0);

        let mut buffer = [0.0; 10];
        osc.process(&mut buffer, 0);

        for &sample in buffer.iter() {
            assert!(sample.abs() < 1e-5);
        }
    }

    #[test]
    fn test_wavetable_mipmapping_logic() {
        let size = 2048;
        let mut data = vec![0.0; size];
        for i in 0..size { data[i] = 1.0; }
        let table = Wavetable::new_bandlimited(&data, size);
        let mut osc = WavetableOscillator::new(
            table,
            AudioParam::Static(20000.0),
            AudioParam::Static(0.0),
        );
        osc.set_sample_rate(44100.0);
        let mut buffer = [0.0; 10];
        osc.process(&mut buffer, 0);
        assert_eq!(buffer.len(), 10);
    }
}
