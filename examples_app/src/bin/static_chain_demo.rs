use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::static_dsp_chain::StaticDspChain;
use infinitedsp_core::effects::time::delay::Delay;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    // Unlike DspChain, StaticDspChain creates a new type for each appended processor.
    // This allows the compiler to inline the entire chain into a single efficient process loop.
    let create_chain = |sample_rate: f32| {
        // 1. Oscillator Source
        let osc = Oscillator::new(AudioParam::hz(440.0), Waveform::Saw);

        // 2. Delay Effect
        let delay = Delay::new(
            1.0,
            AudioParam::ms(350.0),
            AudioParam::linear(0.5),
            AudioParam::linear(0.4),
        );

        // 3. Gain (Volume)
        let gain = Gain::new_fixed(0.1);

        // Construct the static chain.
        // Each .and() call wraps the previous type in a generic SerialProcessor.
        // The final type is: StaticDspChain<Stereo, MonoToStereo<SerialProcessor<SerialProcessor<Oscillator, Delay>, Gain>>>
        // (Roughly speaking, as we also convert to stereo).

        StaticDspChain::new(osc, sample_rate)
            .and(delay)
            .and(gain)
            .to_stereo()
    };

    let (stream, sample_rate) = init_audio_interleaved(create_chain)?;

    println!(
        "Playing Static Chain Demo (Saw -> Delay -> Gain) at {}Hz...",
        sample_rate
    );
    println!("This chain is fully statically dispatched and candidate for 'kernel fusion' optimization.");

    stream.play()?;

    thread::sleep(Duration::from_secs(5));

    Ok(())
}
