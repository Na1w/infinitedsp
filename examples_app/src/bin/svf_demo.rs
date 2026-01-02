use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::filter::state_variable::{StateVariableFilter, SvfType};
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::offset::Offset;
use infinitedsp_core::synthesis::lfo::{Lfo, LfoWaveform};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio;
use std::thread;
use std::time::Duration;

fn create_svf_chain(sample_rate: f32) -> DspChain {
    let osc = Oscillator::new(AudioParam::hz(110.0), Waveform::Saw);

    let lfo = Lfo::new(AudioParam::hz(0.5), LfoWaveform::Sine);

    let lfo_scaled = DspChain::new(lfo, sample_rate)
        .and(Gain::new_fixed(1000.0))
        .and(Offset::new(1200.0));

    let cutoff_param = AudioParam::Dynamic(Box::new(lfo_scaled));

    let filter = StateVariableFilter::new(SvfType::BandPass, cutoff_param, AudioParam::linear(0.8));

    DspChain::new(osc, sample_rate)
        .and(filter)
        .and(Gain::new_db(-3.0))
}

fn main() -> Result<()> {
    let chain = create_svf_chain(44100.0);
    println!("Signal Chain:\n{}", chain.get_graph());

    let (stream, sample_rate) = init_audio(create_svf_chain)?;

    println!(
        "Playing State Variable Filter Demo (BandPass Sweep) at {}Hz...",
        sample_rate
    );

    stream.play()?;

    thread::sleep(Duration::from_secs(10));

    Ok(())
}
