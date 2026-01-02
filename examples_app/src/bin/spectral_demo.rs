use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::ola::Ola;
use infinitedsp_core::effects::spectral::pitch_shift::FftPitchShift;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::synthesis::lfo::{Lfo, LfoWaveform};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio;
use std::thread;
use std::time::Duration;

fn create_spectral_chain(sample_rate: f32) -> DspChain {
    let osc = Oscillator::new(AudioParam::hz(220.0), Waveform::Saw);

    let lfo = Lfo::new(AudioParam::hz(0.2), LfoWaveform::Sine);

    let hop_size = 512.0;
    let control_rate = sample_rate / hop_size;

    let lfo_scaled = DspChain::new(lfo, control_rate)
        .and(Gain::new_fixed(12.0));

    let pitch_param = AudioParam::Dynamic(Box::new(lfo_scaled));

    let pitch_shifter = FftPitchShift::<1024>::new(pitch_param);

    let ola_processor = Ola::<_, 1024>::with(pitch_shifter);

    DspChain::new(osc, sample_rate)
        .and(ola_processor)
        .and(Gain::new_db(-6.0))
}

fn main() -> Result<()> {
    let chain = create_spectral_chain(44100.0);
    println!("Signal Chain:\n{}", chain.get_graph());

    let (stream, sample_rate) = init_audio(create_spectral_chain)?;

    println!("Playing Spectral Pitch Shift Demo at {}Hz...", sample_rate);

    stream.play()?;

    thread::sleep(Duration::from_secs(15));

    Ok(())
}
