use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::Stereo;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::gate::TimedGate;
use infinitedsp_core::low_mem::effects::time::reverb_low_mem::ReverbLowMem;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use std::thread;
use std::time::Duration;

fn create_noise_source(sample_rate: f32) -> DspChain<Stereo> {
    let noise = Oscillator::new(AudioParam::hz(0.0), Waveform::WhiteNoise);

    let mut gate = TimedGate::new(0.5, sample_rate);
    gate.trigger();

    let gate_gain = Gain::new(AudioParam::Dynamic(Box::new(gate)));
    DspChain::new(noise, sample_rate).and(gate_gain).to_stereo()
}

const REVERB_SIZE: f32 = 1.0;
const REVERB_DAMPING: f32 = 0.4;
const REVERB_SEED: usize = 0;

fn create_standard_reverb_chain(sample_rate: f32) -> DspChain<Stereo> {
    let source = create_noise_source(sample_rate);

    let reverb = Reverb::new_with_params(
        AudioParam::Static(REVERB_SIZE),
        AudioParam::Static(REVERB_DAMPING),
        REVERB_SEED,
    );

    source.and(reverb).and(Gain::new_db(-3.0))
}

fn create_low_mem_reverb_chain(sample_rate: f32) -> DspChain<Stereo> {
    let source = create_noise_source(sample_rate);

    let reverb = ReverbLowMem::new_with_params(
        AudioParam::Static(REVERB_SIZE),
        AudioParam::Static(REVERB_DAMPING),
        REVERB_SEED,
    );

    source.and(reverb).and(Gain::new_db(-3.0))
}

fn run_demo(name: &str, factory: impl Fn(f32) -> DspChain<Stereo> + Send + 'static) -> Result<()> {
    let chain = factory(44100.0);
    println!("\n--- {} ---", name);
    println!("Signal Chain:\n{}", chain.get_graph());

    let (stream, sample_rate) = init_audio_interleaved(factory)?;
    println!("Playing {} at {}Hz...", name, sample_rate);
    stream.play()?;
    thread::sleep(Duration::from_secs(6));
    Ok(())
}

fn main() -> Result<()> {
    println!("Comparing Standard Reverb vs Low Memory Reverb");
    run_demo("Standard Reverb (f32)", create_standard_reverb_chain)?;
    run_demo(
        "Low Mem Reverb (i16 + downsample)",
        create_low_mem_reverb_chain,
    )?;

    Ok(())
}
