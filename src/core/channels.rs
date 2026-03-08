use crate::FrameProcessor;
use alloc::vec::Vec;
/// Marker type representing a Mono signal configuration (1 channel).
pub struct Mono;

/// Marker type representing a Stereo signal configuration (2 channels, interleaved).
pub struct Stereo;

/// Trait implemented by channel configurations to provide buffer utility methods.
pub trait ChannelConfig: Send + Sync {
    fn num_channels() -> usize;
}

impl ChannelConfig for Mono {
    #[inline(always)]
    fn num_channels() -> usize {
        1
    }
}

impl ChannelConfig for Stereo {
    #[inline(always)]
    fn num_channels() -> usize {
        2
    }
}

/// A wrapper that processes a stereo interleaved signal using two independent mono processors.
///
/// This implements a "Dual Mono" topology. It splits the interleaved input buffer
/// into Left and Right channels, processes them independently, and then interleaves them back.
///
/// Useful for applying mono effects (like Filters, Distortion, or simple Delays) to a stereo signal.
pub struct DualMono<L, R> {
    pub left: L,
    pub right: R,
    left_buffer: Vec<f32>,
    right_buffer: Vec<f32>,
}

impl<L, R> DualMono<L, R>
where
    L: FrameProcessor<Mono>,
    R: FrameProcessor<Mono>,
{
    /// Creates a new DualMono processor wrapper.
    ///
    /// # Arguments
    /// * `left` - The processor for the left channel.
    /// * `right` - The processor for the right channel.
    pub fn new(left: L, right: R) -> Self {
        DualMono {
            left,
            right,
            left_buffer: Vec::new(),
            right_buffer: Vec::new(),
        }
    }
}

impl<L, R> FrameProcessor<Stereo> for DualMono<L, R>
where
    L: FrameProcessor<Mono> + Send,
    R: FrameProcessor<Mono> + Send,
{
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let frames = buffer.len() / 2;

        if self.left_buffer.len() < frames {
            self.left_buffer.resize(frames, 0.0);
        }
        if self.right_buffer.len() < frames {
            self.right_buffer.resize(frames, 0.0);
        }

        for (i, frame) in buffer.chunks(2).enumerate() {
            if frame.len() == 2 {
                self.left_buffer[i] = frame[0];
                self.right_buffer[i] = frame[1];
            }
        }

        self.left
            .process(&mut self.left_buffer[0..frames], sample_index);
        self.right
            .process(&mut self.right_buffer[0..frames], sample_index);

        for (i, frame) in buffer.chunks_mut(2).enumerate() {
            if frame.len() == 2 {
                frame[0] = self.left_buffer[i];
                frame[1] = self.right_buffer[i];
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.left.set_sample_rate(sample_rate);
        self.right.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
    }

    fn latency_samples(&self) -> u32 {
        self.left
            .latency_samples()
            .max(self.right.latency_samples())
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "DualMono"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> alloc::string::String {
        use alloc::format;
        let spaces = " ".repeat(indent);
        let mut output = format!("{}DualMono Wrapper\n", spaces);

        output.push_str(&format!("{}  Left Channel:\n", spaces));
        output.push_str(&self.left.visualize(indent + 4));

        output.push_str(&format!("{}  Right Channel:\n", spaces));
        output.push_str(&self.right.visualize(indent + 4));

        output
    }
}

/// Converts a Mono signal to Stereo by duplicating the channel.
pub struct MonoToStereo<P> {
    inner: P,
}

impl<P: FrameProcessor<Mono>> MonoToStereo<P> {
    pub fn new(inner: P) -> Self {
        MonoToStereo { inner }
    }
}

impl<P: FrameProcessor<Mono> + Send> FrameProcessor<Stereo> for MonoToStereo<P> {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let frames = buffer.len() / 2;

        let (mono_slice, _) = buffer.split_at_mut(frames);
        self.inner.process(mono_slice, sample_index);

        for i in (0..frames).rev() {
            let mono_sample = buffer[i];
            buffer[2 * i] = mono_sample;
            buffer[2 * i + 1] = mono_sample;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.inner.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn latency_samples(&self) -> u32 {
        self.inner.latency_samples()
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "MonoToStereo Converter"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> alloc::string::String {
        use alloc::format;
        let spaces = " ".repeat(indent);
        let mut output = format!("{}MonoToStereo\n", spaces);
        output.push_str(&self.inner.visualize(indent + 2));
        output
    }
}

/// Converts a Stereo signal to Mono by mixing down (L+R)/2.
pub struct StereoToMono<P> {
    inner: P,
    stereo_buffer: Vec<f32>,
}

impl<P: FrameProcessor<Stereo>> StereoToMono<P> {
    pub fn new(inner: P) -> Self {
        StereoToMono {
            inner,
            stereo_buffer: Vec::new(),
        }
    }
}

impl<P: FrameProcessor<Stereo> + Send> FrameProcessor<Mono> for StereoToMono<P> {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let frames = buffer.len();
        let stereo_len = frames * 2;

        if self.stereo_buffer.len() < stereo_len {
            self.stereo_buffer.resize(stereo_len, 0.0);
        }

        let stereo_slice = &mut self.stereo_buffer[0..stereo_len];

        for (i, &sample) in buffer.iter().enumerate() {
            stereo_slice[2 * i] = sample;
            stereo_slice[2 * i + 1] = sample;
        }

        self.inner.process(stereo_slice, sample_index);

        for (i, sample) in buffer.iter_mut().enumerate() {
            let l = stereo_slice[2 * i];
            let r = stereo_slice[2 * i + 1];
            *sample = (l + r) * 0.5;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.inner.set_sample_rate(sample_rate);
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn latency_samples(&self) -> u32 {
        self.inner.latency_samples()
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "StereoToMono Converter"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> alloc::string::String {
        use alloc::format;
        let spaces = " ".repeat(indent);
        let mut output = format!("{}StereoToMono\n", spaces);
        output.push_str(&self.inner.visualize(indent + 2));
        output
    }
}
