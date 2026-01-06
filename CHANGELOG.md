# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [x.x.x] - xxxx-xx-xx

### Added
- **LowMem Time Effects:** Added `DelayLowMem` and `ReverbLowMem` with low memory usage, these prioritize low memory usage over high quality and performance.

### Changed
- **Delay:** Improved the quality and performance of `Delay`, it now supports sample accurate modulation as well.

## [0.7.0] - 2026-01-04

### Added
- **PingPongDelay:** Added a new stereo ping-pong delay effect (`src/effects/time/ping_pong_delay.rs`).
- **New Demo:** Added `ping_pong_demo` showcasing the new PingPongDelay effect.
- **Reset Functionality:** Added `reset()` method to `FrameProcessor` trait and all implementations, allowing for state clearing (e.g., delay lines, envelopes) without reallocation.

### Changed
- **TimedGate (Breaking):** TimedGate no longer starts in triggered state.

## [0.6.0] - 2026-01-03

### Added
- **Type-Safe Channel System (Breaking):** Introduced `Mono` and `Stereo` marker types and made `FrameProcessor` generic over `ChannelConfig`.
- **Stereo Processing:** Added `DualMono`, `MonoToStereo`, and `StereoToMono` processors for explicit channel management.
- **DspChain Conversion:** Added `.to_stereo()` and `.to_mono()` methods to `DspChain` for fluent channel conversion.
- **New Demos:**
  - `dual_mono_demo`: Demonstrates independent L/R processing (Ping-Pong Delay).
  - `phaser_demo`: 6-stage Phaser effect.

### Changed
- **Reverb Overhaul (Breaking):**
  - Removed `gain` parameter from `Reverb` (now uses fixed internal scaling).
  - Added `room_size` and `damping` as modulatable `AudioParam`s.
  - Tuned comb filter lengths for better sound quality.
  - Optimized `DelayLine` implementation for better performance.
  - `Reverb` is now a "Wet-only" insert effect. This allows it to be used correctly with `ParallelMixer` (Dry/Wet) and in manual mix topologies.
- **StateVariableFilter:** Minor optimizations for constant parameters and cutoff clamping.
- **BrassModel Overhaul:** Still work in progress..
- **DspChain Visualization:** `get_graph()` now indicates whether the chain is Mono or Stereo.
- **Examples:** All examples updated to use the new type-safe channel system.

## [0.5.0] - 2026-01-02

### Added
- **SummingMixer:** Added `gain` and `soft_clip` (saturation) parameters to `SummingMixer` for better mixing control.
- **Reverb Demo:** Added `reverb_demo.rs`.

### Changed
- **Renaming (Breaking):** Renamed `Mixer` to `ParallelMixer` to better reflect its purpose (Dry/Wet blending).
- **Reverb Overhaul (Breaking):**
  - Removed `gain` parameter from `Reverb` (now uses fixed internal scaling).
  - Added `room_size` and `damping` as modulatable `AudioParam`s.
  - Tuned comb filter lengths for better sound quality.
  - Optimized `DelayLine` implementation for better performance.
- **Demo:** Updated `infinitedsp_demo` to use `SummingMixer` with saturation instead of a recursive tree of `Add` nodes.
- **StateVariableFilter:** Minor optimizations for constant parameters and cutoff clamping.

## [0.4.0] - 2026-01-02

### Added
- **Graph Visualization:** Added `get_graph()` to `DspChain` and `visualize()` to `FrameProcessor` to generate ASCII diagrams of the signal chain.
- **Feature Flag:** Added `debug_visualize` feature (disabled by default) to include visualization code.
- **Modulation Demo:** Added `modulation_demo.rs` showcasing Tremolo, Chorus, and Tape Delay.
- **PredictiveLadderFilter:** Added `PredictiveLadderFilter` which is faster implementation of `LadderFilter` using Linear Prediction ZDF.

### Changed
- **Performance:** Optimized `LadderFilter`, `Compressor`, `Gain`, and `LadderFilter` to skip expensive calculations when parameters are constant.
- **AudioParam:** Added `get_constant()` to efficiently check for static values.
- **Examples:** All examples now print their signal chain graph on startup.
- **Edition:** Synchronized crate and examples to Rust 2021 edition.

## [0.3.0] - 2026-01-01

### Added
- **MapRange:** New utility processor for mapping control signals (0-1) to arbitrary ranges with linear or exponential curves.
- **TimedGate:** New utility processor for generating gate signals with a specific duration.
- **StereoWidener:** New utility processor for M/S-based stereo widening.
- **Box Support:** Implemented `FrameProcessor` for `Box<T>`, enabling easier dynamic dispatch.
- **InfiniteDSP Demo:** Added a new demo to showcase the polyphony and modulation abilities, might be recognizable ;) 

### Changed
- **Optimizations:**
  - Implemented parameter caching in `Biquad`, `Compressor`, and `GranularPitchShift` to reduce CPU usage for static parameters.
  - Replaced `Vec` with arrays in `Phaser` and `Reverb` filter banks to reduce heap allocations.

### Fixed
- **Buffer Reset:** Fixed a critical bug in `Oscillator` and `Adsr` where internal buffers were not cleared, causing issues when used with additive modulation (e.g., `Offset`).
- **Phaser:** Fixed race condition resulting in suboptimal phase response.

## [0.2.0] - 2025-12-31

### Added
- **ADSR Retriggering:** Added `create_trigger()` to `Adsr` to allow manual retriggering via a thread-safe `Trigger` handle.
- **Signal Math:** Added `Add` and `Multiply` processors in `effects::utility` for combining signals.
- **Stereo Panner:** Added `StereoPanner` for panning stereo (interleaved) signals.
- **State Variable Filter:** Added `StateVariableFilter` (SVF) supporting LP, HP, BP, Notch, and Peak outputs.

## [0.1.2] - 2025-12-30

### Fixed
- Fixed `no_std` compatibility by disabling default features for `wide` dependency.
- Replaced `std` math functions with `libm` equivalents throughout the codebase.
- Added missing `alloc` imports.

## [0.1.1] - 2025-12-30

### Fixed
- Corrected repository URL in `Cargo.toml`.

## [0.1.0] - 2025-12-30

### Added
- Initial public release of `infinitedsp-core`.
- Modular DSP architecture with `DspChain` and `Mixer`.
- `AudioParam` system for flexible modulation.
- `no_std` support via `alloc` and `libm`.
- Spectral processing engine (`Ola`) and effects (`FftPitchShift`, `SpectralFilter`).
- Synthesis modules: `Oscillator`, `KarplusStrong`, `BrassModel`, `Lfo`, `Adsr`.
- Effects: `Delay`, `TapeDelay`, `Reverb`, `LadderFilter`, `Biquad`, `Compressor`, `Distortion`, `Phaser`, `Tremolo`, `RingMod`, `GranularPitchShift`.
- SIMD optimization using `wide`.
- Comprehensive examples in `examples_app`.
