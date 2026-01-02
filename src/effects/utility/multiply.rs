use crate::core::audio_param::AudioParam;
use crate::FrameProcessor;
#[cfg(feature = "debug_visualize")]
use alloc::format;
#[cfg(feature = "debug_visualize")]
use alloc::string::String;
use alloc::vec::Vec;
use wide::f32x4;

/// Multiplies two signals together (ring modulation).
/// This is functionally similar to Gain with a dynamic parameter, but can be clearer.
pub struct Multiply {
    input_a: AudioParam,
    input_b: AudioParam,
    buffer_a: Vec<f32>,
    buffer_b: Vec<f32>,
}

impl Multiply {
    /// Creates a new Multiply processor.
    pub fn new(input_a: AudioParam, input_b: AudioParam) -> Self {
        Multiply {
            input_a,
            input_b,
            buffer_a: Vec::new(),
            buffer_b: Vec::new(),
        }
    }
}

impl FrameProcessor for Multiply {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.buffer_a.len() < len {
            self.buffer_a.resize(len, 0.0);
        }
        if self.buffer_b.len() < len {
            self.buffer_b.resize(len, 0.0);
        }

        self.input_a
            .process(&mut self.buffer_a[0..len], sample_index);
        self.input_b
            .process(&mut self.buffer_b[0..len], sample_index);

        let (chunks, remainder) = buffer.as_chunks_mut::<4>();
        let (a_chunks, a_rem) = self.buffer_a[0..len].as_chunks::<4>();
        let (b_chunks, b_rem) = self.buffer_b[0..len].as_chunks::<4>();

        for ((chunk, a_chunk), b_chunk) in chunks.iter_mut().zip(a_chunks).zip(b_chunks) {
            let a = f32x4::from(*a_chunk);
            let b = f32x4::from(*b_chunk);
            *chunk = (a * b).to_array();
        }

        for ((sample, a), b) in remainder.iter_mut().zip(a_rem).zip(b_rem) {
            *sample = *a * *b;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.input_a.set_sample_rate(sample_rate);
        self.input_b.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Multiply (Ring Mod)"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> String {
        let spaces = " ".repeat(indent);
        let mut output = String::new();

        output.push_str(&format!("{}Multiply (Ring Mod)\n", spaces));

        output.push_str(&format!("{}  |-- Input A:\n", spaces));
        if let AudioParam::Dynamic(p) = &self.input_a {
            let inner = p.visualize(0);
            for line in inner.lines() {
                output.push_str(&format!("{}  |    {}\n", spaces, line));
            }
        } else {
            output.push_str(&format!("{}  |    (Static/Linked Value)\n", spaces));
        }

        output.push_str(&format!("{}  |-- Input B:\n", spaces));
        if let AudioParam::Dynamic(p) = &self.input_b {
            let inner = p.visualize(0);
            for line in inner.lines() {
                output.push_str(&format!("{}  |    {}\n", spaces, line));
            }
        } else {
            output.push_str(&format!("{}  |    (Static/Linked Value)\n", spaces));
        }

        output
    }
}
