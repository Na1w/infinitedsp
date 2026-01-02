use crate::FrameProcessor;
use crate::core::audio_param::AudioParam;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq)]
enum AdsrState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// A handle to manually trigger an envelope.
#[derive(Clone)]
pub struct Trigger {
    flag: Arc<AtomicBool>,
}

impl Trigger {
    /// Fires the trigger.
    pub fn fire(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }
}

/// An ADSR (Attack, Decay, Sustain, Release) envelope generator.
///
/// Generates a control signal based on a gate input.
/// Time parameters are in seconds.
pub struct Adsr {
    gate: AudioParam,

    attack_time: AudioParam,
    decay_time: AudioParam,
    sustain_level: AudioParam,
    release_time: AudioParam,

    sample_rate: f32,
    state: AdsrState,
    current_level: f32,
    last_gate: f32,

    attack_step: f32,
    decay_coeff: f32,
    release_coeff: f32,

    last_attack: f32,
    last_decay: f32,
    last_release: f32,

    gate_buffer: Vec<f32>,
    attack_buffer: Vec<f32>,
    decay_buffer: Vec<f32>,
    sustain_buffer: Vec<f32>,
    release_buffer: Vec<f32>,

    retrigger: Arc<AtomicBool>,
}

impl Adsr {
    /// Creates a new ADSR envelope.
    ///
    /// # Arguments
    /// * `gate` - Gate signal (0.0 = off, 1.0 = on).
    /// * `attack_time` - Attack time in seconds.
    /// * `decay_time` - Decay time in seconds.
    /// * `sustain_level` - Sustain level (0.0 - 1.0).
    /// * `release_time` - Release time in seconds.
    pub fn new(gate: AudioParam, attack_time: AudioParam, decay_time: AudioParam, sustain_level: AudioParam, release_time: AudioParam) -> Self {
        let mut adsr = Adsr {
            gate,
            attack_time,
            decay_time,
            sustain_level,
            release_time,
            sample_rate: 44100.0,
            state: AdsrState::Idle,
            current_level: 0.0,
            last_gate: 0.0,
            attack_step: 0.0,
            decay_coeff: 0.0,
            release_coeff: 0.0,
            last_attack: -1.0,
            last_decay: -1.0,
            last_release: -1.0,
            gate_buffer: Vec::new(),
            attack_buffer: Vec::new(),
            decay_buffer: Vec::new(),
            sustain_buffer: Vec::new(),
            release_buffer: Vec::new(),
            retrigger: Arc::new(AtomicBool::new(false)),
        };
        adsr.recalc(0.01, 0.1, 0.1); // Initial dummy recalc
        adsr
    }

    /// Creates a trigger handle for this envelope.
    /// Use this to manually retrigger the envelope from any thread.
    pub fn create_trigger(&self) -> Trigger {
        Trigger {
            flag: self.retrigger.clone(),
        }
    }

    fn recalc(&mut self, attack: f32, decay: f32, release: f32) {
        if (attack - self.last_attack).abs() > 0.0001 {
            let attack_samples = attack * self.sample_rate;
            self.attack_step = if attack_samples > 0.0 { 1.0 / attack_samples } else { 1.0 };
            self.last_attack = attack;
        }

        if (decay - self.last_decay).abs() > 0.0001 {
            let decay_samples = decay * self.sample_rate;
            self.decay_coeff = if decay_samples > 0.0 {
                // libm::expf
                libm::expf(-1.0 / (decay_samples / 3.0))
            } else { 0.0 };
            self.last_decay = decay;
        }

        if (release - self.last_release).abs() > 0.0001 {
            let release_samples = release * self.sample_rate;
            self.release_coeff = if release_samples > 0.0 {
                // libm::expf
                libm::expf(-1.0 / (release_samples / 3.0))
            } else { 0.0 };
            self.last_release = release;
        }
    }

    /// Sets the attack time parameter (seconds).
    pub fn set_attack(&mut self, time: AudioParam) { self.attack_time = time; }
    /// Sets the decay time parameter (seconds).
    pub fn set_decay(&mut self, time: AudioParam) { self.decay_time = time; }
    /// Sets the sustain level parameter.
    pub fn set_sustain(&mut self, level: AudioParam) { self.sustain_level = level; }
    /// Sets the release time parameter (seconds).
    pub fn set_release(&mut self, time: AudioParam) { self.release_time = time; }
}

impl FrameProcessor for Adsr {
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        let len = buffer.len();

        if self.gate_buffer.len() < len { self.gate_buffer.resize(len, 0.0); }
        if self.attack_buffer.len() < len { self.attack_buffer.resize(len, 0.0); }
        if self.decay_buffer.len() < len { self.decay_buffer.resize(len, 0.0); }
        if self.sustain_buffer.len() < len { self.sustain_buffer.resize(len, 0.0); }
        if self.release_buffer.len() < len { self.release_buffer.resize(len, 0.0); }

        self.gate_buffer.fill(0.0);
        self.attack_buffer.fill(0.0);
        self.decay_buffer.fill(0.0);
        self.sustain_buffer.fill(0.0);
        self.release_buffer.fill(0.0);

        self.gate.process(&mut self.gate_buffer[0..len], sample_index);
        self.attack_time.process(&mut self.attack_buffer[0..len], sample_index);
        self.decay_time.process(&mut self.decay_buffer[0..len], sample_index);
        self.sustain_level.process(&mut self.sustain_buffer[0..len], sample_index);
        self.release_time.process(&mut self.release_buffer[0..len], sample_index);

        // Check for manual retrigger
        let mut triggered = false;
        if self.retrigger.load(Ordering::Relaxed) {
            self.retrigger.store(false, Ordering::Relaxed);
            triggered = true;
        }

        for (i, sample) in buffer.iter_mut().enumerate() {
            let gate_val = self.gate_buffer[i];
            let attack = self.attack_buffer[i];
            let decay = self.decay_buffer[i];
            let sustain = self.sustain_buffer[i];
            let release = self.release_buffer[i];

            self.recalc(attack, decay, release);

            if triggered {
                self.state = AdsrState::Attack;
                self.current_level = 0.0; // Reset level on retrigger
                triggered = false; // Only trigger once per block/event
            } else if gate_val >= 0.5 && self.last_gate < 0.5 {
                self.state = AdsrState::Attack;
            } else if gate_val < 0.5 && self.last_gate >= 0.5 {
                self.state = AdsrState::Release;
            }
            self.last_gate = gate_val;

            match self.state {
                AdsrState::Idle => {
                    self.current_level = 0.0;
                },
                AdsrState::Attack => {
                    self.current_level += self.attack_step;
                    if self.current_level >= 1.0 {
                        self.current_level = 1.0;
                        self.state = AdsrState::Decay;
                    }
                },
                AdsrState::Decay => {
                    self.current_level = sustain + (self.current_level - sustain) * self.decay_coeff;
                    if (self.current_level - sustain).abs() < 0.001 {
                        self.current_level = sustain;
                        self.state = AdsrState::Sustain;
                    }
                },
                AdsrState::Sustain => {
                    self.current_level = sustain;
                },
                AdsrState::Release => {
                    self.current_level *= self.release_coeff;
                    if self.current_level < 0.0001 {
                        self.current_level = 0.0;
                        self.state = AdsrState::Idle;
                    }
                }
            }

            *sample = self.current_level;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.gate.set_sample_rate(sample_rate);
        self.attack_time.set_sample_rate(sample_rate);
        self.decay_time.set_sample_rate(sample_rate);
        self.sustain_level.set_sample_rate(sample_rate);
        self.release_time.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Adsr Envelope"
    }
}
