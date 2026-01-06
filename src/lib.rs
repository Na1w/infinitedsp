#![no_std]

//! # InfiniteDSP Core
//!
//! A modular, high-performance audio DSP library for Rust, designed for real-time synthesis and effects processing.
//! It is `no_std` compatible (requires `alloc`).
//!
//! ## Example
//!
//! ```
//! use infinitedsp_core::core::dsp_chain::DspChain;
//! use infinitedsp_core::core::audio_param::AudioParam;
//! use infinitedsp_core::core::frame_processor::FrameProcessor;
//! use infinitedsp_core::core::channels::Mono;
//! use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
//! use infinitedsp_core::effects::utility::gain::Gain;
//!
//! // Create a simple synth chain: Oscillator -> Gain
//! let osc = Oscillator::new(AudioParam::hz(440.0), Waveform::Sine);
//! let gain = Gain::new_fixed(0.5);
//!
//! let mut chain = DspChain::new(osc, 44100.0).and(gain);
//!
//! // Process a block of audio
//! let mut buffer = [0.0; 128];
//! chain.process(&mut buffer, 0);
//!
//! // Check that something happened (first sample of sine is 0, but next should be non-zero)
//! assert!(buffer[1].abs() > 0.0);
//! ```

extern crate alloc;

pub mod core;
pub mod effects;
pub mod synthesis;
pub mod low_mem;

pub use crate::core::channels::{ChannelConfig, Mono, Stereo};
pub use crate::core::frame_processor::FrameProcessor;
