use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::Mono;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::time::delay::Delay;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::gate::TimedGate;
use infinitedsp_core::low_mem::effects::time::delay_low_mem::DelayLowMem;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio;
use std::thread;
use std::time::Duration;

fn create_noise_source(sample_rate: f32) -> DspChain<Mono> {
    let noise = Oscillator::new(AudioParam::hz(0.0), Waveform::WhiteNoise);

    let mut gate = TimedGate::new(0.2, sample_rate);
    gate.trigger();

    let gate_gain = Gain::new(AudioParam::Dynamic(Box::new(gate)));
    DspChain::new(noise, sample_rate).and(gate_gain)
}

const DELAY_MAX: f32 = 0.5;
const DELAY_TIME: f32 = DELAY_MAX * 1000.0;
const DELAY_FB: f32 = 0.8;
const DELAY_MIX: f32 = 0.5;

fn create_standard_delay_chain(sample_rate: f32) -> DspChain<Mono> {
    let source = create_noise_source(sample_rate);

    let delay = Delay::new(
        DELAY_MAX,
        AudioParam::ms(DELAY_TIME),
        AudioParam::linear(DELAY_FB),
        AudioParam::linear(DELAY_MIX),
    );

    source.and(delay).and(Gain::new_db(-6.0))
}

fn create_low_mem_delay_chain(sample_rate: f32) -> DspChain<Mono> {
    let source = create_noise_source(sample_rate);

    let delay = DelayLowMem::new(
        DELAY_MAX,
        AudioParam::ms(DELAY_TIME),
        AudioParam::linear(DELAY_FB),
        AudioParam::linear(DELAY_MIX),
    );

    source.and(delay).and(Gain::new_db(-6.0))
}

fn run_demo(name: &str, factory: impl Fn(f32) -> DspChain<Mono> + Send + 'static) -> Result<()> {
    let chain = factory(44100.0);
    println!("\n--- {} ---", name);
    println!("Signal Chain:\n{}", chain.get_graph());

    let (stream, sample_rate) = init_audio(factory)?;
    println!("Playing {} at {}Hz...", name, sample_rate);
    stream.play()?;

    thread::sleep(Duration::from_secs(15));
    Ok(())
}

fn main() -> Result<()> {
    println!("Comparing Standard Delay vs Low Memory Delay");
    run_demo("Standard Delay (f32)", create_standard_delay_chain)?;
    run_demo(
        "Low Mem Delay (i16 + downsample)",
        create_low_mem_delay_chain,
    )?;

    Ok(())
}
