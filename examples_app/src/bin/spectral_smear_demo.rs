use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::Mono;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::ola::Ola;
use infinitedsp_core::effects::spectral::spectral_smear::SpectralSmear;
use infinitedsp_core::effects::utility::add::Add;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::synthesis::envelope::{Adsr, Trigger};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_core::FrameProcessor;
use infinitedsp_examples::audio_backend::init_audio;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

struct SonarDemo {
    chain: DspChain<Mono>,
    trigger: Trigger,
}

impl SonarDemo {
    fn new(sample_rate: f32) -> Self {
        let drone = Oscillator::new(AudioParam::hz(440.0), Waveform::Sine);
        let drone_chain = DspChain::new(drone, sample_rate).and(Gain::new_fixed(0.15));

        let ping_osc = Oscillator::new(AudioParam::hz(880.0), Waveform::Sine);
        let mut adsr = Adsr::new(
            AudioParam::linear(0.0),
            AudioParam::ms(5.0),
            AudioParam::ms(145.0),
            AudioParam::linear(0.0),
            AudioParam::ms(10.0),
        );
        adsr.set_sample_rate(sample_rate);
        let trigger = adsr.create_trigger();

        let ping_chain = DspChain::new(ping_osc, sample_rate)
            .and(Gain::new(AudioParam::Dynamic(Box::new(adsr))));

        let mix = Add::new(
            AudioParam::Dynamic(Box::new(drone_chain)),
            AudioParam::Dynamic(Box::new(ping_chain)),
        );

        let smear_proc = SpectralSmear::<1024>::new(AudioParam::Static(0.985));
        let mut smear = Ola::<_, 1024>::with(smear_proc);
        smear.set_sample_rate(sample_rate);

        let chain = DspChain::new(mix, sample_rate)
            .and(smear)
            .and(Gain::new_fixed(0.5));

        SonarDemo { chain, trigger }
    }
}

fn main() -> Result<()> {
    let trigger_ref = Arc::new(Mutex::new(None));
    let trigger_clone = trigger_ref.clone();

    let (stream, sample_rate) = init_audio(move |sr| {
        let demo = SonarDemo::new(sr);
        *trigger_clone.lock().unwrap() = Some(demo.trigger.clone());
        demo.chain
    })?;

    println!(
        "Signal Chain ({}Hz):\n{}",
        sample_rate,
        SonarDemo::new(sample_rate).chain.get_graph()
    );
    println!("Starting Sonar Demo...");
    println!("Playing for 15 seconds...");

    stream.play()?;

    let start_time = Instant::now();
    let duration = Duration::from_secs(15);
    let mut last_trigger = Instant::now() - Duration::from_secs(5);

    while start_time.elapsed() < duration {
        if last_trigger.elapsed() >= Duration::from_secs(4) {
            let guard = trigger_ref.lock().unwrap();
            if let Some(trigger) = &*guard {
                trigger.fire();
            }
            last_trigger = Instant::now();
        }
        thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}
