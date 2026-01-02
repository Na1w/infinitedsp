use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::frame_processor::FrameProcessor;
use infinitedsp_core::effects::filter::ladder_filter::LadderFilter;
use infinitedsp_core::effects::filter::predictive_ladder::PredictiveLadderFilter;
use infinitedsp_core::effects::time::delay::Delay;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::offset::Offset;
use infinitedsp_core::synthesis::lfo::{Lfo, LfoWaveform};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio;
use std::thread;
use std::time::Duration;

fn create_cutoff_mod(sample_rate: f32) -> AudioParam {
    let mut lfo = Lfo::new(AudioParam::hz(0.2), LfoWaveform::Sine);
    lfo.set_unipolar(true);

    let min_freq = 100.0;
    let max_freq = 18000.0;
    let freq_range = max_freq - min_freq;

    let mod_chain = DspChain::new(lfo, sample_rate)
        .and(Gain::new_fixed(freq_range))
        .and(Offset::new(min_freq));

    AudioParam::Dynamic(Box::new(mod_chain))
}

fn create_common_chain(sample_rate: f32, filter: impl FrameProcessor + Send + 'static) -> DspChain {
    let osc = Oscillator::new(AudioParam::hz(60.0), Waveform::Saw);

    let delay = Delay::new(
        1.0,
        AudioParam::ms(350.0),
        AudioParam::linear(0.4),
        AudioParam::linear(0.3),
    );
    let gain = Gain::new_db(-6.0);

    DspChain::new(osc, sample_rate)
        .and(filter)
        .and(delay)
        .and(gain)
}

fn create_predictive_chain(sample_rate: f32) -> DspChain {
    let cutoff_param = create_cutoff_mod(sample_rate);
    let filter = PredictiveLadderFilter::new(cutoff_param, AudioParam::linear(0.9));
    create_common_chain(sample_rate, filter)
}

fn create_standard_chain(sample_rate: f32) -> DspChain {
    let cutoff_param = create_cutoff_mod(sample_rate);
    let filter = LadderFilter::new(cutoff_param, AudioParam::linear(0.9));
    create_common_chain(sample_rate, filter)
}

fn run_demo(name: &str, factory: impl Fn(f32) -> DspChain + Send + 'static) -> Result<()> {
    let chain = factory(44100.0);
    println!("\n--- {} ---", name);
    println!("Signal Chain:\n{}", chain.get_graph());

    let (stream, sample_rate) = init_audio(factory)?;
    println!("Playing {} at {}Hz...", name, sample_rate);
    stream.play()?;
    thread::sleep(Duration::from_secs(10));
    Ok(())
}

fn main() -> Result<()> {
    println!("Comparing PredictiveLadderFilter vs LadderFilter (Newton-Raphson)");

    run_demo("PredictiveLadderFilter (Fast)", create_predictive_chain)?;
    run_demo("LadderFilter (Iterative - Slow)", create_standard_chain)?;

    Ok(())
}
