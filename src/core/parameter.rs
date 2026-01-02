use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};

/// A thread-safe floating point parameter.
///
/// Uses atomic operations to allow safe concurrent access from UI and audio threads.
#[derive(Clone)]
pub struct Parameter {
    value: Arc<AtomicU32>,
}

impl Parameter {
    /// Creates a new Parameter with an initial value.
    pub fn new(value: f32) -> Self {
        Parameter {
            value: Arc::new(AtomicU32::new(value.to_bits())),
        }
    }

    /// Sets the parameter value.
    pub fn set(&self, value: f32) {
        self.value.store(value.to_bits(), Ordering::Relaxed);
    }

    /// Gets the current parameter value.
    pub fn get(&self) -> f32 {
        f32::from_bits(self.value.load(Ordering::Relaxed))
    }
}
