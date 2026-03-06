use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::effects::filter::vowel::VowelFilter;
use crate::effects::time::stutter::Stutter;
use crate::synthesis::oscillator::{Oscillator, Waveform};
use crate::synthesis::stack::Stack;
use crate::FrameProcessor;

/// A single phonetic unit for the [`SpeechSynth`].
///
/// Contains formant frequencies, source mix levels, and timing information.
#[derive(Clone, Copy, Debug)]
pub struct Phoneme {
    /// Duration of the phoneme in milliseconds.
    pub duration_ms: f32,
    /// First formant frequency (F1).
    pub f1: f32,
    /// Second formant frequency (F2).
    pub f2: f32,
    /// Third formant frequency (F3).
    pub f3: f32,
    /// Level of the voiced (oscillator) source.
    pub mix_voice: f32,
    /// Level of the noise source (for consonants).
    pub mix_noise: f32,
    /// Pitch modulation factor (1.0 is base pitch).
    pub pitch_mod: f32,
    /// Overall amplitude of the phoneme.
    pub amp: f32,
    /// If true, frequencies will jump instantly instead of smoothing.
    pub jump_freq: bool,
    /// Number of digital glitch repetitions.
    pub glitch_repeats: u32,
}

impl Phoneme {
    /// Creates a new Phoneme.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        duration_ms: f32,
        f1: f32,
        f2: f32,
        f3: f32,
        mix_voice: f32,
        mix_noise: f32,
        pitch_mod: f32,
        amp: f32,
        jump_freq: bool,
    ) -> Self {
        Self {
            duration_ms,
            f1,
            f2,
            f3,
            mix_voice,
            mix_noise,
            pitch_mod,
            amp,
            jump_freq,
            glitch_repeats: 0,
        }
    }

    /// Returns a silent gap phoneme.
    pub const fn gap(duration_ms: f32) -> Self {
        Self::new(duration_ms, 300.0, 1000.0, 2000.0, 0.0, 0.0, 1.0, 0.0, true)
    }

    /// Returns the phoneme sequence for a given token (e.g. "A", "SH", "CH").
    ///
    /// The returned slice contains one or more phonemes representing the token.
    pub fn from_token(token: &str) -> &'static [Self] {
        match token.to_uppercase().as_str() {
            "A" => &PHONEME_A,
            "E" => &PHONEME_E,
            "I" => &PHONEME_I,
            "O" => &PHONEME_O,
            "U" => &PHONEME_U,
            "S" => &PHONEME_S,
            "Z" => &PHONEME_Z,
            "F" => &PHONEME_F,
            "V" => &PHONEME_V,
            "H" => &PHONEME_H,
            "TH" => &PHONEME_TH,
            "SH" => &PHONEME_SH,
            "R" => &PHONEME_R,
            "L" => &PHONEME_L,
            "N" => &PHONEME_N,
            "M" => &PHONEME_M,
            "NG" => &PHONEME_NG,
            "W" => &PHONEME_W,
            "Y" => &PHONEME_Y,
            "EE" => &PHONEME_EE,
            "GAP" => &PHONEME_GAP,
            "CH" => &PHONEME_CH,
            "J" => &PHONEME_J,
            "D" => &PHONEME_D,
            "B" => &PHONEME_B,
            "P" => &PHONEME_P,
            "T" => &PHONEME_T,
            "K" => &PHONEME_K,
            "G" => &PHONEME_G,
            "AI" => &PHONEME_AI,
            _ => &[],
        }
    }
}

static PHONEME_A: [Phoneme; 1] = [Phoneme::new(
    150.0,
    VowelFilter::A.0,
    VowelFilter::A.1,
    VowelFilter::A.2,
    1.2,
    0.0,
    1.0,
    1.0,
    false,
)];
static PHONEME_E: [Phoneme; 1] = [Phoneme::new(
    150.0,
    VowelFilter::E.0,
    VowelFilter::E.1,
    VowelFilter::E.2,
    1.2,
    0.0,
    1.0,
    1.0,
    false,
)];
static PHONEME_I: [Phoneme; 1] = [Phoneme::new(
    150.0,
    VowelFilter::I.0,
    VowelFilter::I.1,
    VowelFilter::I.2,
    1.2,
    0.0,
    1.0,
    1.0,
    false,
)];
static PHONEME_O: [Phoneme; 1] = [Phoneme::new(
    150.0,
    VowelFilter::O.0,
    VowelFilter::O.1,
    VowelFilter::O.2,
    1.2,
    0.0,
    1.0,
    1.0,
    false,
)];
static PHONEME_U: [Phoneme; 1] = [Phoneme::new(
    150.0,
    VowelFilter::U.0,
    VowelFilter::U.1,
    VowelFilter::U.2,
    1.2,
    0.0,
    1.0,
    1.0,
    false,
)];
static PHONEME_S: [Phoneme; 1] = [Phoneme::new(
    250.0,
    VowelFilter::S.0,
    VowelFilter::S.1,
    VowelFilter::S.2,
    0.0,
    4.5,
    1.0,
    1.3,
    true,
)];
static PHONEME_Z: [Phoneme; 1] = [Phoneme::new(
    220.0,
    VowelFilter::Z.0,
    VowelFilter::Z.1,
    VowelFilter::Z.2,
    0.6,
    3.5,
    0.9,
    1.2,
    true,
)];
static PHONEME_F: [Phoneme; 1] = [Phoneme::new(
    180.0,
    VowelFilter::F.0,
    VowelFilter::F.1,
    VowelFilter::F.2,
    0.0,
    3.5,
    1.0,
    1.2,
    true,
)];
static PHONEME_V: [Phoneme; 1] = [Phoneme::new(
    160.0,
    VowelFilter::V.0,
    VowelFilter::V.1,
    VowelFilter::V.2,
    0.7,
    2.0,
    0.9,
    1.1,
    true,
)];
static PHONEME_H: [Phoneme; 1] = [Phoneme::new(
    120.0,
    VowelFilter::H.0,
    VowelFilter::H.1,
    VowelFilter::H.2,
    0.0,
    1.8,
    1.0,
    0.8,
    true,
)];
static PHONEME_TH: [Phoneme; 1] = [Phoneme::new(
    180.0,
    VowelFilter::TH.0,
    VowelFilter::TH.1,
    VowelFilter::TH.2,
    0.3,
    2.5,
    1.0,
    1.0,
    true,
)];
static PHONEME_SH: [Phoneme; 1] = [Phoneme::new(
    250.0,
    VowelFilter::SH.0,
    VowelFilter::SH.1,
    VowelFilter::SH.2,
    0.0,
    4.0,
    1.0,
    1.3,
    true,
)];
static PHONEME_R: [Phoneme; 1] = [Phoneme::new(
    140.0,
    VowelFilter::R.0,
    VowelFilter::R.1,
    VowelFilter::R.2,
    1.1,
    0.1,
    0.95,
    1.0,
    false,
)];
static PHONEME_L: [Phoneme; 1] = [Phoneme::new(
    130.0,
    VowelFilter::L.0,
    VowelFilter::L.1,
    VowelFilter::L.2,
    1.0,
    0.0,
    1.0,
    0.9,
    false,
)];
static PHONEME_N: [Phoneme; 1] = [Phoneme::new(
    120.0,
    VowelFilter::N.0,
    VowelFilter::N.1,
    VowelFilter::N.2,
    0.8,
    0.2,
    1.0,
    0.7,
    false,
)];
static PHONEME_M: [Phoneme; 1] = [Phoneme::new(
    120.0,
    VowelFilter::M.0,
    VowelFilter::M.1,
    VowelFilter::M.2,
    0.8,
    0.2,
    1.0,
    0.7,
    false,
)];
static PHONEME_NG: [Phoneme; 1] = [Phoneme::new(
    140.0,
    VowelFilter::NG.0,
    VowelFilter::NG.1,
    VowelFilter::NG.2,
    0.8,
    0.2,
    1.0,
    0.7,
    false,
)];
static PHONEME_W: [Phoneme; 1] = [Phoneme::new(
    130.0,
    VowelFilter::W.0,
    VowelFilter::W.1,
    VowelFilter::W.2,
    1.2,
    0.0,
    0.95,
    1.0,
    false,
)];
static PHONEME_Y: [Phoneme; 1] = [Phoneme::new(
    130.0,
    VowelFilter::Y.0,
    VowelFilter::Y.1,
    VowelFilter::Y.2,
    1.2,
    0.0,
    1.05,
    1.0,
    false,
)];
static PHONEME_EE: [Phoneme; 1] = [Phoneme::new(
    200.0, 300.0, 2300.0, 3200.0, 1.2, 0.0, 1.0, 1.0, false,
)];
static PHONEME_GAP: [Phoneme; 1] = [Phoneme::new(
    150.0, 300.0, 1000.0, 2000.0, 0.0, 0.0, 1.0, 0.0, true,
)];
static PHONEME_CH: [Phoneme; 2] = [
    Phoneme::new(40.0, 300.0, 1000.0, 2000.0, 0.0, 0.0, 1.0, 0.0, true),
    Phoneme::new(
        150.0,
        VowelFilter::SH.0,
        VowelFilter::SH.1,
        VowelFilter::SH.2,
        0.0,
        3.8,
        1.0,
        1.2,
        true,
    ),
];
static PHONEME_J: [Phoneme; 2] = [
    Phoneme::new(40.0, 250.0, 1000.0, 2000.0, 0.5, 0.0, 0.9, 0.6, true),
    Phoneme::new(150.0, 2000.0, 3200.0, 5000.0, 0.4, 3.2, 0.9, 1.1, true),
];
static PHONEME_D: [Phoneme; 2] = [
    Phoneme::new(40.0, 200.0, 1000.0, 2000.0, 0.5, 0.0, 1.0, 0.6, true),
    Phoneme::new(40.0, 350.0, 1800.0, 2700.0, 0.2, 2.5, 1.0, 1.3, true),
];
static PHONEME_B: [Phoneme; 2] = [
    Phoneme::new(40.0, 200.0, 800.0, 1500.0, 0.6, 0.0, 1.0, 0.6, true),
    Phoneme::new(40.0, 300.0, 1100.0, 2200.0, 0.2, 2.5, 1.0, 1.3, true),
];
static PHONEME_P: [Phoneme; 2] = [
    Phoneme::new(30.0, 300.0, 1000.0, 2000.0, 0.0, 0.0, 1.0, 0.0, true),
    Phoneme::new(50.0, 500.0, 1500.0, 2500.0, 0.0, 4.0, 1.0, 1.3, true),
];
static PHONEME_T: [Phoneme; 2] = [
    Phoneme::new(30.0, 300.0, 1000.0, 2000.0, 0.0, 0.0, 1.0, 0.0, true),
    Phoneme::new(50.0, 5000.0, 7000.0, 9000.0, 0.0, 3.5, 1.0, 1.3, true),
];
static PHONEME_K: [Phoneme; 2] = [
    Phoneme::new(30.0, 300.0, 1000.0, 2000.0, 0.0, 0.0, 1.0, 0.0, true),
    Phoneme::new(50.0, 2000.0, 3000.0, 4000.0, 0.0, 3.0, 1.0, 1.3, true),
];
static PHONEME_G: [Phoneme; 2] = [
    Phoneme::new(40.0, 200.0, 1000.0, 2000.0, 0.5, 0.0, 0.9, 0.6, true),
    Phoneme::new(50.0, 1800.0, 2500.0, 3500.0, 0.4, 2.5, 0.9, 1.2, true),
];
static PHONEME_AI: [Phoneme; 2] = [
    Phoneme::new(
        80.0,
        VowelFilter::A.0,
        VowelFilter::A.1,
        VowelFilter::A.2,
        1.2,
        0.0,
        1.0,
        1.0,
        false,
    ),
    Phoneme::new(
        100.0,
        VowelFilter::I.0,
        VowelFilter::I.1,
        VowelFilter::I.2,
        1.2,
        0.0,
        1.05,
        1.0,
        false,
    ),
];

/// A complete Speech Synthesizer component.
///
/// Implements a formant-based vocal synthesizer that can play back sequences of phonemes.
pub struct SpeechSynth<'a> {
    sample_rate: f32,
    time_in_phoneme: f32,
    phonemes: &'a [Phoneme],
    current_idx: usize,
    cur_f1: f32,
    cur_f2: f32,
    cur_f3: f32,
    cur_voice: f32,
    cur_noise: f32,
    cur_pitch: f32,
    cur_amp: f32,

    vowel_filter: VowelFilter,
    stutter: Stutter,
    voice_stack: Stack,
    sub_osc: Oscillator,
    noise_gen: Oscillator,
    just_entered_phoneme: bool,
    is_finished: bool,
}

impl<'a> SpeechSynth<'a> {
    /// Creates a new SpeechSynth.
    pub fn new(sample_rate: f32) -> Self {
        let mut vowel_filter = VowelFilter::new(AudioParam::Static(0.0), AudioParam::Static(18.0));
        vowel_filter.set_sample_rate(sample_rate);

        let mut stutter = Stutter::new(
            1000.0,
            AudioParam::ms(65.0),
            AudioParam::Static(0.0),
            AudioParam::Static(0.0),
        );
        stutter.set_sample_rate(sample_rate);

        let mut voice_stack = Stack::new(
            4,
            AudioParam::hz(110.0),
            Waveform::NaiveSaw,
            AudioParam::Static(0.5),
        );
        voice_stack.set_sample_rate(sample_rate);
        voice_stack.align_phases();

        let mut sub_osc = Oscillator::new(AudioParam::hz(55.0), Waveform::NaiveSaw);
        sub_osc.set_sample_rate(sample_rate);
        sub_osc.set_phase(0.0);

        let mut noise_gen = Oscillator::new(AudioParam::Static(0.0), Waveform::WhiteNoise);
        noise_gen.set_sample_rate(sample_rate);

        Self {
            sample_rate,
            time_in_phoneme: 0.0,
            phonemes: &[],
            current_idx: 0,
            cur_f1: 300.0,
            cur_f2: 1000.0,
            cur_f3: 2000.0,
            cur_voice: 0.0,
            cur_noise: 0.0,
            cur_pitch: 1.0,
            cur_amp: 0.0,
            vowel_filter,
            stutter,
            voice_stack,
            sub_osc,
            noise_gen,
            just_entered_phoneme: false,
            is_finished: true,
        }
    }

    /// Sets the phoneme sequence to be played.
    pub fn set_phonemes(&mut self, phonemes: &'a [Phoneme]) {
        self.phonemes = phonemes;
        self.current_idx = 0;
        self.time_in_phoneme = 0.0;
        self.is_finished = self.phonemes.is_empty();
        self.just_entered_phoneme = !self.is_finished;

        if !self.is_finished {
            let start = &self.phonemes[0];
            self.cur_f1 = start.f1;
            self.cur_f2 = start.f2;
            self.cur_f3 = start.f3;
            self.cur_voice = start.mix_voice;
            self.cur_noise = start.mix_noise;
            self.cur_pitch = start.pitch_mod;
            self.cur_amp = start.amp;
        }
    }

    /// Returns true if the phoneme sequence has finished playing.
    pub fn is_finished(&self) -> bool {
        self.is_finished
    }
}

impl<'a> FrameProcessor<Mono> for SpeechSynth<'a> {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        if self.is_finished {
            buffer.fill(0.0);
            return;
        }

        let dt_ms = 1000.0 / self.sample_rate;

        for (i, sample) in buffer.iter_mut().enumerate() {
            let current_sample_idx = sample_index + i as u64;
            self.time_in_phoneme += dt_ms;

            if self.current_idx < self.phonemes.len() {
                let current_p = &self.phonemes[self.current_idx];
                if self.time_in_phoneme >= current_p.duration_ms {
                    self.time_in_phoneme -= current_p.duration_ms;
                    self.current_idx += 1;

                    if self.current_idx >= self.phonemes.len() {
                        self.is_finished = true;
                        *sample = 0.0;
                        continue;
                    }
                    self.just_entered_phoneme = true;
                }
            }

            if self.is_finished {
                *sample = 0.0;
                continue;
            }

            let t = &self.phonemes[self.current_idx];
            let smooth_freq = if t.jump_freq {
                1.0 - libm::expf(-1.0 / (0.001 * self.sample_rate))
            } else {
                1.0 - libm::expf(-1.0 / (0.015 * self.sample_rate))
            };
            let smooth_amp = 1.0 - libm::expf(-1.0 / (0.008 * self.sample_rate));

            self.cur_f1 += (t.f1 - self.cur_f1) * smooth_freq;
            self.cur_f2 += (t.f2 - self.cur_f2) * smooth_freq;
            self.cur_f3 += (t.f3 - self.cur_f3) * smooth_freq;
            self.cur_voice += (t.mix_voice - self.cur_voice) * smooth_amp;
            self.cur_noise += (t.mix_noise - self.cur_noise) * smooth_amp;
            self.cur_pitch += (t.pitch_mod - self.cur_pitch) * smooth_freq;
            self.cur_amp += (t.amp - self.cur_amp) * smooth_amp;

            let p = 110.0 * self.cur_pitch;

            let voiced_stack = self
                .voice_stack
                .oscillators
                .iter_mut()
                .enumerate()
                .map(|(idx, osc)| {
                    let spread = (idx as f32 / 3.0) * 2.0 - 1.0;
                    osc.tick(p * (1.0 + spread * 0.005))
                })
                .sum::<f32>()
                * 0.25;

            let sub_out = self.sub_osc.tick(p * 0.5);
            let noise_out = self.noise_gen.tick(0.0);

            let voiced = voiced_stack * 1.0 + sub_out * 0.6;
            let exciter = voiced * self.cur_voice + noise_out * self.cur_noise;
            let q = if t.mix_noise > 2.0 { 6.0 } else { 18.0 };

            let synth_out =
                self.vowel_filter
                    .tick_manual(exciter, self.cur_f1, self.cur_f2, self.cur_f3, q)
                    * self.cur_amp;

            if self.just_entered_phoneme && t.glitch_repeats > 0 {
                self.stutter
                    .set_repeats(AudioParam::Static(t.glitch_repeats as f32));
                self.stutter.set_trigger(AudioParam::Static(1.0));
            } else {
                self.stutter.set_trigger(AudioParam::Static(0.0));
            }
            self.just_entered_phoneme = false;

            let mut stutter_buf = [synth_out];
            self.stutter.process(&mut stutter_buf, current_sample_idx);
            *sample = stutter_buf[0];
        }
    }

    fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr;
        self.vowel_filter.set_sample_rate(sr);
        self.stutter.set_sample_rate(sr);
        self.voice_stack.set_sample_rate(sr);
        self.sub_osc.set_sample_rate(sr);
        self.noise_gen.set_sample_rate(sr);
    }

    fn reset(&mut self) {
        self.current_idx = 0;
        self.time_in_phoneme = 0.0;
        self.vowel_filter.reset();
        self.stutter.reset();
        self.voice_stack.reset();
        self.sub_osc.reset();
        self.noise_gen.reset();
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "SpeechSynth"
    }
}
