use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::{Mono, Stereo};
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::frame_processor::FrameProcessor;
use infinitedsp_core::core::parallel_mixer::ParallelMixer;
use infinitedsp_core::core::parameter::Parameter;
use infinitedsp_core::effects::dynamics::compressor::Compressor;
use infinitedsp_core::effects::filter::ladder_filter::LadderFilter;
use infinitedsp_core::effects::modulation::modulated_delay::ModulatedDelay;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::effects::time::tape_delay::TapeDelay;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::map_range::{CurveType, MapRange};
use infinitedsp_core::synthesis::envelope::Adsr;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::{init_audio_stereo, StereoProcessor};
use std::thread;
use std::time::{Duration, Instant};

struct PitchDetune {
    pitch: Parameter,
    ratio: f32,
}
impl FrameProcessor<Mono> for PitchDetune {
    fn process(&mut self, buffer: &mut [f32], _idx: u64) {
        let f = self.pitch.get() * self.ratio;
        buffer.fill(f);
    }
    fn set_sample_rate(&mut self, _: f32) {}
}

struct CyberCity {
    bass: DspChain<Mono>,
    lead: DspChain<Mono>,
    reverb: ParallelMixer<Reverb, Stereo>,
    limiter: Compressor,
    mix_buffer: Vec<f32>,
    stereo_acc: Vec<f32>,
}

impl StereoProcessor for CyberCity {
    fn process(&mut self, left: &mut [f32], right: &mut [f32], sample_index: u64) {
        let len = left.len();
        if self.mix_buffer.len() < len {
            self.mix_buffer.resize(len, 0.0);
            self.stereo_acc.resize(len * 2, 0.0);
        }
        self.stereo_acc.fill(0.0);

        self.mix_buffer.fill(0.0);
        self.bass.process(&mut self.mix_buffer, sample_index);
        for i in 0..len {
            self.stereo_acc[2*i] += self.mix_buffer[i];
            self.stereo_acc[2*i+1] += self.mix_buffer[i];
        }

        self.mix_buffer.fill(0.0);
        self.lead.process(&mut self.mix_buffer, sample_index);
        for i in 0..len {
            self.stereo_acc[2*i] += self.mix_buffer[i];
            self.stereo_acc[2*i+1] += self.mix_buffer[i];
        }

        self.reverb.process(&mut self.stereo_acc, sample_index);

        for i in 0..len {
            left[i] = self.stereo_acc[2*i];
            right[i] = self.stereo_acc[2*i+1];
        }

        self.limiter.process(left, sample_index);
        self.limiter.process(right, sample_index);
    }
}

fn create_bass(sr: f32, pitch: Parameter, gate: Parameter) -> DspChain<Mono> {
    let osc = Oscillator::new(AudioParam::Linked(pitch), Waveform::Saw);
    
    let amp_env = Adsr::new(
        AudioParam::Linked(gate.clone()),
        AudioParam::Static(0.005),
        AudioParam::Static(0.15),
        AudioParam::Static(0.0),
        AudioParam::Static(0.05)
    );

    let filter_env = Adsr::new(
        AudioParam::Linked(gate),
        AudioParam::Static(0.005),
        AudioParam::Static(0.12),
        AudioParam::Static(0.0),
        AudioParam::Static(0.05)
    );

    let mapped_filter_env = MapRange::new(
        AudioParam::Dynamic(Box::new(filter_env)),
        AudioParam::Static(100.0),
        AudioParam::Static(2500.0),
        CurveType::Exponential
    );

    let filter = LadderFilter::new(
        AudioParam::Dynamic(Box::new(mapped_filter_env)),
        AudioParam::Static(0.3)
    );

    let amp = Gain::new(AudioParam::Dynamic(Box::new(amp_env)));

    DspChain::new(osc, sr).and(filter).and(amp).and(Gain::new_db(2.0))
}

fn create_lead(sr: f32, pitch: Parameter, gate: Parameter) -> DspChain<Mono> {
    let osc1 = Oscillator::new(AudioParam::Linked(pitch.clone()), Waveform::Saw);
    let osc2 = Oscillator::new(AudioParam::Dynamic(Box::new(PitchDetune { pitch: pitch.clone(), ratio: 1.006 })), Waveform::Saw);
    let osc3 = Oscillator::new(AudioParam::Dynamic(Box::new(PitchDetune { pitch, ratio: 0.994 })), Waveform::Saw);

    let amp_env = Adsr::new(
        AudioParam::Linked(gate),
        AudioParam::Static(0.05),
        AudioParam::Static(0.0),
        AudioParam::Static(1.0),
        AudioParam::Static(0.4)
    );

    let filter = LadderFilter::new(AudioParam::Static(3000.0), AudioParam::Static(0.1));
    let amp = Gain::new(AudioParam::Dynamic(Box::new(amp_env)));
    let chorus = ModulatedDelay::new_chorus();
    
    let delay = TapeDelay::new(
        2.0, AudioParam::Static(0.5), AudioParam::Static(0.4), AudioParam::Static(0.35)
    );

    DspChain::new(osc1, sr)
        .and_mix(0.8, osc2)
        .and_mix(0.8, osc3)
        .and(filter)
        .and(amp)
        .and(chorus)
        .and(delay)
        .and(Gain::new_db(-8.0))
}

fn main() -> Result<()> {
    let bass_pitch = Parameter::new(41.2);
    let bass_gate = Parameter::new(0.0);
    let lead_pitch = Parameter::new(164.81);
    let lead_gate = Parameter::new(0.0);

    let bp_clone = bass_pitch.clone();
    let bg_clone = bass_gate.clone();
    let lp_clone = lead_pitch.clone();
    let lg_clone = lead_gate.clone();

    let (stream, _sr) = init_audio_stereo(move |sr| {
        let bass = create_bass(sr, bp_clone, bg_clone);
        let lead = create_lead(sr, lp_clone, lg_clone);
        let reverb_unit = Reverb::new_with_params(AudioParam::Static(0.9), AudioParam::Static(0.3), 101);
        
        CyberCity {
            bass,
            lead,
            reverb: ParallelMixer::new(0.3, reverb_unit),
            limiter: Compressor::new_limiter(),
            mix_buffer: Vec::new(),
            stereo_acc: Vec::new(),
        }
    })?;

    stream.play()?;

    let bass_notes = [
        41.20, 41.20, 41.20, 41.20,  41.20, 41.20, 41.20, 49.00,
        41.20, 41.20, 41.20, 41.20,  55.00, 55.00, 49.00, 49.00,
        32.70, 32.70, 32.70, 32.70,  32.70, 32.70, 32.70, 38.89,
        32.70, 32.70, 32.70, 32.70,  38.89, 38.89, 41.20, 41.20,
    ];

    let lead_notes = [
        164.81, 164.81, 164.81, 164.81,  164.81, 164.81, 164.81, 164.81,
        196.00, 196.00, 196.00, 196.00,  196.00, 196.00, 246.94, 246.94,
        130.81, 130.81, 130.81, 130.81,  130.81, 130.81, 130.81, 130.81,
        146.83, 146.83, 146.83, 146.83,  146.83, 146.83, 164.81, 164.81,
    ];

    let lead_gates = [
        1.0, 1.0, 1.0, 1.0,  1.0, 1.0, 0.0, 0.0,
        1.0, 1.0, 1.0, 1.0,  0.0, 0.0, 1.0, 1.0,
        1.0, 1.0, 1.0, 1.0,  1.0, 1.0, 0.0, 0.0,
        1.0, 1.0, 1.0, 1.0,  0.0, 0.0, 1.0, 1.0,
    ];

    let start = Instant::now();
    let mut step = 0;
    
    while start.elapsed() < Duration::from_secs(15) {
        bass_pitch.set(bass_notes[step]);
        lead_pitch.set(lead_notes[step]);
        lead_gate.set(lead_gates[step]);
        
        bass_gate.set(1.0);
        thread::sleep(Duration::from_millis(60));
        bass_gate.set(0.0);
        thread::sleep(Duration::from_millis(65));

        step = (step + 1) % 32;
    }

    Ok(())
}
