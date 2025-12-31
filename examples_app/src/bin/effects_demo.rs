use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::dynamics::distortion::{Distortion, DistortionType};
use infinitedsp_core::effects::utility::add::Add;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::multiply::Multiply;
use infinitedsp_core::effects::utility::panner::StereoPanner;
use infinitedsp_core::synthesis::lfo::{Lfo, LfoWaveform};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use std::thread;
use std::time::Duration;

fn create_effects_chain(sample_rate: f32) -> DspChain {
    let osc1 = Oscillator::new(AudioParam::hz(110.0), Waveform::Sine);
    let osc2 = Oscillator::new(AudioParam::hz(220.0), Waveform::Triangle);

    let osc1_param = AudioParam::Dynamic(Box::new(DspChain::new(osc1, sample_rate)));
    let osc2_param = AudioParam::Dynamic(Box::new(DspChain::new(osc2, sample_rate)));

    let combined = Add::new(osc1_param, osc2_param);

    let lfo = Lfo::new(AudioParam::hz(1.0), LfoWaveform::Sine);
    let lfo_param = AudioParam::Dynamic(Box::new(DspChain::new(lfo, sample_rate)));

    let modulated = Multiply::new(
        AudioParam::Dynamic(Box::new(DspChain::new(combined, sample_rate))),
        lfo_param
    );

    let dist = Distortion::new(
        AudioParam::linear(5.0),
        AudioParam::linear(1.0),
        DistortionType::SoftClip
    );

    let pan_lfo = Lfo::new(AudioParam::hz(0.5), LfoWaveform::Sine);
    let pan_param = AudioParam::Dynamic(Box::new(DspChain::new(pan_lfo, sample_rate)));

    let panner = StereoPanner::new(pan_param);

    DspChain::new(modulated, sample_rate)
        .and(dist)
        .and(Gain::new_db(-6.0))
        .and(panner)
}

fn main() -> Result<()> {
    let (stream, sample_rate) = init_audio_interleaved(|sr| create_effects_chain(sr))?;

    println!("Playing Effects Demo (Add, Multiply, Distortion, Panner) at {}Hz...", sample_rate);

    stream.play()?;

    thread::sleep(Duration::from_secs(10));

    Ok(())
}
