use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::Stereo;
use infinitedsp_core::core::frame_processor::FrameProcessor;
use infinitedsp_core::core::ola::Ola;
use infinitedsp_core::core::static_dsp_chain::StaticDspChain;
use infinitedsp_core::effects::dynamics::compressor::Compressor;
use infinitedsp_core::effects::dynamics::distortion::{Distortion, DistortionType};
use infinitedsp_core::effects::spectral::pitch_shift::FftPitchShift;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::stereo_widener::StereoWidener;
use infinitedsp_core::synthesis::speech::{Phoneme, SpeechSynth};
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use std::env;
use std::thread;
use std::time::Duration;

struct TextParser;

impl TextParser {
    fn get_phoneme(token: &str) -> &'static [Phoneme] {
        Phoneme::from_token(token)
    }

    fn parse(text: &str) -> Vec<Phoneme> {
        let mut phonemes = Vec::new();

        phonemes.push(Phoneme::gap(150.0));

        let input = text.to_uppercase();
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        let keys = [
            "TH", "NG", "SH", "CH", "AI", "EE", "GAP", "A", "E", "I", "O", "U", "S", "Z", "F", "V",
            "H", "W", "Y", "N", "M", "R", "L", "J", "D", "B", "P", "T", "K", "G",
        ];

        while i < chars.len() {
            if chars[i].is_whitespace() {
                phonemes.extend_from_slice(Self::get_phoneme("GAP"));
                i += 1;
                continue;
            }

            if chars[i] == '[' {
                let mut token = String::new();
                i += 1;
                while i < chars.len() && chars[i] != ']' {
                    token.push(chars[i]);
                    i += 1;
                }
                i += 1;

                let added = Self::get_phoneme(&token);
                let mut added_vec = added.to_vec();

                if i < chars.len() && chars[i] == '!' {
                    i += 1;
                    let mut num_str = String::new();
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        num_str.push(chars[i]);
                        i += 1;
                    }
                    if let Ok(num) = num_str.parse::<u32>() {
                        if let Some(last) = added_vec.last_mut() {
                            last.glitch_repeats = num;
                        }
                    }
                }
                phonemes.extend(added_vec);
                continue;
            }

            let mut matched = false;
            for key in &keys {
                let key_chars: Vec<char> = key.chars().collect();
                if i + key_chars.len() <= chars.len() && chars[i..i + key_chars.len()] == key_chars
                {
                    let added = Self::get_phoneme(key);
                    let mut added_vec = added.to_vec();
                    i += key_chars.len();

                    if i < chars.len() && chars[i] == '!' {
                        i += 1;
                        let mut num_str = String::new();
                        while i < chars.len() && chars[i].is_ascii_digit() {
                            num_str.push(chars[i]);
                            i += 1;
                        }
                        if let Ok(num) = num_str.parse::<u32>() {
                            if let Some(last) = added_vec.last_mut() {
                                last.glitch_repeats = num;
                            }
                        }
                    }
                    phonemes.extend(added_vec);
                    matched = true;
                    break;
                }
            }

            if !matched {
                i += 1;
            }
        }

        phonemes.push(Phoneme::gap(800.0));
        phonemes
    }
}

fn create_speech_chain(
    sr: f32,
    phonemes: &'static [Phoneme],
) -> Box<dyn FrameProcessor<Stereo> + Send> {
    let mut synth = SpeechSynth::new(sr);
    synth.set_phonemes(phonemes);

    let spectral_shifter: Ola<FftPitchShift<1024>, 1024> =
        Ola::with(FftPitchShift::<1024>::new(AudioParam::Static(-7.0)));

    let saturation = Distortion::new(
        AudioParam::Static(3.5),
        AudioParam::Static(0.4),
        DistortionType::SoftClip,
    );

    let mono_chain = StaticDspChain::new(synth, sr)
        .and(spectral_shifter)
        .and(saturation)
        .and(Compressor::new_limiter());

    let stereo_chain = mono_chain
        .to_stereo()
        .and(StereoWidener::new(AudioParam::Static(1.6)))
        .and_mix(
            0.1,
            Reverb::new_with_params(AudioParam::Static(0.85), AudioParam::Static(0.7), 0),
        )
        .and(Gain::new_fixed(1.0));

    Box::new(stereo_chain)
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let input = if args.len() > 1 {
        args[1..].join(" ")
    } else {
        "SISTEM ONLAIN!3   INFINIT DIESPI PAUER".to_string()
    };

    let phonemes_vec = TextParser::parse(&input);
    let phonemes: &'static [Phoneme] = Box::leak(phonemes_vec.into_boxed_slice());

    let mut total_duration_ms: f32 = 0.0;
    for p in phonemes {
        total_duration_ms += p.duration_ms;
        if p.glitch_repeats > 0 {
            total_duration_ms += p.glitch_repeats as f32 * 65.0;
        }
    }

    println!("Parsing Phonetics: '{}'", input);
    println!("Now playing...");

    let (stream, _sr) = init_audio_interleaved(|sr| create_speech_chain(sr, phonemes))?;
    stream.play()?;

    thread::sleep(Duration::from_millis(total_duration_ms as u64 + 1500));

    Ok(())
}
