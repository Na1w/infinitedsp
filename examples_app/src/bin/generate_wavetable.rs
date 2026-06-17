use anyhow::Result;
use std::f32::consts::PI;
use std::path::Path;

fn main() -> Result<()> {
    let samples_per_frame = 2048;
    let num_frames = 4;
    let mut all_samples: Vec<f32> = Vec::with_capacity(samples_per_frame * num_frames);

    for f in 0..num_frames {
        for i in 0..samples_per_frame {
            let t = i as f32 / samples_per_frame as f32;
            let val = match f {
                0 => libm::sinf(t * 2.0 * PI), // Sine
                1 => {
                    // Triangle
                    if t < 0.25 {
                        4.0 * t
                    } else if t < 0.75 {
                        2.0 - 4.0 * t
                    } else {
                        -4.0 + 4.0 * t
                    }
                }
                2 => {
                    if t < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                } // Square
                3 => {
                    // Saw (sum of 100 harmonics)
                    let mut s = 0.0;
                    for h in 1..=100 {
                        s += libm::sinf(t * 2.0 * PI * h as f32) / h as f32;
                    }
                    s
                }
                _ => 0.0,
            };
            all_samples.push(val);
        }
    }

    let mut max_abs = 0.0f32;
    for &s in &all_samples {
        let abs_s = if s < 0.0 { -s } else { s };
        if abs_s > max_abs {
            max_abs = abs_s;
        }
    }

    if max_abs > 0.0 {
        for s in &mut all_samples {
            *s /= max_abs;
        }
    }

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let path = Path::new("assets/audio/demo_wavetable.wav");
    let mut writer = hound::WavWriter::create(path, spec)?;
    for s in all_samples {
        writer.write_sample(s)?;
    }
    writer.finalize()?;

    println!("Successfully generated wavetable at: {:?}", path);
    Ok(())
}
