<div align="center">
  <img src="assets/logo.svg" alt="InfiniteDSP Logo" width="600">
</div>

# InfiniteDSP Core

A modular, high-performance audio DSP library for Rust, designed for real-time synthesis and effects processing. 
It is `no_std` compatible (requires `alloc`), making it suitable for embedded audio applications as well as desktop software.

## Features

*   **`no_std` Compatible:** Built for portability using `libm` and `alloc`.
*   **Type-Safe Channel System:** Explicit `Mono` and `Stereo` types prevent routing errors.
*   **Modular Architecture:** Build complex audio chains using `DspChain` and `Mixer`.
*   **AudioParam System:** All parameters can be static, linked to thread-safe controls (atomics), or modulated by other audio signals (e.g., LFOs, Envelopes) at sample rate.
*   **Signal Math:** Combine signals easily with `Add` and `Multiply` processors.
*   **SIMD Optimization:** Uses `wide` for SIMD-accelerated processing where applicable.
*   **Graph Visualization:** Generate ASCII diagrams of your signal chain for easy debugging (`chain.get_graph()`).
*   **Spectral Processing:** Includes a robust Overlap-Add (OLA) engine for FFT-based effects.
*   **Comprehensive Effect Suite:**
    *   **Time:** Delay, Tape Delay (with saturation & flutter), Reverb (Schroeder).
    *   **Filter:** Biquad (LowPass, HighPass, BandPass, Notch), Ladder Filter (Moog-style, both Iterative and Predictive ZDF), State Variable Filter (TPT/ZDF).
    *   **Dynamics:** Compressor, Limiter, Distortion (Soft/Hard Clip, BitCrush, Foldback).
    *   **Modulation:** Phaser, Tremolo, Ring Modulator, Chorus, Flanger.
    *   **Spectral:** FFT Pitch Shift, Granular Pitch Shift, Spectral Filter.
    *   **Utility:** Gain, Offset, Stereo Panner, Stereo Widener, MapRange, TimedGate.
*   **Synthesis:**
    *   **Oscillators:** Sine, Triangle, Saw, Square (PolyBLEP anti-aliased), Noise.
    *   **Physical Modeling:** Karplus-Strong (String), Brass Model.
    *   **Control:** LFO, ADSR Envelope (with retrigger support).

## Demos

Listen to some of the examples generated with this library:

[![Filter Sweep Demo](assets/player_filter_sweep.svg)](assets/audio/filter_sweep.wav)

[![Trance Synth Demo](assets/player_trance_synth.svg)](assets/audio/trance_synth.wav)

## Project Structure

*   `src/core`: Core traits and infrastructure (`FrameProcessor`, `AudioParam`, `DspChain`, `Ola`, `ParallelMixer`, `SummingMixer`, `Stereo`).
*   `src/effects`: Audio effects implementations.
*   `src/synthesis`: Sound generators and control signals.
*   `examples_app`: A separate workspace member containing runnable examples using `cpal`.

## Usage

Add `infinitedsp-core` to your dependencies.

```rust
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::Mono;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_core::effects::time::delay::Delay;

// Create an oscillator (Mono source)
let osc = Oscillator::new(AudioParam::hz(440.0), Waveform::Saw);

// Create a delay effect (Mono effect)
let delay = Delay::new(
    1.0, // Max delay time in seconds
    AudioParam::ms(350.0),   // Delay time
    AudioParam::linear(0.5), // Feedback
    AudioParam::linear(0.3)  // Mix
);

// Chain them together. The chain is typed as DspChain<Mono>.
let mut chain = DspChain::new(osc, 44100.0).and(delay);

// Print the signal chain (requires 'debug_visualize' feature)
println!("{}", chain.get_graph());

// Process a buffer
let mut buffer = [0.0; 512];
chain.process(&mut buffer, 0);
```

### Feature Flags

*   **`debug_visualize`**: Enables `get_graph()` and `visualize()` methods for debugging signal chains. Disabled by default to minimize binary size for embedded targets.

## Running Examples

The project includes several runnable examples in the `examples_app` folder that demonstrate different capabilities using `cpal` for real-time audio output.

Run an example using:
```sh
cargo run --release -p infinitedsp-examples --bin <example_name>
```

### Available Examples:

*   **`infinitedsp_demo`**: A complex polyphonic demo showcasing 30 voices, filters, envelopes, and effects (Stereo).
*   **`filter_sweep`**: Compares `PredictiveLadderFilter` vs `LadderFilter` with an LFO sweep (Mono).
*   **`dual_mono_demo`**: Demonstrates independent processing of Left/Right channels (Ping-Pong Delay).
*   **`trance_synth`**: A massive stereo supersaw trance pluck with delay, reverb, and a sequencer.
*   **`karplus_demo`**: Physical modeling of a guitar string (Karplus-Strong algorithm) (Mono).
*   **`svf_demo`**: State Variable Filter demonstration (BandPass sweep).
*   **`spectral_demo`**: FFT-based Pitch Shifting using the Overlap-Add (OLA) engine.
*   **`granular_demo`**: Time-domain Granular Pitch Shifting.
*   **`modulation_demo`**: Showcases Tremolo, Chorus, and Tape Delay.
*   **`phaser_demo`**: 6-stage Phaser effect.
*   **`effects_demo`**: Demonstrates signal math (Add/Multiply) and distortion.

## Documentation

To generate and view the API documentation:

```sh
cargo doc --open
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
