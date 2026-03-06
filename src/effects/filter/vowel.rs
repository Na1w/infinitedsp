use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::effects::filter::state_variable::{StateVariableFilter, SvfType};
use crate::FrameProcessor;

/// Standard vowels for the [`VowelFilter`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Vowel {
    /// Vowel 'A' as in 'Father'.
    A,
    /// Vowel 'E' as in 'Eat'.
    E,
    /// Vowel 'I' as in 'It'.
    I,
    /// Vowel 'O' as in 'Old'.
    O,
    /// Vowel 'U' as in 'Cool'.
    U,
}

impl Vowel {
    /// Returns the formant frequencies (F1, F2, F3) for this vowel.
    pub fn formants(&self) -> (f32, f32, f32) {
        match self {
            Vowel::A => VowelFilter::A,
            Vowel::E => VowelFilter::E,
            Vowel::I => VowelFilter::I,
            Vowel::O => VowelFilter::O,
            Vowel::U => VowelFilter::U,
        }
    }

    /// Returns the index of the vowel (0-4).
    pub fn index(&self) -> usize {
        match self {
            Vowel::A => 0,
            Vowel::E => 1,
            Vowel::I => 2,
            Vowel::O => 3,
            Vowel::U => 4,
        }
    }

    /// Returns a vowel from an index (0-4).
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Vowel::A,
            1 => Vowel::E,
            2 => Vowel::I,
            3 => Vowel::O,
            _ => Vowel::U,
        }
    }
}

/// A Vowel (Formant) Filter.
///
/// Simulates the human vocal tract by using three parallel band-pass filters
/// tuned to specific formant frequencies. It can morph between different vowels
/// using the `vowel_morph` parameter (0.0 to 4.0).
pub struct VowelFilter {
    /// Internal filter for F1.
    pub f1: StateVariableFilter,
    /// Internal filter for F2.
    pub f2: StateVariableFilter,
    /// Internal filter for F3.
    pub f3: StateVariableFilter,
    vowel_morph: AudioParam,
    q: AudioParam,
    sample_rate: f32,
    manual_formants: Option<(f32, f32, f32)>,
}

impl VowelFilter {
    /// Vowel 'A' formant frequencies.
    pub const A: (f32, f32, f32) = (750.0, 1200.0, 2400.0);
    /// Vowel 'E' formant frequencies.
    pub const E: (f32, f32, f32) = (530.0, 1840.0, 2500.0);
    /// Vowel 'I' formant frequencies.
    pub const I: (f32, f32, f32) = (400.0, 2200.0, 3000.0);
    /// Vowel 'O' formant frequencies.
    pub const O: (f32, f32, f32) = (500.0, 900.0, 2300.0);
    /// Vowel 'U' formant frequencies.
    pub const U: (f32, f32, f32) = (350.0, 800.0, 2100.0);

    /// Consonant 'S' formant frequencies.
    pub const S: (f32, f32, f32) = (6500.0, 8500.0, 9800.0);
    /// Consonant 'Z' formant frequencies.
    pub const Z: (f32, f32, f32) = (6000.0, 8000.0, 9500.0);
    /// Consonant 'F' formant frequencies.
    pub const F: (f32, f32, f32) = (3500.0, 5000.0, 7500.0);
    /// Consonant 'V' formant frequencies.
    pub const V: (f32, f32, f32) = (3000.0, 4500.0, 6500.0);
    /// Consonant 'H' formant frequencies.
    pub const H: (f32, f32, f32) = (1000.0, 2000.0, 3000.0);
    /// Consonant 'TH' formant frequencies.
    pub const TH: (f32, f32, f32) = (4500.0, 6000.0, 8000.0);
    /// Consonant 'SH' formant frequencies.
    pub const SH: (f32, f32, f32) = (2200.0, 3500.0, 5500.0);
    /// Consonant 'R' formant frequencies.
    pub const R: (f32, f32, f32) = (450.0, 1300.0, 1700.0);
    /// Consonant 'L' formant frequencies.
    pub const L: (f32, f32, f32) = (400.0, 1000.0, 2800.0);
    /// Consonant 'N' formant frequencies.
    pub const N: (f32, f32, f32) = (250.0, 1000.0, 2000.0);
    /// Consonant 'M' formant frequencies.
    pub const M: (f32, f32, f32) = (250.0, 800.0, 1500.0);
    /// Consonant 'NG' formant frequencies.
    pub const NG: (f32, f32, f32) = (200.0, 1200.0, 2500.0);
    /// Consonant 'W' formant frequencies.
    pub const W: (f32, f32, f32) = (300.0, 700.0, 2200.0);
    /// Consonant 'Y' formant frequencies.
    pub const Y: (f32, f32, f32) = (300.0, 2200.0, 3200.0);

    /// Creates a new VowelFilter.
    ///
    /// # Arguments
    /// * `vowel_morph` - A parameter (0.0 to 4.0) to morph between A, E, I, O, U.
    /// * `q` - The resonance (Q) of the formant filters.
    pub fn new(vowel_morph: AudioParam, q: AudioParam) -> Self {
        let sr = 44100.0;
        VowelFilter {
            f1: StateVariableFilter::new(
                SvfType::BandPass,
                AudioParam::Static(0.0),
                AudioParam::Static(10.0),
            ),
            f2: StateVariableFilter::new(
                SvfType::BandPass,
                AudioParam::Static(0.0),
                AudioParam::Static(10.0),
            ),
            f3: StateVariableFilter::new(
                SvfType::BandPass,
                AudioParam::Static(0.0),
                AudioParam::Static(10.0),
            ),
            vowel_morph,
            q,
            sample_rate: sr,
            manual_formants: None,
        }
    }

    fn get_formants_at(&self, morph: f32) -> (f32, f32, f32) {
        let m = morph.clamp(0.0, 4.0);
        let idx = libm::floorf(m) as usize;
        let frac = m - idx as f32;

        if idx >= 4 {
            return Vowel::U.formants();
        }

        let v1 = Vowel::from_index(idx).formants();
        let v2 = Vowel::from_index(idx + 1).formants();

        (
            v1.0 + (v2.0 - v1.0) * frac,
            v1.1 + (v2.1 - v1.1) * frac,
            v1.2 + (v2.2 - v1.2) * frac,
        )
    }

    /// Manually sets the formant frequencies. This overrides the `vowel_morph` parameter.
    pub fn set_formants(&mut self, f1: f32, f2: f32, f3: f32) {
        self.manual_formants = Some((f1, f2, f3));
    }

    /// Sets the resonance (Q) for all formant filters.
    pub fn set_q(&mut self, q: AudioParam) {
        self.q = q;
    }

    /// Efficiently processes a single sample with manual formant control.
    #[inline(always)]
    pub fn tick_manual(&mut self, input: f32, f1: f32, f2: f32, f3: f32, q: f32) -> f32 {
        let o1 = self.f1.tick(input, f1, q);
        let o2 = self.f2.tick(input, f2, q);
        let o3 = self.f3.tick(input, f3, q);
        o1 * 1.4 + o2 * 0.8 + o3 * 0.6
    }
}

impl FrameProcessor<Mono> for VowelFilter {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        for (i, sample) in buffer.iter_mut().enumerate() {
            let current_idx = sample_index + i as u64;
            let input = *sample;

            let morph = self.vowel_morph.get_value_at(current_idx);
            let cur_q_val = self.q.get_value_at(current_idx);

            let (f1, f2, f3) = if let Some(manual) = self.manual_formants {
                manual
            } else {
                self.get_formants_at(morph)
            };

            *sample = self.tick_manual(input, f1, f2, f3, cur_q_val);
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.f1.set_sample_rate(sample_rate);
        self.f2.set_sample_rate(sample_rate);
        self.f3.set_sample_rate(sample_rate);
        self.vowel_morph.set_sample_rate(sample_rate);
        self.q.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.f1.reset();
        self.f2.reset();
        self.f3.reset();
        self.vowel_morph.reset();
        self.q.reset();
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "VowelFilter"
    }
}
