# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-03-08

### Added
- **Spectral Smear:** Introduced `SpectralSmear` for time-averaging and smearing of frequency components in the spectral domain.
- **New Demo:** Added `spectral_smear_demo.rs` showcasing the new spectral smear effect.

### Changed
- **Compressor (Optimization):** Added a dual-path process loop that optimizes performance when parameters are constant.
- **RNG Refactor:** Centralized random number generation logic.

## [1.0.1] - 2026-03-06

### Fixed
- **Ola** Modulation parameters provided to spectral processors did not propagate

## [1.0.0] - 2026-03-06

### Added
- **Speech Synthesizer:** Introduced `SpeechSynth` in `src/synthesis/speech.rs`, a formant-based vocal synthesizer with a complete phoneme system.
- **Vowel Filter:** Added `VowelFilter` in `src/effects/filter/vowel.rs`, simulating the human vocal tract with multiple formants.
- **Stutter Effect:** Added `Stutter` in `src/effects/time/stutter.rs` for rhythmic repetitions and digital glitch effects.
- **Oscillator Stack:** Added `Stack` in `src/synthesis/stack.rs` for thick, detuned multi-oscillator sounds.
- **New Examples:** Added `speech_synth` demo and significantly updated `acid_trip` to showcase the new synthesis capabilities.

### Changed
- **Oscillator (Enhanced):** Added `NaiveSaw` waveform and exposed internal state (phase, rng) for more flexible modulation. Added `tick` method for sample-by-sample processing.
- **AudioParam:** Added `get_value_at` for direct sampling of parameters at specific indices.
- **StateVariableFilter:** Optimized with a new `tick` method for high-performance usage in complex filters like `VowelFilter`.

## [0.9.0] - 2026-03-04

### Added
- **Bypass Wrapper:** Added `Bypass<T, C>` in `src/effects/utility/bypass.rs`, allowing any processor to be dynamically toggled in a chain.

### Changed
- **Oscillator (Optimization):** Fully integrated vectorised block processing from `picoDSP`. Standard `Oscillator` now uses SIMD for performance gains on all waveforms.
- **LFO (Optimization):** Integrated fast sine approximation from `picoDSP`. Added `set_range` and `set_unipolar` methods for easier modulation setup.
- **Consolidation:** Standardized `Waveform` and `LfoWaveform` enums across the ecosystem, removing redundant "Fast" variants.

## [0.8.0] - 2026-01-24

### Added
- **LowMem Time Effects:** Added `DelayLowMem` and `ReverbLowMem` with low memory usage, these prioritize low memory usage over high quality and performance.
- **Static Dispatch:** Added `StaticDspChain` a statically typed alternative to `DspChain` allowing the rust compiler to inline/optimize more aggressively.

### Changed
- **Delay:** Improved the quality and performance of `Delay`, it now supports sample accurate modulation as well.
- **SummingMixer:** Made `SummingMixer` generic to support both dynamic (`Box<dyn FrameProcessor>`) and static (`StaticDspChain`) inputs, enabling fully static mixing pipelines.
- **Performance:** This release includes the first AI-contributed improvements to the library. Google Labs' "Jules" identified and implemented a significant optimization in the `Adsr` module. Additionally, it implemented the `StaticDspChain` struct for static dispatch optimization.

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
