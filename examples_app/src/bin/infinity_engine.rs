use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::{Mono, Stereo};
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::frame_processor::FrameProcessor;
use infinitedsp_core::core::ola::Ola;
use infinitedsp_core::core::parallel_mixer::ParallelMixer;
use infinitedsp_core::core::parameter::Parameter;
use infinitedsp_core::effects::dynamics::compressor::Compressor;
use infinitedsp_core::effects::filter::ladder_filter::LadderFilter;
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

struct InfinityEngine {
    star_pluck: DspChain<Mono>,
    drone_voice: DspChain<Mono>,
    shimmer_bus: DspChain<Mono>,
    master_reverb: ParallelMixer<Reverb, Stereo>,
    widener: StereoWidener,
    limiter: Compressor,
    mixing_buffer: Vec<f32>,
    shimmer_buffer: Vec<f32>,
}

impl StereoProcessor for InfinityEngine {
    fn process(&mut self, left: &mut [f32], right: &mut [f32], sample_index: u64) {
        let len = left.len();
        if self.mixing_buffer.len() < len {
            self.mixing_buffer.resize(len, 0.0);
            self.shimmer_buffer.resize(len, 0.0);
        }

        self.mixing_buffer.fill(0.0);
        self.drone_voice
            .process(&mut self.mixing_buffer, sample_index);

        left[..len].copy_from_slice(&self.mixing_buffer[..len]);
        right[..len].copy_from_slice(&self.mixing_buffer[..len]);

        self.mixing_buffer.fill(0.0);
        self.star_pluck
            .process(&mut self.mixing_buffer, sample_index);

        for i in 0..len {
            left[i] += self.mixing_buffer[i] * 0.6;
            right[i] += self.mixing_buffer[i] * 0.6;
        }

        self.shimmer_buffer.copy_from_slice(&self.mixing_buffer);
        self.shimmer_bus
            .process(&mut self.shimmer_buffer, sample_index);

        let mut stereo_buf = vec![0.0; len * 2];
        for i in 0..len {
            let s = self.shimmer_buffer[i] * 0.3;
            stereo_buf[2 * i] = left[i] + s;
            stereo_buf[2 * i + 1] = right[i] + s;
        }

        self.master_reverb.process(&mut stereo_buf, sample_index);
        self.widener.process(&mut stereo_buf, sample_index);

        for i in 0..len {
            left[i] = stereo_buf[2 * i];
            right[i] = stereo_buf[2 * i + 1];
        }

        self.limiter.process(left, sample_index);
        self.limiter.process(right, sample_index);
    }
}

fn create_drone(sample_rate: f32) -> DspChain<Mono> {
    let mut lfo = Lfo::new(AudioParam::Static(0.05), LfoWaveform::Sine);
    lfo.set_range(200.0, 800.0);

    let osc = Oscillator::new(AudioParam::Static(60.0), Waveform::Saw);
    let filter = LadderFilter::new(AudioParam::Dynamic(Box::new(lfo)), AudioParam::Static(0.4));

    DspChain::new(osc, sample_rate)
        .and(filter)
        .and(Gain::new_db(-20.0))
}

fn create_star_pluck(sample_rate: f32, pitch: Parameter, gate: Parameter) -> DspChain<Mono> {
    let pluck = KarplusStrong::new(
        AudioParam::Linked(pitch),
        AudioParam::Linked(gate),
        AudioParam::Static(0.1),
        AudioParam::Static(0.5),
    );

    let delay = TapeDelay::new(
        2.0,
        AudioParam::Static(0.6),
        AudioParam::Static(0.4),
        AudioParam::Static(1.0),
    );

    DspChain::new(pluck, sample_rate).and_mix(0.5, delay)
}

fn main() -> Result<()> {
    let pluck_trigger = Parameter::new(0.0);
    let pluck_pitch = Parameter::new(440.0);

    let pt_for_engine = pluck_trigger.clone();
    let pp_for_engine = pluck_pitch.clone();

    let (stream, _sample_rate) = init_audio_stereo(move |sr| {
        let shimmer_pitch = FftPitchShift::<1024>::new(AudioParam::Static(12.0));
        let shimmer_ola = Ola::<_, 1024>::with(shimmer_pitch);
        let reverb_unit =
            Reverb::new_with_params(AudioParam::Static(0.95), AudioParam::Static(0.1), 42);

        InfinityEngine {
            star_pluck: create_star_pluck(sr, pp_for_engine.clone(), pt_for_engine.clone()),
            drone_voice: create_drone(sr),
            shimmer_bus: DspChain::new(shimmer_ola, sr).and(Gain::new_db(-6.0)),
            master_reverb: ParallelMixer::new(0.4, reverb_unit),
            widener: StereoWidener::new(AudioParam::Static(1.5)),
            limiter: Compressor::new_limiter(),
            mixing_buffer: Vec::new(),
            shimmer_buffer: Vec::new(),
        }
    })?;

    stream.play()?;

    let scale = [55.0, 58.27, 61.74, 65.41, 73.42, 82.41, 87.31, 98.0];
    let mut rng_state: u32 = 12345;
    let start = Instant::now();

    while start.elapsed() < Duration::from_secs(15) {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let r = (rng_state >> 16) & 0xFF;
        let note_idx = (r as usize) % scale.len();
        let octave = if r > 200 {
            4.0
        } else if r > 100 {
            3.0
        } else {
            2.0
        };
        let freq = scale[note_idx] * octave;

        pluck_pitch.set(freq);
        pluck_trigger.set(1.0);
        thread::sleep(Duration::from_millis(50));
        pluck_trigger.set(0.0);

        let wait = 500 + (r as u64) * 10;
        thread::sleep(Duration::from_millis(wait));
    }

    Ok(())
}
