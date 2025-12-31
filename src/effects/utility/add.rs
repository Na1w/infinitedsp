use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use wide::f32x4;
use alloc::vec::Vec;

/// Adds two signals together.
pub struct Add {
    input_a: AudioParam,
    input_b: AudioParam,
    buffer_a: Vec<f32>,
    buffer_b: Vec<f32>,
}

impl Add {
    /// Creates a new Add processor.
    pub fn new(input_a: AudioParam, input_b: AudioParam) -> Self {
        Add {
            input_a,
            input_b,
            buffer_a: Vec::new(),
            buffer_b: Vec::new(),
        }
    }
}

impl FrameProcessor for Add {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();
        if self.buffer_a.len() < len { self.buffer_a.resize(len, 0.0); }
        if self.buffer_b.len() < len { self.buffer_b.resize(len, 0.0); }

        self.input_a.process(&mut self.buffer_a[0..len], sample_index);
        self.input_b.process(&mut self.buffer_b[0..len], sample_index);

        let (chunks, remainder) = buffer.as_chunks_mut::<4>();
        let (a_chunks, a_rem) = self.buffer_a[0..len].as_chunks::<4>();
        let (b_chunks, b_rem) = self.buffer_b[0..len].as_chunks::<4>();

        for ((chunk, a_chunk), b_chunk) in chunks.iter_mut().zip(a_chunks).zip(b_chunks) {
            let a = f32x4::from(*a_chunk);
            let b = f32x4::from(*b_chunk);
            // Note: We overwrite the input buffer with A + B.
            // If the input buffer contained a signal, it is replaced.
            // This acts as a source if A and B are sources.
            // If you want to add to the input signal, use Offset with a dynamic param.
            *chunk = (a + b).to_array();
        }

        for ((sample, a), b) in remainder.iter_mut().zip(a_rem).zip(b_rem) {
            *sample = *a + *b;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.input_a.set_sample_rate(sample_rate);
        self.input_b.set_sample_rate(sample_rate);
    }
}
