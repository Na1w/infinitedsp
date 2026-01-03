use crate::core::channels::ChannelConfig;
use crate::FrameProcessor;

/// A processor that does nothing.
///
/// Passes the input signal directly to the output unchanged.
pub struct Passthrough;

impl Passthrough {
    /// Creates a new Passthrough processor.
    pub fn new() -> Self {
        Passthrough
    }
}

impl Default for Passthrough {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: ChannelConfig> FrameProcessor<C> for Passthrough {
    fn process(&mut self, _buffer: &mut [f32], _sample_index: u64) {}

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Passthrough"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::channels::Mono;

    #[test]
    fn test_passthrough() {
        let mut pt = Passthrough::new();
        let mut buffer = [1.0, -0.5, 0.0];
        let original = buffer;
        FrameProcessor::<Mono>::process(&mut pt, &mut buffer, 0);
        assert_eq!(buffer, original);
    }
}
