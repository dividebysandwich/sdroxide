//! The transmit voice chain: everything between the mic-gain control and the
//! modulator.
//!
//! ```text
//! mic → [denoise] → Gate → EQ → De-esser → Compressor → Output (trim, limiter)
//!                   └──────── order chosen by the operator ────────┘
//! ```
//!
//! One owner, one thread: the engine builds a [`VoiceChain`], calls
//! [`VoiceChain::configure`] whenever the operator's [`TxVoiceState`] differs
//! from the one it was last built from, and runs each 10 ms block of
//! microphone audio through [`VoiceChain::process`]. 48 kHz mono `f32` in
//! place, the rate every voice path in the engine already works at.
//!
//! Retuning never resets a stage's filters or detectors — the operator adjusts
//! this while listening, and a reset on every frame of a slider drag is a click
//! on every frame. The one exception is a stage being switched *on*: its state
//! is whatever it was when it was last running, possibly minutes ago, so it
//! starts clean.

mod dynamics;

use sdroxide_types::{TxVoiceMeters, TxVoiceStage, TxVoiceState};

use crate::NeuralNr;
use crate::eq::ParametricEq;
use dynamics::{Compressor, DeEsser, Gate, Output, lin_to_db};

/// The rate the chain runs at.
pub const VOICE_CHAIN_RATE: f64 = 48_000.0;

pub struct VoiceChain {
    cfg: TxVoiceState,
    /// Built the first time it is asked for — RNNoise's state is not free and
    /// most operators never switch it on.
    denoise: Option<NeuralNr>,
    gate: Gate,
    eq: ParametricEq,
    deess: DeEsser,
    comp: Compressor,
    output: Output,
    in_peak: f32,
    out_peak: f32,
    metered: bool,
    /// Meter the audio even when no stage is on. See [`Self::set_meter_idle`].
    meter_idle: bool,
}

impl VoiceChain {
    pub fn new() -> Self {
        let mut chain = VoiceChain {
            cfg: TxVoiceState::default(),
            denoise: None,
            gate: Gate::new(),
            eq: ParametricEq::new(),
            deess: DeEsser::new(),
            comp: Compressor::new(),
            output: Output::new(),
            in_peak: 0.0,
            out_peak: 0.0,
            metered: false,
            meter_idle: false,
        };
        let cfg = chain.cfg;
        chain.retune(&cfg);
        chain
    }

    /// The settings the chain is running with.
    pub fn config(&self) -> &TxVoiceState {
        &self.cfg
    }

    /// Take on `cfg`. Cheap, but call it only when `cfg` has changed.
    pub fn configure(&mut self, cfg: &TxVoiceState) {
        let was = self.cfg;
        let started = |on: bool, was_on: bool| on && (!was_on || was.bypass);
        if started(cfg.gate.enabled, was.gate.enabled) {
            self.gate.reset();
        }
        if started(cfg.eq.enabled, was.eq.enabled) {
            self.eq.reset();
        }
        if started(cfg.deess.enabled, was.deess.enabled) {
            self.deess.reset();
        }
        if started(cfg.comp.enabled, was.comp.enabled) {
            self.comp.reset();
        }
        if started(cfg.output.limiter, was.output.limiter) {
            self.output.reset();
        }
        if started(cfg.denoise, was.denoise)
            && let Some(d) = self.denoise.as_mut()
        {
            d.reset();
        }
        self.retune(cfg);
    }

    fn retune(&mut self, cfg: &TxVoiceState) {
        let rate = VOICE_CHAIN_RATE;
        self.gate.configure(&cfg.gate, rate);
        self.eq.configure(&cfg.eq, rate);
        self.deess.configure(&cfg.deess, rate);
        self.comp.configure(&cfg.comp, rate);
        self.output.configure(&cfg.output, rate);
        if cfg.denoise && self.denoise.is_none() {
            let mut d = NeuralNr::new();
            d.set_rate(rate);
            self.denoise = Some(d);
        }
        self.cfg = *cfg;
    }

    /// Forget every stage's history, keeping the settings. Called at key-down
    /// so an over never opens on the tail of the last one.
    pub fn reset(&mut self) {
        self.gate.reset();
        self.eq.reset();
        self.deess.reset();
        self.comp.reset();
        self.output.reset();
        if let Some(d) = self.denoise.as_mut() {
            d.reset();
        }
    }

    /// Report levels from [`Self::take_meters`] even while the chain is a
    /// straight wire. For the TX Audio window's test chain, whose input meter
    /// is how an operator sets the microphone level before any stage is on;
    /// the chain on the air path leaves it off and reports nothing when idle.
    pub fn set_meter_idle(&mut self, on: bool) {
        self.meter_idle = on;
    }

    /// Run one block through the chain, in place.
    pub fn process(&mut self, audio: &mut [f32]) {
        let active = self.cfg.active();
        if !active && !self.meter_idle {
            return;
        }
        self.metered = true;
        self.in_peak = audio.iter().fold(self.in_peak, |m, s| m.max(s.abs()));
        if !active {
            self.out_peak = self.in_peak;
            return;
        }
        if self.cfg.denoise
            && let Some(d) = self.denoise.as_mut()
        {
            d.process(audio);
        }
        for stage in self.cfg.order {
            if !self.cfg.stage_enabled(stage) {
                continue;
            }
            match stage {
                TxVoiceStage::Gate => self.gate.process(audio),
                TxVoiceStage::Eq => self.eq.process(audio),
                TxVoiceStage::DeEss => self.deess.process(audio),
                TxVoiceStage::Comp => self.comp.process(audio),
            }
        }
        self.output.process(audio);
        // A NaN here would be carried by every stateful filter downstream —
        // the modulator, the resamplers — for the rest of the over.
        for s in audio.iter_mut() {
            if !s.is_finite() {
                *s = 0.0;
            }
        }
        self.out_peak = audio.iter().fold(self.out_peak, |m, s| m.max(s.abs()));
    }

    /// What the chain did since the last call, and start a new window. `None`
    /// when no audio has gone through it since — the chain was idle or
    /// switched out.
    pub fn take_meters(&mut self) -> Option<TxVoiceMeters> {
        if !std::mem::take(&mut self.metered) {
            return None;
        }
        let on = |enabled: bool, gr: &mut f32| {
            let v = std::mem::take(gr);
            if enabled { v } else { 0.0 }
        };
        let m = TxVoiceMeters {
            input_db: lin_to_db(std::mem::take(&mut self.in_peak)),
            output_db: lin_to_db(std::mem::take(&mut self.out_peak)),
            gate_gr_db: on(self.cfg.gate.enabled, &mut self.gate.gr_db),
            deess_gr_db: on(self.cfg.deess.enabled, &mut self.deess.gr_db),
            comp_gr_db: on(self.cfg.comp.enabled, &mut self.comp.gr_db),
            limiter_gr_db: on(self.cfg.output.limiter, &mut self.output.gr_db),
        };
        Some(m)
    }

    /// Delay the chain adds, in samples: the gate's and the limiter's
    /// look-ahead. RNNoise's frame delay comes on top while it is on.
    pub fn latency(&self) -> usize {
        if !self.cfg.active() {
            return 0;
        }
        let gate = if self.cfg.gate.enabled { self.gate.latency() } else { 0 };
        gate + self.output.latency()
    }
}

impl Default for VoiceChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
