use crate::core::audio_param::AudioParam;
use crate::core::channels::Mono;
use crate::FrameProcessor;
use alloc::vec::Vec;
use wide::f32x4;

/// The type of distortion algorithm to apply.
pub enum DistortionType {
    /// Soft clipping using tanh.
    SoftClip,
    /// Hard clipping clamping between -1.0 and 1.0.
    HardClip,
    /// Bit crushing reducing bit depth.
    BitCrush(f32),
    /// Foldback distortion using sine.
    Foldback,
    /// Asymmetric distortion.
    Asymmetric,
}

/// A distortion effect.
///
/// Adds harmonics and saturation to the signal.
pub struct Distortion {
    drive: AudioParam,
    mix: AudioParam,
    dist_type: DistortionType,
    drive_buffer: Vec<f32>,
    mix_buffer: Vec<f32>,
}

impl Distortion {
    /// Creates a new Distortion effect.
    ///
    /// # Arguments
    /// * `drive` - Input gain/drive amount.
    /// * `mix` - Dry/Wet mix (0.0 - 1.0).
    /// * `dist_type` - The algorithm to use.
    pub fn new(drive: AudioParam, mix: AudioParam, dist_type: DistortionType) -> Self {
        Distortion {
            drive,
            mix,
            dist_type,
            drive_buffer: Vec::new(),
            mix_buffer: Vec::new(),
        }
    }

    /// Sets the drive parameter.
    pub fn set_drive(&mut self, drive: AudioParam) {
        self.drive = drive;
    }

    /// Sets the mix parameter.
    pub fn set_mix(&mut self, mix: AudioParam) {
        self.mix = mix;
    }
}

impl FrameProcessor<Mono> for Distortion {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.drive_buffer.len() < len {
            self.drive_buffer.resize(len, 0.0);
        }
        if self.mix_buffer.len() < len {
            self.mix_buffer.resize(len, 0.0);
        }

        self.drive
            .process(&mut self.drive_buffer[0..len], sample_index);
        self.mix.process(&mut self.mix_buffer[0..len], sample_index);

        let (chunks, remainder) = buffer.as_chunks_mut::<4>();
        let (drive_chunks, drive_rem) = self.drive_buffer[0..len].as_chunks::<4>();
        let (mix_chunks, mix_rem) = self.mix_buffer[0..len].as_chunks::<4>();

        let one_vec = f32x4::splat(1.0);
        let neg_one_vec = f32x4::splat(-1.0);

        match self.dist_type {
            DistortionType::HardClip => {
                for ((chunk, drive_chunk), mix_chunk) in
                    chunks.iter_mut().zip(drive_chunks).zip(mix_chunks)
                {
                    let input = f32x4::from(*chunk);
                    let drive_vec = f32x4::from(*drive_chunk);
                    let mix_vec = f32x4::from(*mix_chunk);
                    let dry_mix_vec = one_vec - mix_vec;

                    let driven = input * drive_vec;
                    let wet = driven.max(neg_one_vec).min(one_vec);
                    let result = input * dry_mix_vec + wet * mix_vec;
                    *chunk = result.to_array();
                }
            }
            DistortionType::BitCrush(bits) => {
                let steps = libm::powf(2.0, bits);
                let steps_vec = f32x4::splat(steps);

                for ((chunk, drive_chunk), mix_chunk) in
                    chunks.iter_mut().zip(drive_chunks).zip(mix_chunks)
                {
                    let input = f32x4::from(*chunk);
                    let drive_vec = f32x4::from(*drive_chunk);
                    let mix_vec = f32x4::from(*mix_chunk);
                    let dry_mix_vec = one_vec - mix_vec;

                    let driven = input * drive_vec;
                    let wet = (driven * steps_vec).round() / steps_vec;
                    let result = input * dry_mix_vec + wet * mix_vec;
                    *chunk = result.to_array();
                }
            }
            _ => {
                for ((chunk, drive_chunk), mix_chunk) in
                    chunks.iter_mut().zip(drive_chunks).zip(mix_chunks)
                {
                    for i in 0..4 {
                        let input = chunk[i];
                        let drive = drive_chunk[i];
                        let mix = mix_chunk[i];

                        let driven = input * drive;
                        let wet = match self.dist_type {
                            DistortionType::SoftClip => libm::tanhf(driven),
                            DistortionType::Foldback => libm::sinf(driven),
                            DistortionType::Asymmetric => {
                                if driven >= 0.0 {
                                    libm::tanhf(driven)
                                } else {
                                    libm::tanhf(driven * 2.0) * 0.5
                                }
                            }
                            _ => unreachable!(),
                        };
                        chunk[i] = input * (1.0 - mix) + wet * mix;
                    }
                }
            }
        }

        for ((sample, &drive), &mix) in remainder.iter_mut().zip(drive_rem).zip(mix_rem) {
            let input = *sample;
            let driven = input * drive;

            let wet = match self.dist_type {
                DistortionType::SoftClip => libm::tanhf(driven),
                DistortionType::HardClip => driven.clamp(-1.0, 1.0),
                DistortionType::BitCrush(bits) => {
                    let steps = libm::powf(2.0, bits);
                    libm::roundf(driven * steps) / steps
                }
                DistortionType::Foldback => libm::sinf(driven),
                DistortionType::Asymmetric => {
                    if driven >= 0.0 {
                        libm::tanhf(driven)
                    } else {
                        libm::tanhf(driven * 2.0) * 0.5
                    }
                }
            };

            *sample = input * (1.0 - mix) + wet * mix;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.drive.set_sample_rate(sample_rate);
        self.mix.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        // Distortion is stateless (memoryless), so nothing to reset
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        match self.dist_type {
            DistortionType::SoftClip => "Distortion (SoftClip)",
            DistortionType::HardClip => "Distortion (HardClip)",
            DistortionType::BitCrush(_) => "Distortion (BitCrush)",
            DistortionType::Foldback => "Distortion (Foldback)",
            DistortionType::Asymmetric => "Distortion (Asymmetric)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hard_clip() {
        let mut dist = Distortion::new(
            AudioParam::Static(2.0),
            AudioParam::Static(1.0),
            DistortionType::HardClip,
        );
        let mut buffer = [0.4, 0.6, -0.6];
        dist.process(&mut buffer, 0);

        assert!((buffer[0] - 0.8).abs() < 1e-6);
        assert!((buffer[1] - 1.0).abs() < 1e-6);
        assert!((buffer[2] - -1.0).abs() < 1e-6);
    }
}
