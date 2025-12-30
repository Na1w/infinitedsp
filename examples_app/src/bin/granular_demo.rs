use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::spectral::granular_pitch::GranularPitchShift;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::synthesis::lfo::{Lfo, LfoWaveform};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio;
use std::thread;
use std::time::Duration;

fn create_granular_chain(sample_rate: f32) -> DspChain {
    let osc = Oscillator::new(AudioParam::hz(220.0), Waveform::Saw);

    let lfo = Lfo::new(AudioParam::hz(0.2), LfoWaveform::Sine);

    let lfo_scaled = DspChain::new(lfo, sample_rate)
        .and(Gain::new_fixed(12.0));

    let pitch_param = AudioParam::Dynamic(Box::new(lfo_scaled));

    let pitch_shifter = GranularPitchShift::new(50.0, pitch_param);

    DspChain::new(osc, sample_rate)
        .and(pitch_shifter)
        .and(Gain::new_db(-6.0))
}

fn main() -> Result<()> {
    let (stream, sample_rate) = init_audio(create_granular_chain)?;

    println!("Playing Granular Pitch Shift Demo at {}Hz...", sample_rate);

    stream.play()?;

    thread::sleep(Duration::from_secs(15));

    Ok(())
}
