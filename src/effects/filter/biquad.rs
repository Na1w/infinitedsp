use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use core::f32::consts::PI;
use alloc::vec::Vec;

/// The type of biquad filter.
pub enum FilterType {
    /// Low-pass filter.
    LowPass,
    /// High-pass filter.
    HighPass,
    /// Band-pass filter.
    BandPass,
    /// Notch filter.
    Notch,
}

/// A biquad filter implementation.
///
/// Can be configured as LowPass, HighPass, BandPass, or Notch.
pub struct Biquad {
    filter_type: FilterType,
    frequency: AudioParam,
    q: AudioParam,
    gain_db: AudioParam,
    sample_rate: f32,

    a0: f32, a1: f32, a2: f32,
    b0: f32, b1: f32, b2: f32,

    x1: f32, x2: f32,
    y1: f32, y2: f32,

    freq_buffer: Vec<f32>,
    q_buffer: Vec<f32>,
    gain_buffer: Vec<f32>,
}

impl Biquad {
    /// Creates a new Biquad filter.
    ///
    /// # Arguments
    /// * `filter_type` - The type of filter.
    /// * `frequency` - Cutoff/Center frequency in Hz.
    /// * `q` - Q factor (resonance).
    pub fn new(filter_type: FilterType, frequency: AudioParam, q: AudioParam) -> Self {
        Biquad {
            filter_type,
            frequency,
            q,
            gain_db: AudioParam::Static(0.0),
            sample_rate: 44100.0,
            a0: 0.0, a1: 0.0, a2: 0.0,
            b0: 0.0, b1: 0.0, b2: 0.0,
            x1: 0.0, x2: 0.0,
            y1: 0.0, y2: 0.0,
            freq_buffer: Vec::new(),
            q_buffer: Vec::new(),
            gain_buffer: Vec::new(),
        }
    }

    /// Creates a new LowPass filter.
    ///
    /// # Arguments
    /// * `frequency` - Cutoff frequency in Hz.
    /// * `q` - Q factor.
    pub fn new_lowpass(frequency: AudioParam, q: AudioParam) -> Self {
        Self::new(FilterType::LowPass, frequency, q)
    }

    /// Sets the Q factor parameter.
    pub fn set_q(&mut self, q: AudioParam) {
        self.q = q;
    }

    /// Sets the gain parameter (for shelving/peaking filters, currently unused in basic types).
    pub fn set_gain(&mut self, gain: AudioParam) {
        self.gain_db = gain;
    }

    fn recalc(&mut self, freq: f32, q: f32) {
        let w0 = 2.0 * PI * freq / self.sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        match self.filter_type {
            FilterType::LowPass => {
                self.b0 = (1.0 - cos_w0) / 2.0;
                self.b1 = 1.0 - cos_w0;
                self.b2 = (1.0 - cos_w0) / 2.0;
                self.a0 = 1.0 + alpha;
                self.a1 = -2.0 * cos_w0;
                self.a2 = 1.0 - alpha;
            },
            FilterType::HighPass => {
                self.b0 = (1.0 + cos_w0) / 2.0;
                self.b1 = -(1.0 + cos_w0);
                self.b2 = (1.0 + cos_w0) / 2.0;
                self.a0 = 1.0 + alpha;
                self.a1 = -2.0 * cos_w0;
                self.a2 = 1.0 - alpha;
            },
            FilterType::BandPass => {
                self.b0 = alpha;
                self.b1 = 0.0;
                self.b2 = -alpha;
                self.a0 = 1.0 + alpha;
                self.a1 = -2.0 * cos_w0;
                self.a2 = 1.0 - alpha;
            },
            FilterType::Notch => {
                self.b0 = 1.0;
                self.b1 = -2.0 * cos_w0;
                self.b2 = 1.0;
                self.a0 = 1.0 + alpha;
                self.a1 = -2.0 * cos_w0;
                self.a2 = 1.0 - alpha;
            },
        }

        let inv_a0 = 1.0 / self.a0;
        self.b0 *= inv_a0;
        self.b1 *= inv_a0;
        self.b2 *= inv_a0;
        self.a1 *= inv_a0;
        self.a2 *= inv_a0;
    }
}

impl FrameProcessor for Biquad {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.freq_buffer.len() < len { self.freq_buffer.resize(len, 0.0); }
        if self.q_buffer.len() < len { self.q_buffer.resize(len, 0.0); }
        if self.gain_buffer.len() < len { self.gain_buffer.resize(len, 0.0); }

        self.frequency.process(&mut self.freq_buffer[0..len], sample_index);
        self.q.process(&mut self.q_buffer[0..len], sample_index);
        self.gain_db.process(&mut self.gain_buffer[0..len], sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let freq = self.freq_buffer[i];
            let q = self.q_buffer[i];

            self.recalc(freq, q);

            let x = *sample;
            let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
                  - self.a1 * self.y1 - self.a2 * self.y2;

            self.x2 = self.x1;
            self.x1 = x;
            self.y2 = self.y1;
            self.y1 = y;

            *sample = y;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.frequency.set_sample_rate(sample_rate);
        self.q.set_sample_rate(sample_rate);
        self.gain_db.set_sample_rate(sample_rate);
    }
}
