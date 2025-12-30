#![no_std]

extern crate alloc;

pub mod core;
pub mod effects;
pub mod synthesis;

pub use crate::core::frame_processor::FrameProcessor;
