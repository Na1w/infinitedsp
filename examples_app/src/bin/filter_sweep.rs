use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::filter::ladder_filter::LadderFilter;
use infinitedsp_core::effects::time::delay::Delay;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::offset::Offset;
use infinitedsp_core::synthesis::lfo::{Lfo, LfoWaveform};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio;

fn create_synth_chain(sample_rate: f32) -> DspChain {
    let mut lfo = Lfo::new(AudioParam::hz(0.2), LfoWaveform::Sine);
    lfo.set_unipolar(true);

    let min_freq = 200.0;
    let max_freq = 4000.0;
    let freq_range = max_freq - min_freq;

    let mod_chain = DspChain::new(lfo, sample_rate)
        .and(Gain::new_fixed(freq_range))
        .and(Offset::new(min_freq));

    let cutoff_param = AudioParam::Dynamic(Box::new(mod_chain));

    let osc = Oscillator::new(AudioParam::hz(110.0), Waveform::Saw);
    let filter = LadderFilter::new(cutoff_param, AudioParam::linear(0.8));
    let delay = Delay::new(
        1.0,
        AudioParam::ms(350.0),
        AudioParam::linear(0.4),
        AudioParam::linear(0.3),
    );
    let gain = Gain::new_db(-9.0);

    DspChain::new(osc, sample_rate)
        .and(filter)
        .and(delay)
        .and(gain)
}

fn main() -> Result<()> {
    let (stream, sample_rate) = init_audio(create_synth_chain)?;

    println!("Playing Filter Sweep Synth at {}Hz...", sample_rate);
    stream.play()?;

    std::thread::sleep(std::time::Duration::from_secs(15));

    Ok(())
}
