use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::parameter::Parameter;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::effects::utility::dc_source::DcSource;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::offset::Offset;
use infinitedsp_core::synthesis::envelope::Adsr;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio;
use std::thread;
use std::time::{Duration, Instant};

struct Note {
    start_beat: f32,
    duration_beats: f32,
    freq: f32,
}

fn create_fm_brass(sample_rate: f32, pitch: Parameter, gate: Parameter) -> DspChain {
    let mod_freq_source = DcSource::new(AudioParam::Linked(pitch.clone()));

    let mod_env = Adsr::new(
        AudioParam::Linked(gate.clone()),
        AudioParam::ms(60.0),
        AudioParam::ms(100.0),
        AudioParam::linear(0.6),
        AudioParam::ms(100.0),
    );

    let mod_signal_chain = DspChain::new(
        Oscillator::new(AudioParam::Dynamic(Box::new(mod_freq_source)), Waveform::Sine),
        sample_rate,
    )
    .and(Gain::new(AudioParam::Dynamic(Box::new(mod_env))))
    .and(Gain::new_fixed(200.0));

    let carrier_base_freq = DcSource::new(AudioParam::Linked(pitch.clone()));

    let carrier_freq_mod = DspChain::new(carrier_base_freq, sample_rate)
        .and(Offset::new_param(AudioParam::Dynamic(Box::new(mod_signal_chain))));

    let carrier_freq_param = AudioParam::Dynamic(Box::new(carrier_freq_mod));

    let carrier = Oscillator::new(carrier_freq_param, Waveform::Sine);

    let amp_env = Adsr::new(
        AudioParam::Linked(gate),
        AudioParam::ms(40.0),
        AudioParam::ms(100.0),
        AudioParam::linear(0.9),
        AudioParam::ms(100.0),
    );

    let vca = Gain::new(AudioParam::Dynamic(Box::new(amp_env)));

    let reverb = Reverb::new(AudioParam::linear(0.015));

    DspChain::new(carrier, sample_rate)
        .and(vca)
        .and(Gain::new_db(-3.0))
        .and_mix(0.2, reverb)
}

fn main() -> Result<()> {
    let pitch_param = Parameter::new(440.0);
    let gate_param = Parameter::new(0.0);

    let pitch_clone = pitch_param.clone();
    let gate_clone = gate_param.clone();

    let (stream, _sample_rate) = init_audio(move |sr| {
        create_fm_brass(sr, pitch_clone, gate_clone)
    })?;

    stream.play()?;
    println!("Playing FM Brass Melody (Run with --release)...");

    let bpm = 100.0;
    let seconds_per_beat = 60.0 / bpm;

    let c3 = 130.81;
    let e3 = 164.81;
    let g3 = 196.00;
    let c4 = 261.63;

    let melody = vec![
        Note { start_beat: 0.0, duration_beats: 1.0, freq: c3 },
        Note { start_beat: 1.0, duration_beats: 1.0, freq: e3 },
        Note { start_beat: 2.0, duration_beats: 1.0, freq: g3 },
        Note { start_beat: 3.0, duration_beats: 1.0, freq: c4 },
        Note { start_beat: 4.0, duration_beats: 2.0, freq: g3 },
    ];

    let start_time = Instant::now();
    let total_duration = Duration::from_secs_f32(8.0 * seconds_per_beat);

    while start_time.elapsed() < total_duration {
        let elapsed = start_time.elapsed().as_secs_f32();
        let current_beat = elapsed / seconds_per_beat;

        let mut active_note = None;

        for note in &melody {
            if current_beat >= note.start_beat && current_beat < (note.start_beat + note.duration_beats) {
                if current_beat < (note.start_beat + note.duration_beats - 0.1) {
                    active_note = Some(note);
                }
            }
        }

        if let Some(note) = active_note {
            if gate_param.get() == 0.0 || (pitch_param.get() - note.freq).abs() > 0.1 {
                pitch_param.set(note.freq);
                gate_param.set(1.0);
            }
        } else {
            if gate_param.get() == 1.0 {
                gate_param.set(0.0);
            }
        }

        thread::sleep(Duration::from_millis(5));
    }

    Ok(())
}
