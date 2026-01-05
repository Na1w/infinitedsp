use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::Stereo;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::time::delay::Delay;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::offset::Offset;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use std::thread;
use std::time::Duration;

fn create_chain(sample_rate: f32) -> DspChain<Stereo> {
    let source = Oscillator::new(AudioParam::hz(440.0), Waveform::Sine);
    let mod_osc = Oscillator::new(AudioParam::hz(110.0), Waveform::Sine);

    let mod_chain = DspChain::new(mod_osc, sample_rate)
        .and(Gain::new_fixed(0.002))
        .and(Offset::new(0.005));

    let delay = Delay::new(
        0.1,
        AudioParam::Dynamic(Box::new(mod_chain)),
        AudioParam::linear(0.0),
        AudioParam::linear(1.0),
    );

    DspChain::new(source, sample_rate)
        .and(delay)
        .and(Gain::new_fixed(0.5))
        .to_stereo()
}

fn main() -> Result<()> {
    let (stream, _sample_rate) = init_audio_interleaved(move |sr| create_chain(sr))?;

    let chain = create_chain(44100.0);

    println!("Signal Chain:\n{}", chain.get_graph());

    println!("Playing Audio-Rate Delay Modulation Demo");
    println!("Source: Sine 440Hz");
    println!("Modulator: Sine 110Hz modulating Delay Time (+/- 2ms)");

    stream.play()?;

    thread::sleep(Duration::from_secs(15));

    Ok(())
}
