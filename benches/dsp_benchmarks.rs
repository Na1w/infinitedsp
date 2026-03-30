use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::ola::Ola;
use infinitedsp_core::effects::filter::state_variable::{StateVariableFilter, SvfType};
use infinitedsp_core::effects::spectral::spectral_smear::SpectralSmear;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::synthesis::envelope::Adsr;
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_core::synthesis::speech::SpeechSynth;
use infinitedsp_core::synthesis::wavetable::{Wavetable, WavetableOscillator};
use infinitedsp_core::FrameProcessor;
use std::hint::black_box;

const SAMPLE_RATE: f32 = 44100.0;
const BUFFER_SIZE: usize = 512;

// --- Setup Functions ---

fn setup_adsr() -> (Adsr, Vec<f32>) {
    let mut adsr = Adsr::new(
        AudioParam::Static(1.0),
        AudioParam::Static(0.1),
        AudioParam::Static(0.1),
        AudioParam::Static(0.5),
        AudioParam::Static(0.2),
    );
    adsr.set_sample_rate(SAMPLE_RATE);
    (adsr, vec![0.0; BUFFER_SIZE])
}

fn setup_osc(wf: Waveform) -> (Oscillator, Vec<f32>) {
    let mut osc = Oscillator::new(AudioParam::hz(440.0), wf);
    osc.set_sample_rate(SAMPLE_RATE);
    (osc, vec![0.0; BUFFER_SIZE])
}

fn setup_wavetable_osc() -> (WavetableOscillator, Vec<f32>) {
    let size = 2048;
    let mut data = vec![0.0; size * 2];
    for i in 0..size {
        let t = i as f32 / size as f32;
        data[i] = libm::sinf(t * 2.0 * core::f32::consts::PI);
        data[size + i] = 2.0 * t - 1.0;
    }
    let table = Wavetable::new(&data, size);
    let mut osc = WavetableOscillator::new(table, AudioParam::hz(440.0), AudioParam::Static(0.5));
    osc.set_sample_rate(SAMPLE_RATE);
    (osc, vec![0.0; BUFFER_SIZE])
}

fn setup_reverb() -> (Reverb, Vec<f32>) {
    let mut reverb = Reverb::new();
    reverb.set_sample_rate(SAMPLE_RATE);
    (reverb, vec![0.0; BUFFER_SIZE * 2])
}

fn setup_svf() -> (StateVariableFilter, Vec<f32>) {
    let mut filter = StateVariableFilter::new(SvfType::LowPass, AudioParam::hz(1000.0), AudioParam::Static(0.7));
    filter.set_sample_rate(SAMPLE_RATE);
    (filter, vec![0.5; BUFFER_SIZE])
}

fn setup_ola_smear() -> (Ola<SpectralSmear<512>, 512>, Vec<f32>) {
    let smear_proc = SpectralSmear::<512>::new(AudioParam::Static(0.9));
    let mut smear = Ola::<_, 512>::with(smear_proc);
    smear.set_sample_rate(SAMPLE_RATE);
    (smear, vec![0.5; BUFFER_SIZE])
}

fn setup_speech() -> (SpeechSynth<'static>, Vec<f32>) {
    let mut speech = SpeechSynth::new(SAMPLE_RATE);
    let tokens = ["A", "E", "I", "O", "U"];
    let mut phonemes = Vec::new();
    for t in tokens {
        for p in infinitedsp_core::synthesis::speech::Phoneme::from_token(t) {
            phonemes.push(*p);
        }
    }
    let phonemes_static: &'static [_] = Box::leak(phonemes.into_boxed_slice());
    speech.set_phonemes(phonemes_static);
    (speech, vec![0.0; BUFFER_SIZE])
}

// --- Benchmarks ---

#[library_benchmark]
#[bench::default(setup_adsr())]
fn bench_adsr(args: (Adsr, Vec<f32>)) {
    let (mut adsr, mut buffer) = args;
    adsr.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
#[bench::sine(setup_osc(Waveform::Sine))]
#[bench::saw(setup_osc(Waveform::Saw))]
#[bench::square(setup_osc(Waveform::Square))]
#[bench::noise(setup_osc(Waveform::WhiteNoise))]
fn bench_oscillator(args: (Oscillator, Vec<f32>)) {
    let (mut osc, mut buffer) = args;
    osc.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
#[bench::default(setup_wavetable_osc())]
fn bench_wavetable_oscillator(args: (WavetableOscillator, Vec<f32>)) {
    let (mut osc, mut buffer) = args;
    osc.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
#[bench::default(setup_reverb())]
fn bench_reverb(args: (Reverb, Vec<f32>)) {
    let (mut reverb, mut buffer) = args;
    reverb.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
#[bench::default(setup_svf())]
fn bench_svf_lowpass(args: (StateVariableFilter, Vec<f32>)) {
    let (mut filter, mut buffer) = args;
    filter.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
#[bench::default(setup_ola_smear())]
fn bench_spectral_smear(args: (Ola<SpectralSmear<512>, 512>, Vec<f32>)) {
    let (mut smear, mut buffer) = args;
    smear.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
#[bench::default(setup_speech())]
fn bench_speech_synth(args: (SpeechSynth<'static>, Vec<f32>)) {
    let (mut speech, mut buffer) = args;
    speech.process(black_box(&mut buffer), 0);
}

library_benchmark_group!(
    name = oscillator;
    benchmarks = bench_oscillator, bench_wavetable_oscillator
);

library_benchmark_group!(
    name = effects;
    benchmarks = bench_reverb, bench_svf_lowpass, bench_spectral_smear
);

library_benchmark_group!(
    name = synthesis;
    benchmarks = bench_adsr, bench_speech_synth
);

main!(
    library_benchmark_groups = oscillator,
    effects,
    synthesis
);
