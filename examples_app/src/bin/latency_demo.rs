use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::Mono;
use infinitedsp_core::core::frame_processor::FrameProcessor;
use infinitedsp_core::core::summing_mixer::SummingMixer;
use infinitedsp_core::core::parameter::Parameter;
use infinitedsp_core::effects::utility::lookahead::Lookahead;
use infinitedsp_core::effects::utility::passthrough::Passthrough;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::init_audio_interleaved;
use std::thread;
use std::time::{Duration, Instant};
use std::vec;

struct LatencyDemoEngine {
    source: Oscillator,
    compensated: SummingMixer<Mono, Box<dyn FrameProcessor<Mono> + Send>>,
    direct: SummingMixer<Mono, Box<dyn FrameProcessor<Mono> + Send>>,
    sync_enabled: Parameter,
}

impl FrameProcessor<Mono> for LatencyDemoEngine {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        self.source.process(buffer, sample_index);
        
        if self.sync_enabled.get() > 0.5 {
            self.compensated.process(buffer, sample_index);
        } else {
            self.direct.process(buffer, sample_index);
        }
    }

    fn set_sample_rate(&mut self, sr: f32) {
        self.source.set_sample_rate(sr);
        self.compensated.set_sample_rate(sr);
        self.direct.set_sample_rate(sr);
    }

    fn reset(&mut self) {
        self.source.reset();
        self.compensated.reset();
        self.direct.reset();
    }

    fn name(&self) -> &str {
        "LatencyDemoEngine"
    }

    fn visualize(&self, indent: usize) -> String {
        use core::fmt::Write;
        let mut output = String::new();
        let spaces = " ".repeat(indent);
        writeln!(output, "{}LatencyDemoEngine", spaces).unwrap();
        writeln!(output, "{}|-- Source:", spaces).unwrap();
        output.push_str(&self.source.visualize(indent + 4));
        writeln!(output, "{}|-- Parallel Paths (Switchable):", spaces).unwrap();
        writeln!(output, "{}    |-- Compensated Mode:", spaces).unwrap();
        output.push_str(&self.compensated.visualize(indent + 8));
        writeln!(output, "{}    |-- Direct Mode (Uncompensated):", spaces).unwrap();
        output.push_str(&self.direct.visualize(indent + 8));
        output
    }
}

fn create_engine(sample_rate: f32, sync_enabled: Parameter) -> LatencyDemoEngine {
    let osc = Oscillator::new(AudioParam::hz(50.0), Waveform::Sine);
    let latency_samples = (10.0 * sample_rate / 1000.0) as u32;

    let path_a_1: Box<dyn FrameProcessor<Mono> + Send> = Box::new(Lookahead::new(latency_samples));
    let path_b_1: Box<dyn FrameProcessor<Mono> + Send> = Box::new(Passthrough::new());
    let compensated = SummingMixer::<Mono, Box<dyn FrameProcessor<Mono> + Send>>::new_sync(vec![path_a_1, path_b_1]);

    let path_a_2: Box<dyn FrameProcessor<Mono> + Send> = Box::new(Lookahead::new(latency_samples));
    let path_b_2: Box<dyn FrameProcessor<Mono> + Send> = Box::new(Passthrough::new());
    let direct = SummingMixer::new(vec![path_a_2, path_b_2]);

    LatencyDemoEngine {
        source: osc,
        compensated,
        direct,
        sync_enabled,
    }
}

fn main() -> Result<()> {
    println!("--- InfiniteDSP Latency Comparison Demo ---");
    println!("Signal: 50Hz Sine wave.");
    println!("Setup: Two parallel paths, one delayed by 10ms (180° phase shift).");
    println!("");

    let sync_param = Parameter::new(1.0);
    let sync_enabled = sync_param.clone();

    let viz_engine = create_engine(44100.0, sync_param.clone());
    println!("Signal Chain Configuration:");
    println!("{}", viz_engine.visualize(0));

    let (stream, sample_rate) = init_audio_interleaved(move |sr| {
        let engine = create_engine(sr, sync_enabled.clone());
        
        struct MonoToStereoWrapper(LatencyDemoEngine);
        impl FrameProcessor<infinitedsp_core::core::channels::Stereo> for MonoToStereoWrapper {
            fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
                let mut mono_buf = vec![0.0; buffer.len() / 2];
                self.0.process(&mut mono_buf, sample_index);
                for (i, &s) in mono_buf.iter().enumerate() {
                    buffer[i * 2] = s;
                    buffer[i * 2 + 1] = s;
                }
            }
            fn set_sample_rate(&mut self, sr: f32) { self.0.set_sample_rate(sr); }
            fn reset(&mut self) { self.0.reset(); }
        }
        
        Box::new(MonoToStereoWrapper(engine))
    })?;

    println!("Playing at {}Hz (Run with --release for best performance)...", sample_rate);
    stream.play()?;
    
    let start_time = Instant::now();
    let duration = Duration::from_secs(12);

    while start_time.elapsed() < duration {
        let elapsed = start_time.elapsed().as_secs_f32();
        let is_sync = (elapsed as i32 / 2) % 2 == 0;
        
        if is_sync {
            sync_param.set(1.0);
            print!("\r[Sync: ON ] - Tone is audible. Latency is compensated.    ");
        } else {
            sync_param.set(0.0);
            print!("\r[Sync: OFF] - Silence. Signals are canceling out.         ");
        }
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        thread::sleep(Duration::from_millis(50));
    }

    println!("\n\nDemo finished.");
    Ok(())
}
