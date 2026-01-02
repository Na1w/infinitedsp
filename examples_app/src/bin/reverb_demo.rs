use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::multiply::Multiply;
use infinitedsp_core::synthesis::envelope::{Adsr, Trigger};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

struct ReverbDemo {
    chain: DspChain,
    trigger: Trigger,
}

impl ReverbDemo {
    fn new(sample_rate: f32) -> Self {
        let osc = Oscillator::new(AudioParam::hz(440.0), Waveform::Saw);

        let adsr = Adsr::new(
            AudioParam::linear(0.0),
            AudioParam::linear(0.01),
            AudioParam::linear(0.1),
            AudioParam::linear(0.0),
            AudioParam::linear(0.1),
        );
        let trigger = adsr.create_trigger();

        let osc_param = AudioParam::Dynamic(Box::new(DspChain::new(osc, sample_rate)));
        let env_param = AudioParam::Dynamic(Box::new(DspChain::new(adsr, sample_rate)));

        let source = Multiply::new(osc_param, env_param);

        let reverb = Reverb::new();

        let chain = DspChain::new(source, sample_rate)
            .and_mix(0.5, reverb)
            .and(Gain::new_db(-6.0));

        ReverbDemo { chain, trigger }
    }
}

fn main() -> Result<()> {
    let trigger_ref = Arc::new(Mutex::new(None));
    let trigger_clone = trigger_ref.clone();

    let (stream, sample_rate) = init_audio_interleaved(move |sr| {
        let demo = ReverbDemo::new(sr);
        *trigger_clone.lock().unwrap() = Some(demo.trigger.clone());
        demo.chain
    })?;

    println!("Playing Reverb Demo at {}Hz...", sample_rate);
    println!("Triggering pluck sound every second...");

    stream.play()?;

    let start_time = Instant::now();
    let duration = Duration::from_secs(15);

    while start_time.elapsed() < duration {
        if let Some(trigger) = &*trigger_ref.lock().unwrap() {
            trigger.fire();
        }
        thread::sleep(Duration::from_millis(1000));
    }

    Ok(())
}
