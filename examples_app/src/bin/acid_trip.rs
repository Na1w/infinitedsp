use anyhow::Result;
use cpal::traits::StreamTrait;
use infinitedsp_core::core::audio_param::AudioParam;
use infinitedsp_core::core::channels::{Mono, Stereo, DualMono, MonoToStereo};
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::frame_processor::FrameProcessor;
use infinitedsp_core::core::ola::Ola;
use infinitedsp_core::core::parallel_mixer::ParallelMixer;
use infinitedsp_core::core::parameter::Parameter;
use infinitedsp_core::core::summing_mixer::SummingMixer;
use infinitedsp_core::effects::dynamics::compressor::Compressor;
use infinitedsp_core::effects::dynamics::distortion::{Distortion, DistortionType};
use infinitedsp_core::effects::filter::predictive_ladder::PredictiveLadderFilter;
use infinitedsp_core::effects::spectral::pitch_shift::FftPitchShift;
use infinitedsp_core::effects::time::delay::Delay;
use infinitedsp_core::effects::time::reverb::Reverb;
use infinitedsp_core::effects::utility::gain::Gain;
use infinitedsp_core::effects::utility::map_range::{CurveType, MapRange};
use infinitedsp_core::effects::utility::stereo_widener::StereoWidener;
use infinitedsp_core::synthesis::envelope::Adsr;
use infinitedsp_core::synthesis::lfo::{Lfo, LfoWaveform};
use infinitedsp_core::synthesis::oscillator::{Oscillator, Waveform};
use infinitedsp_examples::audio_backend::{init_audio_stereo, StereoProcessor};
use std::thread;
use std::time::Duration;

struct AcidTrip {
    synth_bus: Box<dyn FrameProcessor<Stereo> + Send>,
    drum_bus: Box<dyn FrameProcessor<Stereo> + Send>,
    master_reverb: ParallelMixer<Reverb, Stereo>,
    widener: StereoWidener,
    limiter: Compressor,
    pitch_param: Parameter,
    pitch_lfo: Lfo,
    mix_param: Parameter,
    mix_lfo: Lfo,
    p: Parameter, g: Parameter, a: Parameter, kg: Parameter, hg: Parameter,
    s_per_step: u64, cur_s: u64, step: usize, g_s: u64,
    t_p: f32, c_p: f32,
}

impl StereoProcessor for AcidTrip {
    fn process(&mut self, left: &mut [f32], right: &mut [f32], _: u64) {
        let n = [65.4, 65.4, 130.8, 65.4, 77.8, 65.4, 65.4, 98.0, 65.4, 130.8, 116.5, 65.4, 65.4, 77.8, 87.3, 116.5];
        let acc = [false, true, false, false, true, false, false, false, false, true, false, false, true, false, true, false];
        let sld = [false, false, true, false, false, false, false, true, false, false, true, false, false, false, false, true];
        
        for i in 0..left.len() {
            let s = self.step % 16;
            if self.cur_s == 0 {
                self.t_p = n[s];
                let prev_s = if s == 0 { 15 } else { s - 1 };
                if !sld[prev_s] { self.g.set(0.0); self.c_p = n[s]; } else { self.g.set(1.0); }
                self.a.set(if acc[s] { 1.0 } else { 0.0 });
                if s % 4 == 0 { self.kg.set(1.0); }
                if s % 2 == 1 { self.hg.set(1.0); }
            } else if self.cur_s == 1 {
                self.g.set(1.0);
            } else if self.cur_s == self.s_per_step / 2 {
                self.kg.set(0.0); self.hg.set(0.0);
            }
            
            let mut p_buf = [0.0; 1]; self.pitch_lfo.process(&mut p_buf, self.g_s);
            self.pitch_param.set(p_buf[0]);
            
            let mut m_buf = [0.0; 1]; self.mix_lfo.process(&mut m_buf, self.g_s);
            let mix_val = (m_buf[0] + 1.0) * 0.35;
            self.mix_param.set(mix_val);
            
            let prev_s = if s == 0 { 15 } else { s - 1 };
            let glide = if sld[prev_s] { 0.0022 } else { 0.15 };
            self.c_p += (self.t_p - self.c_p) * glide; self.p.set(self.c_p);
            
            let mut s_st = [0.0; 2]; self.synth_bus.process(&mut s_st, self.g_s);
            let mut d_st = [0.0; 2]; self.drum_bus.process(&mut d_st, self.g_s);
            let mut out = [s_st[0] + d_st[0], s_st[1] + d_st[1]];
            self.master_reverb.process(&mut out, self.g_s);
            self.widener.process(&mut out, self.g_s);
            
            left[i] = out[0]; right[i] = out[1];
            self.g_s += 1; self.cur_s += 1;
            if self.cur_s >= self.s_per_step { self.cur_s = 0; self.step += 1; }
        }
        self.limiter.process(left, 0); self.limiter.process(right, 0);
    }
}

fn create_acid_filter(g: Parameter, a: Parameter, lfo_rate: f32, sr: f32) -> Box<dyn FrameProcessor<Mono> + Send> {
    let f_env = Adsr::new(AudioParam::Linked(g), AudioParam::ms(1.0), AudioParam::ms(160.0), AudioParam::linear(0.0), AudioParam::ms(50.0));
    let mut f_lfo = Lfo::new(AudioParam::hz(lfo_rate), LfoWaveform::Sine); f_lfo.set_range(180.0, 1800.0);
    let f_sum = SummingMixer::<Mono, Box<dyn FrameProcessor<Mono> + Send>>::new(vec![Box::new(f_lfo), Box::new(MapRange::new(AudioParam::Dynamic(Box::new(f_env)), AudioParam::linear(0.0), AudioParam::linear(3500.0), CurveType::Linear))]);
    let mut r_lfo = Lfo::new(AudioParam::hz(0.1), LfoWaveform::Sine); r_lfo.set_range(0.86, 0.96);
    let r_sum = SummingMixer::<Mono, Box<dyn FrameProcessor<Mono> + Send>>::new(vec![Box::new(r_lfo), Box::new(MapRange::new(AudioParam::Linked(a), AudioParam::linear(0.0), AudioParam::linear(0.035), CurveType::Linear))]);
    let filter = PredictiveLadderFilter::new(AudioParam::Dynamic(Box::new(f_sum)), AudioParam::Dynamic(Box::new(r_sum)));
    Box::new(DspChain::new(filter, sr).and(Distortion::new(AudioParam::linear(2.2), AudioParam::linear(1.0), DistortionType::SoftClip)))
}

fn main() -> Result<()> {
    let (stream, _) = init_audio_stereo(move |sr| {
        let p = Parameter::new(65.4); let g = Parameter::new(0.0);
        let a = Parameter::new(0.0); let kg = Parameter::new(0.0); let hg = Parameter::new(0.0);
        let pitch_p = Parameter::new(24.0); let mix_p = Parameter::new(0.0);
        
        let synth_core = DspChain::new(Oscillator::new(AudioParam::Linked(p.clone()), Waveform::Saw), sr).and(create_acid_filter(g.clone(), a.clone(), 0.05, sr)).and(Gain::new(AudioParam::Dynamic(Box::new(Adsr::new(AudioParam::Linked(g.clone()), AudioParam::ms(2.0), AudioParam::ms(300.0), AudioParam::linear(0.0), AudioParam::ms(50.0))))));
        let fft_glitch = DspChain::new(Ola::<FftPitchShift<1024>, 1024>::with(FftPitchShift::new(AudioParam::Linked(pitch_p.clone()))), sr).and(Gain::new_db(6.0));
        let mut synth_mixer = ParallelMixer::new(0.0, fft_glitch);
        synth_mixer.set_mix(AudioParam::Linked(mix_p.clone()));
        
        let synth = DspChain::new(synth_core, sr).and(synth_mixer).and(Gain::new_db(4.0)).and(Compressor::new_limiter());
        let sc_map = MapRange::new(AudioParam::Dynamic(Box::new(Adsr::new(AudioParam::Linked(kg.clone()), AudioParam::ms(5.0), AudioParam::ms(150.0), AudioParam::linear(0.0), AudioParam::ms(10.0)))), AudioParam::linear(1.0), AudioParam::linear(0.5), CurveType::Linear);
        let synth_st = DspChain::new(MonoToStereo::new(synth), sr).and(Gain::new(AudioParam::Dynamic(Box::new(sc_map)))).and_mix(0.12, DualMono::new(Delay::new(1.0, AudioParam::ms(312.5), AudioParam::linear(0.5), AudioParam::linear(0.8)), Delay::new(1.0, AudioParam::ms(416.6), AudioParam::linear(0.4), AudioParam::linear(0.8))));
        
        let kick = DspChain::new(Oscillator::new(AudioParam::Dynamic(Box::new(MapRange::new(AudioParam::Dynamic(Box::new(Adsr::new(AudioParam::Linked(kg.clone()), AudioParam::ms(0.0), AudioParam::ms(50.0), AudioParam::linear(0.0), AudioParam::ms(10.0)))), AudioParam::hz(40.0), AudioParam::hz(130.0), CurveType::Exponential))), Waveform::Sine), sr).and(Gain::new(AudioParam::Dynamic(Box::new(Adsr::new(AudioParam::Linked(kg.clone()), AudioParam::ms(2.0), AudioParam::ms(200.0), AudioParam::linear(0.0), AudioParam::ms(10.0)))))).and(Gain::new_db(4.0));
        let hats = DspChain::new(Oscillator::new(AudioParam::Static(0.0), Waveform::WhiteNoise), sr).and(Gain::new(AudioParam::Dynamic(Box::new(Adsr::new(AudioParam::Linked(hg.clone()), AudioParam::ms(2.0), AudioParam::ms(40.0), AudioParam::linear(0.0), AudioParam::ms(10.0))))));
        let drum_bus = DspChain::new(MonoToStereo::new(SummingMixer::<Mono, Box<dyn FrameProcessor<Mono> + Send>>::new(vec![Box::new(kick), Box::new(hats)])), sr).and(Gain::new_db(-6.0));
        
        let mut pl = Lfo::new(AudioParam::hz(0.5), LfoWaveform::Sine); pl.set_range(12.0, 36.0);
        let mut ml = Lfo::new(AudioParam::hz(0.03), LfoWaveform::Saw); ml.set_range(-1.0, 1.0);
        
        let bpm = 140.0; let step_len_s = 60.0 / (bpm * 4.0);
        AcidTrip {
            synth_bus: Box::new(synth_st), drum_bus: Box::new(drum_bus),
            master_reverb: ParallelMixer::new(0.12, Reverb::new_with_params(AudioParam::linear(0.85), AudioParam::linear(0.25), 101)),
            widener: StereoWidener::new(AudioParam::linear(1.1)), limiter: Compressor::new_limiter(),
            pitch_param: pitch_p, pitch_lfo: pl, mix_param: mix_p, mix_lfo: ml,
            p, g, a, kg, hg, s_per_step: (sr * step_len_s as f32) as u64, cur_s: 0, step: 0, g_s: 0, t_p: 65.4, c_p: 65.4,
        }
    })?;
    stream.play()?;
    thread::sleep(Duration::from_secs(15));
    Ok(())
}
