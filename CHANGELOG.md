# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2024-05-22

### Added
- **ADSR Retriggering:** Added `create_trigger()` to `Adsr` to allow manual retriggering via a thread-safe `Trigger` handle.
- **Signal Math:** Added `Add` and `Multiply` processors in `effects::utility` for combining signals.
- **Stereo Panner:** Added `StereoPanner` for panning stereo (interleaved) signals.
- **State Variable Filter:** Added `StateVariableFilter` (SVF) supporting LP, HP, BP, Notch, and Peak outputs.

## [0.1.2] - 2024-05-22

### Fixed
- Fixed `no_std` compatibility by disabling default features for `wide` dependency.
- Replaced `std` math functions with `libm` equivalents throughout the codebase.
- Added missing `alloc` imports.

## [0.1.1] - 2024-05-22

### Fixed
- Corrected repository URL in `Cargo.toml`.

## [0.1.0] - 2024-05-22

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
