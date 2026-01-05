use criterion::{criterion_group, criterion_main, Criterion};
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::parameter::Parameter;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_core::FrameProcessor;
use std::hint::black_box;

fn benchmark_oscillator(c: &mut Criterion) {
    let mut group = c.benchmark_group("Oscillator");
    let sample_rate = 44100.0;
    let buffer_size = 512;

    group.bench_function("Sine", |b| {
        let param = Parameter::new(440.0);
        let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::Sine);
        osc.set_sample_rate(sample_rate);
        let mut buffer = vec![0.0; buffer_size];

        b.iter(|| {
            osc.process(black_box(&mut buffer), 0);
        })
    });

    group.bench_function("Saw", |b| {
        let param = Parameter::new(440.0);
        let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::Saw);
        osc.set_sample_rate(sample_rate);
        let mut buffer = vec![0.0; buffer_size];

        b.iter(|| {
            osc.process(black_box(&mut buffer), 0);
        })
    });

    group.bench_function("Square", |b| {
        let param = Parameter::new(440.0);
        let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::Square);
        osc.set_sample_rate(sample_rate);
        let mut buffer = vec![0.0; buffer_size];

        b.iter(|| {
            osc.process(black_box(&mut buffer), 0);
        })
    });

    group.bench_function("Noise", |b| {
        let param = Parameter::new(440.0);
        let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::WhiteNoise);
        osc.set_sample_rate(sample_rate);
        let mut buffer = vec![0.0; buffer_size];

        b.iter(|| {
            osc.process(black_box(&mut buffer), 0);
        })
    });
    group.finish();
}

fn benchmark_reverb(c: &mut Criterion) {
    let mut group = c.benchmark_group("Reverb");
    let sample_rate = 44100.0;
    let buffer_size = 512;

    group.bench_function("Process Stereo", |b| {
        let mut reverb = Reverb::new();
        reverb.set_sample_rate(sample_rate);
        let mut buffer = vec![0.0; buffer_size * 2];

        b.iter(|| {
            reverb.process(black_box(&mut buffer), 0);
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_oscillator, benchmark_reverb);
criterion_main!(benches);
