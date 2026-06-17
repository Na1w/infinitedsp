use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::Stereo;
use infinitedsp_core::core::frame_processor::FrameProcessor;
use infinitedsp_core::synthesis::lfo::{Lfo, LfoWaveform};
use infinitedsp_core::synthesis::wavetable::WavetableOscillator;
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use infinitedsp_examples::wavetable_loader::load_wavetable;
use std::thread;
use std::time::Duration;

fn create_wavetable_demo(_sample_rate: f32) -> Result<Box<dyn FrameProcessor<Stereo> + Send>> {
    let table = load_wavetable("assets/audio/demo_wavetable.wav", 2048)?;
    let lfo = Lfo::new(AudioParam::hz(0.5), LfoWaveform::Sine);

    let osc = WavetableOscillator::new(
        table,
        AudioParam::hz(110.0),
        AudioParam::Dynamic(Box::new(lfo)),
    );

    struct MonoToStereo(WavetableOscillator);
    impl FrameProcessor<Stereo> for MonoToStereo {
        fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
            let len = buffer.len() / 2;
            let mut mono_buf = vec![0.0; len];
            self.0.process(&mut mono_buf, sample_index);
            for (i, &s) in mono_buf.iter().enumerate() {
                buffer[i * 2] = s;
                buffer[i * 2 + 1] = s;
            }
        }
        fn set_sample_rate(&mut self, sr: f32) {
            self.0.set_sample_rate(sr);
        }
        fn reset(&mut self) {
            self.0.reset();
        }
        fn name(&self) -> &str {
            "Wavetable Demo"
        }
        fn visualize(&self, indent: usize) -> String {
            self.0.visualize(indent)
        }
    }

    Ok(Box::new(MonoToStereo(osc)))
}

fn main() -> Result<()> {
    println!("--- InfiniteDSP Wavetable Demo ---");
    println!("Loading wavetable from assets/audio/demo_wavetable.wav");
    println!("Morphing through Sine -> Triangle -> Square -> Saw...");
    println!("");

    let (stream, sample_rate) = init_audio_interleaved(|sr| {
        create_wavetable_demo(sr).expect("Failed to create demo chain")
    })?;

    println!("Playing at {}Hz...", sample_rate);
    stream.play()?;

    thread::sleep(Duration::from_secs(10));

    Ok(())
}
