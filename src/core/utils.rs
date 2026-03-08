/// A lightweight random number generator.
pub struct FastRng {
    state: u32,
}

impl FastRng {
    /// Creates a new FastRng with a given seed.
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// Generates the next random `u32` from a state.
    #[inline(always)]
    pub fn next_u32_stateless(state: &mut u32) -> u32 {
        *state = state.wrapping_mul(1103515245).wrapping_add(12345);
        *state
    }

    /// Generates a random `f32` in the range [-1.0, 1.0] from a state.
    #[inline(always)]
    pub fn next_f32_bipolar_stateless(state: &mut u32) -> f32 {
        let val = (Self::next_u32_stateless(state) >> 16) & 0x7FFF;
        (val as f32 / 32768.0) * 2.0 - 1.0
    }

    /// Generates a random `f32` in the range [0.0, 1.0) from a state.
    #[inline(always)]
    pub fn next_f32_unipolar_stateless(state: &mut u32) -> f32 {
        (Self::next_u32_stateless(state) as f32) / (u32::MAX as f32)
    }

    /// Generates the next random `u32`.
    #[inline(always)]
    pub fn next_u32(&mut self) -> u32 {
        Self::next_u32_stateless(&mut self.state)
    }

    /// Generates a random `f32` in the range [-1.0, 1.0].
    #[inline(always)]
    pub fn next_f32_bipolar(&mut self) -> f32 {
        Self::next_f32_bipolar_stateless(&mut self.state)
    }

    /// Generates a random `f32` in the range [0.0, 1.0).
    #[inline(always)]
    pub fn next_f32_unipolar(&mut self) -> f32 {
        Self::next_f32_unipolar_stateless(&mut self.state)
    }
}

impl Default for FastRng {
    fn default() -> Self {
        Self::new(12345)
    }
}
