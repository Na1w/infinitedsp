use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::{DualMono, Mono, Stereo};
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::frame_processor::FrameProcessor;
use infinitedsp_core::core::ola::Ola;
use infinitedsp_core::core::parallel_mixer::ParallelMixer;
use infinitedsp_core::core::parameter::Parameter;
use infinitedsp_core::effects::dynamics::compressor::Compressor;
use infinitedsp_core::effects::filter::ladder_filter::LadderFilter;
use infinitedsp_core::effects::spectral::granular_pitch::GranularPitchShift;
use infinitedsp_core::effects::spectral::pitch_shift::FftPitchShift;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::effects::time::tape_delay::TapeDelay;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::stereo_widener::StereoWidener;
use infinitedsp_core::synthesis::karplus_strong::KarplusStrong;
use infinitedsp_core::synthesis::lfo::{Lfo, LfoWaveform};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::{init_audio_stereo, StereoProcessor};
use std::thread;
use std::time::{Duration, Instant};

struct GranularCloud {
    arpeggio: DualMono<DspChain<Mono>, DspChain<Mono>>,
    drone: DspChain<Mono>,
    glitter: DspChain<Mono>,
    shimmer: Ola<FftPitchShift<1024>, 1024>,
    reverb: ParallelMixer<Reverb, Stereo>,
    widener: StereoWidener,
    limiter: Compressor,
    layer_buf: Vec<f32>,
    stereo_acc: Vec<f32>,
}

impl StereoProcessor for GranularCloud {
    fn process(&mut self, left: &mut [f32], right: &mut [f32], sample_index: u64) {
        let len = left.len();
        if self.layer_buf.len() < len {
            self.layer_buf.resize(len, 0.0);
            self.stereo_acc.resize(len * 2, 0.0);
        }

        self.stereo_acc.fill(0.0);

        self.layer_buf.fill(0.0);
        self.drone.process(&mut self.layer_buf, sample_index);
        for i in 0..len {
            self.stereo_acc[2 * i] += self.layer_buf[i];
            self.stereo_acc[2 * i + 1] += self.layer_buf[i];
        }

        let mut arp_buf = vec![0.0; len * 2];
        self.arpeggio.process(&mut arp_buf, sample_index);
        for (acc, arp) in self.stereo_acc.iter_mut().zip(&arp_buf) {
            *acc += *arp;
        }

        self.layer_buf.fill(0.0);
        self.glitter.process(&mut self.layer_buf, sample_index);
        for i in 0..len {
            let g = self.layer_buf[i] * 0.4;
            self.stereo_acc[2 * i] += g;
            self.stereo_acc[2 * i + 1] += g;
        }

        let mut shim_in = vec![0.0; len];
        for (i, shim) in shim_in.iter_mut().enumerate() {
            *shim = (self.stereo_acc[2 * i] + self.stereo_acc[2 * i + 1]) * 0.5;
        }
        self.shimmer.process(&mut shim_in, sample_index);
        for (i, shim) in shim_in.iter().enumerate() {
            let s = *shim * 0.6;
            self.stereo_acc[2 * i] += s;
            self.stereo_acc[2 * i + 1] += s;
        }

        self.reverb.process(&mut self.stereo_acc, sample_index);
        self.widener.process(&mut self.stereo_acc, sample_index);

        for i in 0..len {
            left[i] = self.stereo_acc[2 * i];
            right[i] = self.stereo_acc[2 * i + 1];
        }
        self.limiter.process(left, sample_index);
        self.limiter.process(right, sample_index);
    }
}

fn create_arp_voice(sr: f32, p: Parameter, g: Parameter, rate: f32, d: f32) -> DspChain<Mono> {
    let pluck = KarplusStrong::new(
        AudioParam::Linked(p),
        AudioParam::Linked(g),
        AudioParam::linear(0.05),
        AudioParam::linear(0.8),
    );
    let mut lfo = Lfo::new(AudioParam::hz(rate), LfoWaveform::Sine);
    lfo.set_range(400.0, 2000.0);
    let filter = LadderFilter::new(AudioParam::Dynamic(Box::new(lfo)), AudioParam::linear(0.7));
    let gran = GranularPitchShift::new(45.0, AudioParam::linear(12.0));
    let delay = TapeDelay::new(
        1.0,
        AudioParam::ms(d),
        AudioParam::linear(0.6),
        AudioParam::linear(0.4),
    );
    DspChain::new(pluck, sr)
        .and(filter)
        .and(gran)
        .and(delay)
        .and(Gain::new_db(-10.0))
}

fn create_drone(sr: f32) -> DspChain<Mono> {
    let osc1 = Oscillator::new(AudioParam::hz(55.0), Waveform::Saw);
    let osc2 = Oscillator::new(AudioParam::hz(55.2), Waveform::Saw);
    let mut lfo = Lfo::new(AudioParam::hz(0.05), LfoWaveform::Sine);
    lfo.set_range(80.0, 500.0);
    let filter = LadderFilter::new(AudioParam::Dynamic(Box::new(lfo)), AudioParam::linear(0.4));
    DspChain::new(osc1, sr)
        .and_mix(0.5, osc2)
        .and(filter)
        .and(Gain::new_db(-6.0))
}

fn create_glitter(sr: f32) -> DspChain<Mono> {
    let noise = Oscillator::new(AudioParam::Static(0.0), Waveform::WhiteNoise);
    let mut f_lfo = Lfo::new(AudioParam::hz(0.15), LfoWaveform::Sine);
    f_lfo.set_range(3000.0, 9000.0);
    let filter = LadderFilter::new(
        AudioParam::Dynamic(Box::new(f_lfo)),
        AudioParam::linear(0.96),
    );
    let mut p_lfo = Lfo::new(AudioParam::hz(15.0), LfoWaveform::SampleAndHold);
    p_lfo.set_range(0.0, 24.0);
    let gran = GranularPitchShift::new(15.0, AudioParam::Dynamic(Box::new(p_lfo)));
    DspChain::new(noise, sr)
        .and(filter)
        .and(gran)
        .and(Gain::new_db(-14.0))
}

fn main() -> Result<()> {
    let pitch = Parameter::new(110.0);
    let gate = Parameter::new(0.0);
    let p_clone = pitch.clone();
    let g_clone = gate.clone();
    let (stream, _sr) = init_audio_stereo(move |sr| {
        let left = create_arp_voice(sr, p_clone.clone(), g_clone.clone(), 0.1, 300.0);
        let right = create_arp_voice(sr, p_clone, g_clone, 0.12, 450.0);
        let shimmer_unit = FftPitchShift::<1024>::new(AudioParam::Static(12.0));
        let reverb_unit =
            Reverb::new_with_params(AudioParam::linear(0.94), AudioParam::linear(0.2), 42);
        GranularCloud {
            arpeggio: DualMono::new(left, right),
            drone: create_drone(sr),
            glitter: create_glitter(sr),
            shimmer: Ola::<_, 1024>::with(shimmer_unit),
            reverb: ParallelMixer::new(0.4, reverb_unit),
            widener: StereoWidener::new(AudioParam::linear(1.4)),
            limiter: Compressor::new_limiter(),
            layer_buf: Vec::new(),
            stereo_acc: Vec::new(),
        }
    })?;
    stream.play()?;
    let melody = [110.0, 130.81, 164.81, 196.0, 220.0, 196.0, 164.81, 130.81];
    let start = Instant::now();
    let mut step = 0;
    while start.elapsed() < Duration::from_secs(15) {
        pitch.set(melody[step]);
        gate.set(1.0);
        thread::sleep(Duration::from_millis(50));
        gate.set(0.0);
        step = (step + 1) % melody.len();
        thread::sleep(Duration::from_millis(200));
    }
    Ok(())
}
