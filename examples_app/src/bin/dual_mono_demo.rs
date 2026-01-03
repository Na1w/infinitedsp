use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::DualMono;
use infinitedsp_core::core::channels::Stereo;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::time::delay::Delay;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::synthesis::envelope::{Adsr, Trigger};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn create_dual_mono_chain(sample_rate: f32) -> (DspChain<Stereo>, Trigger) {
    let osc = Oscillator::new(AudioParam::hz(440.0), Waveform::Saw);

    let adsr = Adsr::new(
        AudioParam::Static(0.0),
        AudioParam::ms(5.0),
        AudioParam::ms(200.0),
        AudioParam::linear(0.0),
        AudioParam::ms(100.0),
    );
    let trigger = adsr.create_trigger();

    let source =
        DspChain::new(osc, sample_rate).and(Gain::new(AudioParam::Dynamic(Box::new(adsr))));

    let delay_l = Delay::new(
        1.0,
        AudioParam::ms(300.0),
        AudioParam::linear(0.6),
        AudioParam::linear(0.5),
    );

    let delay_r = Delay::new(
        1.0,
        AudioParam::ms(450.0),
        AudioParam::linear(0.6),
        AudioParam::linear(0.5),
    );

    let dual_mono_delay = DualMono::new(delay_l, delay_r);

    let chain = source
        .to_stereo()
        .and(dual_mono_delay)
        .and(Gain::new_db(-3.0));

    (chain, trigger)
}

fn main() -> Result<()> {
    let trigger_ref = Arc::new(Mutex::new(None));
    let trigger_clone = trigger_ref.clone();

    let (stream, sample_rate) = init_audio_interleaved(move |sr| {
        let (chain, trigger) = create_dual_mono_chain(sr);
        *trigger_clone.lock().unwrap() = Some(trigger);
        chain
    })?;

    let (chain, _) = create_dual_mono_chain(44100.0);
    println!("Signal Chain (Dual Mono):\n{}", chain.get_graph());

    println!("Playing Dual Mono Demo at {}Hz...", sample_rate);
    println!("Left: 300ms Delay");
    println!("Right: 450ms Delay");

    stream.play()?;

    for _ in 0..10 {
        if let Some(t) = &*trigger_ref.lock().unwrap() {
            t.fire();
        }
        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
