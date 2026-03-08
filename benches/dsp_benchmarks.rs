use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::Mono;
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::ola::Ola;
use infinitedsp_core::core::parameter::Parameter;
use infinitedsp_core::core::static_dsp_chain::StaticDspChain;
use infinitedsp_core::effects::filter::biquad::{Biquad, FilterType as BiquadType};
use infinitedsp_core::effects::filter::ladder_filter::LadderFilter;
use infinitedsp_core::effects::filter::predictive_ladder::PredictiveLadderFilter;
use infinitedsp_core::effects::filter::state_variable::{StateVariableFilter, SvfType};
use infinitedsp_core::effects::spectral::pitch_shift::FftPitchShift;
use infinitedsp_core::effects::spectral::spectral_smear::SpectralSmear;
use infinitedsp_core::effects::time::ping_pong_delay::PingPongDelay;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::panner::StereoPanner;
use infinitedsp_core::synthesis::brass_model::BrassModel;
use infinitedsp_core::synthesis::envelope::Adsr;
use infinitedsp_core::synthesis::karplus_strong::KarplusStrong;
use infinitedsp_core::synthesis::lfo::{Lfo, LfoWaveform};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_core::synthesis::speech::SpeechSynth;
use infinitedsp_core::FrameProcessor;
use std::hint::black_box;

const SAMPLE_RATE: f32 = 44100.0;
const BUFFER_SIZE: usize = 512;

#[library_benchmark]
fn bench_adsr() {
    let gate = AudioParam::Static(1.0);
    let attack = AudioParam::Static(0.1);
    let decay = AudioParam::Static(0.1);
    let sustain = AudioParam::Static(0.5);
    let release = AudioParam::Static(0.2);
    let mut adsr = Adsr::new(gate, attack, decay, sustain, release);
    adsr.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE];
    adsr.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_oscillator_sine() {
    let param = Parameter::new(440.0);
    let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::Sine);
    osc.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE];
    osc.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_oscillator_saw() {
    let param = Parameter::new(440.0);
    let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::Saw);
    osc.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE];
    osc.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_oscillator_square() {
    let param = Parameter::new(440.0);
    let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::Square);
    osc.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE];
    osc.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_oscillator_noise() {
    let param = Parameter::new(440.0);
    let mut osc = Oscillator::new(AudioParam::Linked(param), Waveform::WhiteNoise);
    osc.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE];
    osc.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_reverb() {
    let mut reverb = Reverb::new();
    reverb.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE * 2];
    reverb.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_compressor() {
    let mut compressor = infinitedsp_core::effects::dynamics::compressor::Compressor::new(
        AudioParam::Static(-10.0),
        AudioParam::Static(4.0),
    );
    compressor.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.5; BUFFER_SIZE];
    compressor.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_svf_lowpass() {
    let mut filter = StateVariableFilter::new(SvfType::LowPass, AudioParam::hz(1000.0), AudioParam::Static(0.7));
    filter.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.5; BUFFER_SIZE];
    filter.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_ladder_lowpass() {
    let mut filter = LadderFilter::new(AudioParam::hz(1000.0), AudioParam::Static(0.7));
    filter.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.5; BUFFER_SIZE];
    filter.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_predictive_ladder_lowpass() {
    let mut filter = PredictiveLadderFilter::new(AudioParam::hz(1000.0), AudioParam::Static(0.7));
    filter.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.5; BUFFER_SIZE];
    filter.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_biquad_lowpass() {
    let mut filter = Biquad::new(BiquadType::LowPass, AudioParam::hz(1000.0), AudioParam::Static(0.7));
    filter.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.5; BUFFER_SIZE];
    filter.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_spectral_smear() {
    let smear_proc = SpectralSmear::<512>::new(AudioParam::Static(0.9));
    let mut smear = Ola::<_, 512>::with(smear_proc);
    smear.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.5; BUFFER_SIZE];
    smear.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_fft_pitch_shift() {
    let shift_proc = FftPitchShift::<512>::new(AudioParam::Static(7.0));
    let mut shift = Ola::<_, 512>::with(shift_proc);
    shift.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.5; BUFFER_SIZE];
    shift.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_dynamic_chain() {
    let osc = Oscillator::new(AudioParam::hz(440.0), Waveform::Sine);
    let filter = StateVariableFilter::new(SvfType::LowPass, AudioParam::hz(1000.0), AudioParam::Static(0.7));
    let gain = Gain::new_fixed(0.5);
    let mut chain = DspChain::new(osc, SAMPLE_RATE).and(filter).and(gain);
    let mut buffer = vec![0.0; BUFFER_SIZE];
    chain.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_static_chain() {
    let osc = Oscillator::new(AudioParam::hz(440.0), Waveform::Sine);
    let filter = StateVariableFilter::new(SvfType::LowPass, AudioParam::hz(1000.0), AudioParam::Static(0.7));
    let gain = Gain::new_fixed(0.5);
    let mut chain = StaticDspChain::<Mono, _>::new(osc, SAMPLE_RATE).and(filter).and(gain);
    let mut buffer = vec![0.0; BUFFER_SIZE];
    chain.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_speech_synth() {
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
    let mut buffer = vec![0.0; BUFFER_SIZE];
    speech.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_karplus_strong() {
    let mut ks = KarplusStrong::new(
        AudioParam::hz(440.0),
        AudioParam::Static(1.0),
        AudioParam::Static(0.98),
        AudioParam::Static(0.99),
    );
    ks.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE];
    ks.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_brass_model() {
    let mut brass = BrassModel::new(AudioParam::hz(440.0), AudioParam::Static(0.8), AudioParam::Static(0.5));
    brass.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE];
    brass.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_lfo_sine() {
    let mut lfo = Lfo::new(AudioParam::hz(1.0), LfoWaveform::Sine);
    lfo.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE];
    lfo.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_lfo_sh() {
    let mut lfo = Lfo::new(AudioParam::hz(10.0), LfoWaveform::SampleAndHold);
    lfo.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE];
    lfo.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_ping_pong_delay() {
    let mut delay = PingPongDelay::new(
        1.0,
        AudioParam::ms(250.0),
        AudioParam::Static(0.5),
        AudioParam::Static(0.5),
    );
    delay.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE * 2];
    delay.process(black_box(&mut buffer), 0);
}

#[library_benchmark]
fn bench_stereo_panner() {
    let mut panner = StereoPanner::new(AudioParam::Static(0.0));
    panner.set_sample_rate(SAMPLE_RATE);
    let mut buffer = vec![0.0; BUFFER_SIZE * 2];
    panner.process(black_box(&mut buffer), 0);
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

library_benchmark_group!(
    name = compressor;
    benchmarks = bench_compressor
);

library_benchmark_group!(
    name = filters;
    benchmarks = bench_svf_lowpass, bench_ladder_lowpass, bench_predictive_ladder_lowpass, bench_biquad_lowpass
);

library_benchmark_group!(
    name = spectral;
    benchmarks = bench_spectral_smear, bench_fft_pitch_shift
);

library_benchmark_group!(
    name = utility;
    benchmarks = bench_stereo_panner, bench_ping_pong_delay
);

library_benchmark_group!(
    name = chains;
    benchmarks = bench_dynamic_chain, bench_static_chain
);

library_benchmark_group!(
    name = synthesis_extended;
    benchmarks = bench_speech_synth, bench_karplus_strong, bench_brass_model
);

library_benchmark_group!(
    name = modulation;
    benchmarks = bench_lfo_sine, bench_lfo_sh
);

main!(
    library_benchmark_groups = oscillator,
    reverb,
    envelope,
    compressor,
    filters,
    spectral,
    utility,
    chains,
    synthesis_extended,
    modulation
);
