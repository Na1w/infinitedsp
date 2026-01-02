use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::modulation::modulated_delay::ModulatedDelay;
use infinitedsp_core::effects::modulation::tremolo::Tremolo;
use infinitedsp_core::effects::time::tape_delay::TapeDelay;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::passthrough::Passthrough;
use infinitedsp_core::synthesis::envelope::Adsr;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio;
use std::thread;
use std::time::{Duration, Instant};

fn create_modulation_chain(sample_rate: f32, gate: AudioParam) -> DspChain {
    let osc = Oscillator::new(AudioParam::hz(110.0), Waveform::Saw);

    let env = Adsr::new(
        gate,
        AudioParam::ms(10.0),
        AudioParam::ms(100.0),
        AudioParam::linear(0.5),
        AudioParam::ms(200.0),
    );

    let vca = Gain::new(AudioParam::Dynamic(Box::new(env)));

    let tremolo = Tremolo::new(AudioParam::hz(6.0), AudioParam::linear(0.7));

    let chorus = ModulatedDelay::new_chorus();

    let tape_delay = TapeDelay::new(
        1.0,
        AudioParam::ms(300.0),
        AudioParam::linear(0.4),
        AudioParam::linear(0.3),
    );

    let passthrough = Passthrough::new();

    DspChain::new(osc, sample_rate)
        .and(vca)
        .and(tremolo)
        .and(chorus)
        .and(tape_delay)
        .and(passthrough)
        .and(Gain::new_db(-3.0))
}

fn main() -> Result<()> {
    use infinitedsp_core::core::parameter::Parameter;

    let gate_param = Parameter::new(0.0);
    let g = gate_param.clone();

    let chain = create_modulation_chain(44100.0, AudioParam::Linked(gate_param.clone()));
    println!("Signal Chain:\n{}", chain.get_graph());

    let (stream, _sample_rate) =
        init_audio(move |sr| create_modulation_chain(sr, AudioParam::Linked(g.clone())))?;

    println!("Playing Modulation Demo (Tremolo -> Chorus -> TapeDelay)...");
    stream.play()?;

    let start = Instant::now();
    let duration = Duration::from_secs(10);

    while start.elapsed() < duration {
        gate_param.set(1.0);
        thread::sleep(Duration::from_millis(100));
        gate_param.set(0.0);
        thread::sleep(Duration::from_millis(400));
    }

    Ok(())
}
