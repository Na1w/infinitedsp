use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::frame_processor::FrameProcessor;
use infinitedsp_core::core::parameter::Parameter;
use infinitedsp_core::effects::dynamics::compressor::Compressor;
use infinitedsp_core::effects::filter::ladder_filter::LadderFilter;
use infinitedsp_core::effects::time::delay::Delay;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::offset::Offset;
use infinitedsp_core::synthesis::envelope::Adsr;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::{init_audio_stereo, StereoProcessor};
use std::thread;
use std::time::{Duration, Instant};

struct Note {
    start_beat: f32,
    duration_beats: f32,
    freq: f32,
}

fn d_freq(g: f32) -> f32 {
    g * 1.4983
}

fn create_trance_voice(
    sample_rate: f32,
    pitch: Parameter,
    gate: Parameter,
    delay_time_s: f32,
) -> DspChain {
    let osc = Oscillator::new(AudioParam::Linked(pitch.clone()), Waveform::Saw);
    let noise = Oscillator::new(AudioParam::Static(0.0), Waveform::WhiteNoise);

    let filter_env = Adsr::new(
        AudioParam::Linked(gate.clone()),
        AudioParam::ms(10.0),
        AudioParam::ms(150.0),
        AudioParam::linear(0.0),
        AudioParam::ms(100.0),
    );

    let cutoff_mod = DspChain::new(filter_env, sample_rate)
        .and(Gain::new_fixed(5000.0))
        .and(Offset::new(100.0));

    let filter = LadderFilter::new(
        AudioParam::Dynamic(Box::new(cutoff_mod)),
        AudioParam::Static(0.4),
    );

    let amp_env = Adsr::new(
        AudioParam::Linked(gate),
        AudioParam::ms(5.0),
        AudioParam::ms(100.0),
        AudioParam::linear(0.8),
        AudioParam::ms(100.0),
    );

    let vca = Gain::new(AudioParam::Dynamic(Box::new(amp_env)));

    let delay = Delay::new(
        1.0,
        AudioParam::seconds(delay_time_s),
        AudioParam::linear(0.5),
        AudioParam::linear(1.0),
    );

    DspChain::new(osc, sample_rate)
        .and_mix(0.15, noise)
        .and(filter)
        .and(vca)
        .and_mix(0.5, delay)
        .and(Gain::new_db(-3.0))
}

fn create_riser_voice(sample_rate: f32, cutoff: Parameter, gain: Parameter) -> DspChain {
    let noise = Oscillator::new(AudioParam::Static(0.0), Waveform::WhiteNoise);

    let filter = LadderFilter::new(AudioParam::Linked(cutoff), AudioParam::Static(0.7));

    DspChain::new(noise, sample_rate)
        .and(filter)
        .and(Gain::new(AudioParam::Linked(gain)))
        .and(Gain::new_db(-12.0))
}

struct StereoEngine {
    left_voice: DspChain,
    right_voice: DspChain,
    riser_voice: DspChain,
    master_filter_l: LadderFilter,
    master_filter_r: LadderFilter,
    reverb_l: Reverb,
    reverb_r: Reverb,
    master_comp_l: Compressor,
    master_comp_r: Compressor,

    riser_buffer: Vec<f32>,
    reverb_buf_l: Vec<f32>,
    reverb_buf_r: Vec<f32>,
}

impl StereoProcessor for StereoEngine {
    fn process(&mut self, left: &mut [f32], right: &mut [f32], sample_index: u64) {
        let len = left.len();
        if self.riser_buffer.len() < len {
            self.riser_buffer.resize(len, 0.0);
        }
        if self.reverb_buf_l.len() < len {
            self.reverb_buf_l.resize(len, 0.0);
        }
        if self.reverb_buf_r.len() < len {
            self.reverb_buf_r.resize(len, 0.0);
        }

        self.left_voice.process(left, sample_index);
        self.right_voice.process(right, sample_index);

        self.riser_buffer.fill(0.0);
        self.riser_voice
            .process(&mut self.riser_buffer[0..len], sample_index);

        for i in 0..len {
            let r = self.riser_buffer[i] * 0.5;
            left[i] += r;
            right[i] += r;
        }

        self.master_filter_l.process(left, sample_index);
        self.master_filter_r.process(right, sample_index);

        self.reverb_buf_l[0..len].copy_from_slice(left);
        self.reverb_buf_r[0..len].copy_from_slice(right);

        self.reverb_l
            .process(&mut self.reverb_buf_l[0..len], sample_index);
        self.reverb_r
            .process(&mut self.reverb_buf_r[0..len], sample_index);

        for i in 0..len {
            let wet_l = self.reverb_buf_l[i] * 0.2;
            let wet_r = self.reverb_buf_r[i] * 0.2;
            left[i] += wet_l;
            right[i] += wet_r;
        }

        self.master_comp_l.process(left, sample_index);
        self.master_comp_r.process(right, sample_index);
    }
}

fn main() -> Result<()> {
    let pitch_param = Parameter::new(440.0);
    let pitch_param_r = Parameter::new(440.0);
    let gate_param = Parameter::new(0.0);
    let master_cutoff = Parameter::new(100.0);
    let riser_cutoff = Parameter::new(100.0);
    let riser_gain = Parameter::new(1.0);

    let bpm = 138.0;
    let beat_sec = 60.0 / bpm;
    let delay_l = beat_sec * 0.75;
    let delay_r = beat_sec * 1.0;

    let voice_chain =
        create_trance_voice(44100.0, pitch_param.clone(), gate_param.clone(), delay_l);
    println!("Trance Voice Chain:\n{}", voice_chain.get_graph());

    let p_l = pitch_param.clone();
    let p_r = pitch_param_r.clone();
    let g = gate_param.clone();
    let mc = master_cutoff.clone();
    let rc = riser_cutoff.clone();
    let rg = riser_gain.clone();

    let (stream, sample_rate) = init_audio_stereo(move |sr| StereoEngine {
        left_voice: create_trance_voice(sr, p_l.clone(), g.clone(), delay_l),
        right_voice: create_trance_voice(sr, p_r.clone(), g.clone(), delay_r),
        riser_voice: create_riser_voice(sr, rc.clone(), rg.clone()),
        master_filter_l: LadderFilter::new(AudioParam::Linked(mc.clone()), AudioParam::Static(0.2)),
        master_filter_r: LadderFilter::new(AudioParam::Linked(mc.clone()), AudioParam::Static(0.2)),
        reverb_l: Reverb::new(),
        reverb_r: Reverb::new_with_seed(1),
        master_comp_l: Compressor::new_limiter(),
        master_comp_r: Compressor::new_limiter(),
        riser_buffer: Vec::new(),
        reverb_buf_l: Vec::new(),
        reverb_buf_r: Vec::new(),
    })?;

    println!(
        "Playing Massive Trance Arp at {}Hz (Run with --release)...",
        sample_rate
    );
    stream.play()?;

    let a1 = 55.00;
    let c2 = 65.41;
    let f2 = 87.31;
    let g2 = 98.00;
    let c3 = 130.81;
    let e3 = 164.81;

    let mut melody = Vec::new();
    let mut beat = 0.0;

    let mut add_arp = |root: f32, fifth: f32, transpose: f32| {
        let r = root * transpose;
        let f = fifth * transpose;
        let root2 = r * 2.0;
        let fifth2 = f * 2.0;
        let root3 = r * 4.0;

        for _ in 0..2 {
            melody.push(Note {
                start_beat: beat + 0.00,
                duration_beats: 0.25,
                freq: r,
            });
            melody.push(Note {
                start_beat: beat + 0.25,
                duration_beats: 0.25,
                freq: f,
            });
            melody.push(Note {
                start_beat: beat + 0.50,
                duration_beats: 0.25,
                freq: root2,
            });
            melody.push(Note {
                start_beat: beat + 0.75,
                duration_beats: 0.25,
                freq: fifth2,
            });

            melody.push(Note {
                start_beat: beat + 1.00,
                duration_beats: 0.25,
                freq: root3,
            });
            melody.push(Note {
                start_beat: beat + 1.25,
                duration_beats: 0.25,
                freq: fifth2,
            });
            melody.push(Note {
                start_beat: beat + 1.50,
                duration_beats: 0.25,
                freq: root2,
            });
            melody.push(Note {
                start_beat: beat + 1.75,
                duration_beats: 0.25,
                freq: f,
            });

            beat += 2.0;
        }
    };

    for _ in 0..2 {
        add_arp(a1, e3 / 2.0, 1.0);
        add_arp(f2, c3, 1.0);
        add_arp(c2, g2, 1.0);
        add_arp(g2, d_freq(g2), 1.0);
    }

    add_arp(a1, e3 / 2.0, 2.0);
    add_arp(f2, c3, 2.0);
    add_arp(c2, g2, 2.0);
    add_arp(g2, d_freq(g2), 2.0);

    add_arp(a1, e3 / 2.0, 4.0);
    add_arp(f2, c3, 4.0);
    add_arp(c2, g2, 4.0);
    add_arp(g2, d_freq(g2), 4.0);

    let total_beats = beat + 8.0;
    let start_time = Instant::now();
    let seconds_per_beat = 60.0 / bpm;
    let total_duration = Duration::from_secs_f32(total_beats * seconds_per_beat);

    while start_time.elapsed() < total_duration {
        let elapsed = start_time.elapsed().as_secs_f32();
        let current_beat = elapsed / seconds_per_beat;
        let progress = elapsed / total_duration.as_secs_f32();

        let cutoff_val = 50.0 * (300.0f32).powf(progress);
        master_cutoff.set(cutoff_val);

        let riser_val = 100.0 * (150.0f32).powf(progress);
        riser_cutoff.set(riser_val);

        if current_beat >= (beat - 4.0) {
            riser_gain.set(0.0);
        } else {
            riser_gain.set(1.0);
        }

        let mut active_note = None;
        for note in &melody {
            if current_beat >= note.start_beat
                && current_beat < (note.start_beat + note.duration_beats)
            {
                if current_beat < (note.start_beat + note.duration_beats - 0.05) {
                    active_note = Some(note);
                }
            }
        }

        if let Some(note) = active_note {
            if gate_param.get() == 0.0 || (pitch_param.get() - note.freq).abs() > 0.1 {
                pitch_param.set(note.freq);
                pitch_param_r.set(note.freq * 1.002);
                gate_param.set(1.0);
            }
        } else {
            if gate_param.get() == 1.0 {
                gate_param.set(0.0);
            }
        }

        thread::sleep(Duration::from_millis(2));
    }

    Ok(())
}
