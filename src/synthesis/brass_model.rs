use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec;
use alloc::vec::Vec;
use core::f32::consts::PI;

#[derive(Clone, Copy)]
struct PhysBiQuad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl PhysBiQuad {
    fn new() -> Self {
        PhysBiQuad {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn set_resonance_lowpass(&mut self, freq: f32, radius: f32, sample_rate: f32) {
        let norm_freq = (2.0 * PI * freq / sample_rate).clamp(0.0, PI);

        self.a2 = radius * radius;
        self.a1 = -2.0 * radius * libm::cosf(norm_freq);
        self.b0 = 1.0 + self.a1 + self.a2;
        self.b1 = 0.0;
        self.b2 = 0.0;
    }

    fn process(&mut self, input: f32) -> f32 {
        let out = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = out;

        out
    }
}

/// A physical model of a brass instrument.
/// This is very much WIP and will continue to be tweaked over time...
pub struct BrassModel {
    pitch: AudioParam,
    breath_pressure: AudioParam,
    lip_tension: AudioParam,

    delay_line: Vec<f32>,
    write_ptr: usize,
    sample_rate: f32,

    lip_filter: PhysBiQuad,
    dc_blocker: f32,
    lp_state: f32,
    bell_state: f32,
    last_out: f32,
    vibrato_phase: f32,

    pitch_buffer: Vec<f32>,
    breath_buffer: Vec<f32>,
    tension_buffer: Vec<f32>,

    rng_state: u32,
}

impl BrassModel {
    pub fn new(pitch: AudioParam, breath: AudioParam, tension: AudioParam) -> Self {
        let sample_rate = 44100.0;
        let buffer_size = (sample_rate / 20.0) as usize;

        BrassModel {
            pitch,
            breath_pressure: breath,
            lip_tension: tension,
            delay_line: vec![0.0; buffer_size],
            write_ptr: 0,
            sample_rate,
            lip_filter: PhysBiQuad::new(),
            dc_blocker: 0.0,
            lp_state: 0.0,
            bell_state: 0.0,
            last_out: 0.0,
            vibrato_phase: 0.0,
            pitch_buffer: Vec::new(),
            breath_buffer: Vec::new(),
            tension_buffer: Vec::new(),
            rng_state: 12345,
        }
    }

    fn next_random(rng_state: &mut u32) -> f32 {
        *rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let val = (*rng_state >> 16) & 0x7FFF;
        (val as f32 / 32768.0) * 2.0 - 1.0
    }
}

impl FrameProcessor<Mono> for BrassModel {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.pitch_buffer.len() < len {
            self.pitch_buffer.resize(len, 0.0);
        }
        if self.breath_buffer.len() < len {
            self.breath_buffer.resize(len, 0.0);
        }
        if self.tension_buffer.len() < len {
            self.tension_buffer.resize(len, 0.0);
        }

        self.pitch
            .process(&mut self.pitch_buffer[0..len], sample_index);
        self.breath_pressure
            .process(&mut self.breath_buffer[0..len], sample_index);
        self.lip_tension
            .process(&mut self.tension_buffer[0..len], sample_index);

        let delay_len = self.delay_line.len();
        if delay_len == 0 {
            return;
        }

        let inv_sr = 1.0 / self.sample_rate;

        for (i, sample) in buffer.iter_mut().enumerate() {
            let base_pitch = self.pitch_buffer[i];
            let breath = self.breath_buffer[i];
            let tension = self.tension_buffer[i];

            self.vibrato_phase += 5.0 * inv_sr;
            if self.vibrato_phase > 1.0 {
                self.vibrato_phase -= 1.0;
            }

            let vib_depth = 0.005 * breath;
            let vibrato = libm::sinf(self.vibrato_phase * 2.0 * PI) * vib_depth;

            let pitch_val = base_pitch * (1.0 + vibrato);

            let lip_freq = pitch_val * (1.01 + 0.05 * tension);
            self.lip_filter
                .set_resonance_lowpass(lip_freq, 0.996, self.sample_rate);

            let period = (self.sample_rate / pitch_val).max(2.0);
            let read_pos = (self.write_ptr as f32 - period + delay_len as f32) % delay_len as f32;
            let idx_a = read_pos as usize;
            let idx_b = (idx_a + 1) % delay_len;
            let frac = read_pos - idx_a as f32;
            let bore_out = self.delay_line[idx_a] * (1.0 - frac) + self.delay_line[idx_b] * frac;

            let delta_p = breath - bore_out * 0.9;
            let lip_pos = self.lip_filter.process(delta_p);

            let threshold = 0.05;
            let lip_opening = (lip_pos - threshold).max(0.0);

            let noise = Self::next_random(&mut self.rng_state) * 0.02 * breath;
            let airflow = (breath + noise) * lip_opening;

            let saturated = libm::tanhf(airflow);

            let lp_cutoff = 0.1 + 0.6 * breath;
            self.lp_state += lp_cutoff * (saturated - self.lp_state);

            let ac_signal = self.lp_state - self.dc_blocker + 0.995 * self.dc_blocker;
            self.dc_blocker = self.lp_state;

            self.delay_line[self.write_ptr] = ac_signal;

            let rc = 1.0 / (2.0 * PI * 250.0);
            let dt = 1.0 / self.sample_rate;
            let alpha = rc / (rc + dt);
            let bell_out = alpha * (self.bell_state + ac_signal - self.last_out);
            self.bell_state = bell_out;
            self.last_out = ac_signal;

            *sample = bell_out * 3.0;

            self.write_ptr = (self.write_ptr + 1) % delay_len;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.pitch.set_sample_rate(sample_rate);
        self.breath_pressure.set_sample_rate(sample_rate);
        self.lip_tension.set_sample_rate(sample_rate);

        let buffer_size = (sample_rate / 20.0) as usize;
        if buffer_size > self.delay_line.len() {
            self.delay_line.resize(buffer_size, 0.0);
        }
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "BrassModel"
    }
}
