use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::parameter::Parameter;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::synthesis::karplus_strong::KarplusStrong;
use infinitedsp_examples::audio_backend::init_audio;
use std::thread;
use std::time::{Duration, Instant};

struct Note {
    start_beat: f32,
    duration_beats: f32,
    freq: f32,
}

fn create_karplus_chain(sample_rate: f32, pitch: Parameter, gate: Parameter) -> DspChain {
    let string = KarplusStrong::new(
        AudioParam::Linked(pitch),
        AudioParam::Linked(gate),
        AudioParam::linear(0.7),
        AudioParam::linear(0.8)
    );

    DspChain::new(string, sample_rate)
        .and(Gain::new_db(-3.0))
}

fn main() -> Result<()> {
    let pitch_param = Parameter::new(440.0);
    let gate_param = Parameter::new(0.0);

    let chain = create_karplus_chain(44100.0, pitch_param.clone(), gate_param.clone());
    println!("Signal Chain:\n{}", chain.get_graph());

    let p = pitch_param.clone();
    let g = gate_param.clone();

    let (stream, _sample_rate) = init_audio(move |sr| {
        create_karplus_chain(sr, p, g)
    })?;

    println!("Playing Karplus-Strong Guitar Demo (Dry)...");
    stream.play()?;

    let e2 = 82.41;
    let a2 = 110.00;
    let d3 = 146.83;
    let g3 = 196.00;
    let b3 = 246.94;
    let e4 = 329.63;

    let melody = vec![
        Note { start_beat: 0.0, duration_beats: 1.0, freq: e2 },
        Note { start_beat: 1.0, duration_beats: 1.0, freq: a2 },
        Note { start_beat: 2.0, duration_beats: 1.0, freq: d3 },
        Note { start_beat: 3.0, duration_beats: 1.0, freq: g3 },
        Note { start_beat: 4.0, duration_beats: 1.0, freq: b3 },
        Note { start_beat: 5.0, duration_beats: 1.0, freq: e4 },

        Note { start_beat: 6.0, duration_beats: 0.5, freq: b3 },
        Note { start_beat: 6.5, duration_beats: 0.5, freq: g3 },
        Note { start_beat: 7.0, duration_beats: 0.5, freq: d3 },
        Note { start_beat: 7.5, duration_beats: 0.5, freq: a2 },
        Note { start_beat: 8.0, duration_beats: 2.0, freq: e2 },
    ];

    let bpm = 100.0;
    let seconds_per_beat = 60.0 / bpm;
    let start_time = Instant::now();
    let total_duration = Duration::from_secs_f32(12.0 * seconds_per_beat);

    while start_time.elapsed() < total_duration {
        let elapsed = start_time.elapsed().as_secs_f32();
        let current_beat = elapsed / seconds_per_beat;

        let mut active_note = None;

        for note in &melody {
            if current_beat >= note.start_beat && current_beat < (note.start_beat + note.duration_beats) {
                if current_beat < (note.start_beat + 0.1) {
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
