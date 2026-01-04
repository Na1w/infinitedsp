use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::Stereo;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::time::ping_pong_delay::PingPongDelay;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::synthesis::envelope::{Adsr, Trigger};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn create_ping_pong_chain(sample_rate: f32) -> (DspChain<Stereo>, Trigger) {
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

    let ping_pong = PingPongDelay::new(
        1.0,
        AudioParam::ms(300.0),
        AudioParam::linear(0.6),
        AudioParam::linear(0.5),
    );

    let chain = source.to_stereo().and(ping_pong).and(Gain::new_db(-3.0));

    (chain, trigger)
}

fn main() -> Result<()> {
    let trigger_ref = Arc::new(Mutex::new(None));
    let trigger_clone = trigger_ref.clone();

    let (stream, sample_rate) = init_audio_interleaved(move |sr| {
        let (chain, trigger) = create_ping_pong_chain(sr);
        *trigger_clone.lock().unwrap() = Some(trigger);
        chain
    })?;

    let (chain, _) = create_ping_pong_chain(44100.0);
    println!("Signal Chain (Ping Pong):\n{}", chain.get_graph());

    println!("Playing Ping Pong Delay Demo at {}Hz...", sample_rate);

    stream.play()?;

    for _ in 0..10 {
        if let Some(t) = &*trigger_ref.lock().unwrap() {
            t.fire();
        }
        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
