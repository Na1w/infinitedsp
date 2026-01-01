# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
