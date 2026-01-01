use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::modulation::phaser::Phaser;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use std::thread;
use std::time::Duration;

fn create_phaser_chain(sample_rate: f32) -> DspChain {
    let osc = Oscillator::new(AudioParam::hz(55.0), Waveform::Saw);

    let phaser = Phaser::new(
        AudioParam::hz(100.0),
        AudioParam::hz(8000.0),
        AudioParam::linear(0.85),
        AudioParam::linear(0.5)
    );

    DspChain::new(osc, sample_rate)
        .and(phaser)
}

fn main() -> Result<()> {
    let (stream, sample_rate) = init_audio_interleaved(|sr| create_phaser_chain(sr))?;

    println!("Playing Phaser Demo at {}Hz...", sample_rate);

    stream.play()?;

    thread::sleep(Duration::from_secs(10));

    Ok(())
}
