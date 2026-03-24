use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use num_complex::Complex32;

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
    pub fn new(data: Vec<f32>, samples_per_frame: usize) -> Self {
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

            let mut levels = vec![base_frame];
            
            for i in 1..10 {
                let prev_level = &levels[i - 1];
                let new_size = prev_level.len() / 2;
                if new_size < 8 { break; }
                
                let mut new_level = vec![0.0; new_size];
                for j in 0..new_size {
                    new_level[j] = (prev_level[j * 2] + prev_level[j * 2 + 1]) * 0.5;
                }
                levels.push(new_level);
            }
            
            frames.push(MipmappedFrame { levels });
        }

        Wavetable {
            frames: Arc::new(frames),
        }
    }

    /// Band-limited constructor that uses FFT to properly band-limit each mipmap level.
    pub fn new_bandlimited(data: Vec<f32>, samples_per_frame: usize) -> Self {
        assert_eq!(samples_per_frame, 2048);
        let num_frames = data.len() / samples_per_frame;
        let mut frames = Vec::with_capacity(num_frames);

        for f in 0..num_frames {
            let start = f * samples_per_frame;
            let raw_samples = &data[start..start + samples_per_frame];
            
            let mut complex_buf = [Complex32::new(0.0, 0.0); 2048];
            for i in 0..2048 { complex_buf[i] = Complex32::new(raw_samples[i], 0.0); }
            
            let _ = microfft::complex::cfft_2048(&mut complex_buf);
            
            let mut levels = Vec::new();
            
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

                for x in level_complex.iter_mut() { *x = x.conj(); }
                let _ = microfft::complex::cfft_2048(&mut level_complex);
                for x in level_complex.iter_mut() { *x = x.conj() / 2048.0; }
                
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

/// A Wavetable Oscillator with anti-aliasing.
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
    pub fn new(wavetable: Wavetable, frequency: AudioParam, position: AudioParam) -> Self {
        WavetableOscillator {
            wavetable,
            frequency,
            position,
            phase: 0.0,
            sample_rate: 44100.0,
            freq_buffer: Vec::new(),
            pos_buffer: Vec::new(),
        }
    }

    #[inline(always)]
    fn tick(&mut self, freq: f32, position: f32) -> f32 {
        let inc = freq / self.sample_rate;
        self.phase += inc;
        while self.phase >= 1.0 { self.phase -= 1.0; }
        while self.phase < 0.0 { self.phase += 1.0; }

        let max_allowed_harmonics = self.sample_rate / (2.0 * freq.max(1.0));
        let mip_level_f = libm::log2f(1024.0 / max_allowed_harmonics).max(0.0);
        let mip_level = (mip_level_f as usize).min(self.wavetable.frames[0].levels.len() - 2);
        let mip_frac = mip_level_f - mip_level as f32;

        let num_frames = self.wavetable.frames.len();
        let pos_scaled = position.clamp(0.0, 1.0) * (num_frames - 1) as f32;
        let frame_idx = pos_scaled as usize;
        let next_frame_idx = (frame_idx + 1).min(num_frames - 1);
        let frame_frac = pos_scaled - frame_idx as f32;

        let read_frame = |f_idx: usize, m_idx: usize, phase: f32| -> f32 {
            let level_data = &self.wavetable.frames[f_idx].levels[m_idx];
            let size = level_data.len();
            let p = phase * size as f32;
            let i_a = p as usize;
            let i_b = (i_a + 1) % size;
            let frac = p - i_a as f32;
            level_data[i_a] + (level_data[i_b] - level_data[i_a]) * frac
        };

        let v1_m1 = read_frame(frame_idx, mip_level, self.phase);
        let v1_m2 = read_frame(frame_idx, mip_level + 1, self.phase);
        let v1 = v1_m1 + (v1_m2 - v1_m1) * mip_frac;

        let v2_m1 = read_frame(next_frame_idx, mip_level, self.phase);
        let v2_m2 = read_frame(next_frame_idx, mip_level + 1, self.phase);
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

        for i in 0..len {
            buffer[i] = self.tick(self.freq_buffer[i], self.pos_buffer[i]);
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
