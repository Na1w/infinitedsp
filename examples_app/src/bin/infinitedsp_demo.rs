use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::DualMono;
use infinitedsp_core::core::channels::Stereo;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::frame_processor::FrameProcessor;
use infinitedsp_core::core::summing_mixer::SummingMixer;
use infinitedsp_core::effects::dynamics::compressor::Compressor;
use infinitedsp_core::effects::filter::state_variable::{StateVariableFilter, SvfType};
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::gate::TimedGate;
use infinitedsp_core::effects::utility::map_range::{CurveType, MapRange};
use infinitedsp_core::effects::utility::panner::StereoPanner;
use infinitedsp_core::effects::utility::stereo_widener::StereoWidener;
use infinitedsp_core::synthesis::envelope::Adsr;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use std::thread;
use std::time::Duration;

struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    fn new(seed: u32) -> Self {
        SimpleRng { state: seed }
    }

    fn next_float(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        let val = (self.state >> 16) & 0x7FFF;
        val as f32 / 32768.0
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_float()
    }
}

struct VoiceConfig {
    start_freq: f32,
    end_freq: f32,
    pan: f32,
    attack_time: f32,
    sample_rate: f32,
}

fn create_envelope(
    gate: AudioParam,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    sample_rate: f32,
) -> AudioParam {
    let env = Adsr::new(
        gate,
        AudioParam::seconds(attack),
        AudioParam::seconds(decay),
        AudioParam::linear(sustain),
        AudioParam::seconds(release),
    );
    AudioParam::Dynamic(Box::new(DspChain::new(env, sample_rate)))
}

fn create_voice(config: VoiceConfig) -> Box<dyn FrameProcessor<Stereo> + Send> {
    let pitch_param = create_envelope(
        AudioParam::Static(1.0),
        config.attack_time,
        0.1,
        1.0,
        0.1,
        config.sample_rate,
    );

    let sweep = MapRange::new(
        pitch_param,
        AudioParam::Static(config.start_freq),
        AudioParam::Static(config.end_freq),
        CurveType::Exponential,
    );

    let osc = Oscillator::new(AudioParam::Dynamic(Box::new(sweep)), Waveform::Saw);

    let filter_param = create_envelope(
        AudioParam::Static(1.0),
        config.attack_time + 1.0,
        0.1,
        1.0,
        0.1,
        config.sample_rate,
    );

    let cutoff_sweep = MapRange::new(
        filter_param,
        AudioParam::hz(200.0),
        AudioParam::hz(15000.0),
        CurveType::Exponential,
    );

    let filter = StateVariableFilter::new(
        SvfType::LowPass,
        AudioParam::Dynamic(Box::new(cutoff_sweep)),
        AudioParam::linear(0.1),
    );

    let gate_proc = TimedGate::new(12.0, config.sample_rate);
    let amp_gate = AudioParam::Dynamic(Box::new(gate_proc));

    let amp_param = create_envelope(amp_gate, 0.5, 0.1, 0.04, 2.5, config.sample_rate);

    let panner = StereoPanner::new(AudioParam::Static(config.pan));
    let gain = Gain::new(amp_param);

    Box::new(
        DspChain::new(osc, config.sample_rate)
            .and(filter)
            .to_stereo()
            .and(panner)
            .and(gain),
    )
}

fn create_thx_chain(sample_rate: f32) -> DspChain<Stereo> {
    let mut rng = SimpleRng::new(12345);
    let mut voices = Vec::new();

    let target_notes = [
        36.71, 36.71, 73.42, 73.42, 110.00, 110.00, 146.83, 146.83, 185.00, 220.00, 220.00, 293.66,
        293.66, 369.99, 440.00, 440.00, 587.33, 739.99, 880.00, 1174.66, 1479.98, 1760.00, 2349.32,
        2960.00, 3520.00, 4698.63, 5919.91, 7040.00, 9397.27, 11839.82,
    ];

    for &end_freq in target_notes.iter() {
        let is_bass = end_freq < 100.0;

        let start_freq = if is_bass {
            rng.range(40.0, 80.0)
        } else {
            rng.range(200.0, 400.0)
        };

        let pan = if is_bass {
            rng.range(-0.2, 0.2)
        } else {
            rng.range(-1.0, 1.0)
        };

        let detune = rng.range(0.995, 1.005);

        voices.push(create_voice(VoiceConfig {
            start_freq,
            end_freq: end_freq * detune,
            pan,
            attack_time: rng.range(6.0, 9.0),
            sample_rate,
        }));
    }

    let summed = SummingMixer::new(voices)
        .with_gain(AudioParam::linear(1.2))
        .with_soft_clip(true);

    let limiter_l = Compressor::new_limiter();
    let limiter_r = Compressor::new_limiter();
    let stereo_limiter = DualMono::new(limiter_l, limiter_r);

    DspChain::new(summed, sample_rate)
        .and(stereo_limiter)
        .and(StereoWidener::new(AudioParam::Static(1.5)))
        .and_mix(
            0.5,
            Reverb::new_with_params(AudioParam::Static(0.9), AudioParam::Static(0.4), 0),
        )
}

fn main() -> Result<()> {
    let chain = create_thx_chain(44100.0);
    println!("Signal Chain:\n{}", chain.get_graph());

    let (stream, sample_rate) = init_audio_interleaved(|sr| create_thx_chain(sr))?;

    println!("Playing InfiniteDSP Demo at {}Hz...", sample_rate);
    println!("Wait for the drop...");

    stream.play()?;

    thread::sleep(Duration::from_secs(16));

    Ok(())
}
