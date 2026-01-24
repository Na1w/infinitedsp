use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::parameter::Parameter;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::synthesis::envelope::Adsr;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_core::FrameProcessor;
use std::hint::black_box;

#[library_benchmark]
fn bench_adsr() {
    let sample_rate = 44100.0;
    let buffer_size = 512;

    // Set up parameters
    let gate = AudioParam::Static(1.0); // Gate On
    let attack = AudioParam::Static(0.1);
    let decay = AudioParam::Static(0.1);
    let sustain = AudioParam::Static(0.5);
    let release = AudioParam::Static(0.2);

    let mut adsr = Adsr::new(gate, attack, decay, sustain, release);
    adsr.set_sample_rate(sample_rate);

    let mut buffer = vec![0.0; buffer_size];
    adsr.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_oscillator_sine() {
    let sample_rate = 44100.0;
    let buffer_size = 512;
    let param = Parameter::new(440.0);
    let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::Sine);
    osc.set_sample_rate(sample_rate);
    let mut buffer = vec![0.0; buffer_size];
    osc.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_oscillator_saw() {
    let sample_rate = 44100.0;
    let buffer_size = 512;
    let param = Parameter::new(440.0);
    let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::Saw);
    osc.set_sample_rate(sample_rate);
    let mut buffer = vec![0.0; buffer_size];
    osc.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_oscillator_square() {
    let sample_rate = 44100.0;
    let buffer_size = 512;
    let param = Parameter::new(440.0);
    let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::Square);
    osc.set_sample_rate(sample_rate);
    let mut buffer = vec![0.0; buffer_size];
    osc.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_oscillator_noise() {
    let sample_rate = 44100.0;
    let buffer_size = 512;
    let param = Parameter::new(440.0);
    let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::WhiteNoise);
    osc.set_sample_rate(sample_rate);
    let mut buffer = vec![0.0; buffer_size];
    osc.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_reverb() {
    let sample_rate = 44100.0;
    let buffer_size = 512;
    let mut reverb = Reverb::new();
    reverb.set_sample_rate(sample_rate);
    let mut buffer = vec![0.0; buffer_size * 2];
    reverb.process(black_box(&mut buffer), 0);
}

library_benchmark_group!(
    name = oscillator;
    benchmarks = bench_oscillator_sine, bench_oscillator_saw, bench_oscillator_square, bench_oscillator_noise
);

library_benchmark_group!(
    name = reverb;
    benchmarks = bench_reverb
);

library_benchmark_group!(
    name = envelope;
    benchmarks = bench_adsr
);

main!(library_benchmark_groups = oscillator, reverb, envelope);
