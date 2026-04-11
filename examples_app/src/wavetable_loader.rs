use anyhow::{Context, Result};
use infinitedsp_core::synthesis::wavetable::Wavetable;
use std::path::Path;

/// Loads a wavetable from a WAV file.
///
/// Supports files with 2048 samples per frame (standard Serum format).
pub fn load_wavetable<P: AsRef<Path>>(path: P, samples_per_frame: usize) -> Result<Wavetable> {
    let mut reader = hound::WavReader::open(path).context("Failed to open WAV file")?;
    let spec = reader.spec();

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect()
        }
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            reader.samples::<i32>()
                .map(|s| s.unwrap_or(0) as f32 / max_val)
                .collect()
        }
    };

    let mono_samples = if spec.channels > 1 {
        samples.chunks(spec.channels as usize).map(|chunk| chunk[0]).collect()
    } else {
        samples
    };

    let total_samples = (mono_samples.len() / samples_per_frame) * samples_per_frame;
    let final_samples = mono_samples[0..total_samples].to_vec();

    Ok(Wavetable::new_bandlimited(final_samples, samples_per_frame))
}
