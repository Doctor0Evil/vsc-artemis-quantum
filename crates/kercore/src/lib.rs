#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]

use core::f64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CorridorBands {
    pub var_id: &'static str,
    pub units: &'static str,
    pub safemax: f64,
    pub goldmax: f64,
    pub hardmax: f64,
    pub weight: f64,
    pub lyap_channel: u8,
    pub mandatory: bool,
}

impl CorridorBands {
    pub const fn new(
        var_id: &'static str,
        units: &'static str,
        safemax: f64,
        goldmax: f64,
        hardmax: f64,
        weight: f64,
        lyap_channel: u8,
        mandatory: bool,
    ) -> Self {
        Self { var_id, units, safemax, goldmax, hardmax, weight, lyap_channel, mandatory }
    }

    pub fn normalize(self, x: f64) -> f64 {
        if x <= self.safemax { 0.0 }
        else if x >= self.hardmax { 1.0 }
        else {
            let span = (self.hardmax - self.safemax).max(1e-9);
            ((x - self.safemax) / span).min(1.0).max(0.0)
        }
    }
}

#[derive(Clone, Debug)]
pub struct MetricFields {
    pub rx: Vec<f64>,
    pub vt: f64,
    pub vtnext: f64,
    pub k: f64,
    pub e: f64,
    pub r: f64,
}

impl MetricFields {
    pub fn new(rx: Vec<f64>, weights: &[f64]) -> Self {
        let vt = weights.iter().zip(rx.iter()).map(|(w, r)| w * r * r).sum();
        let r = rx.iter().cloned().fold(0.0, f64::max);
        Self { rx, vt, vtnext: vt, k: 1.0, e: (1.0 - r).max(0.0), r }
    }
}

pub fn compute_residual(weights: &[f64], rx: &[f64]) -> f64 {
    weights.iter().zip(rx.iter()).map(|(w, r)| w * r * r).sum()
}

pub fn ker_from_history(kt: &[f64], et: &[f64], rt: &[f64]) -> (f64, f64, f64) {
    let n = kt.len().max(1) as f64;
    (kt.iter().sum::<f64>() / n, et.iter().sum::<f64>() / n, rt.iter().sum::<f64>() / n)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorridorDecision { Ok, Derate, Stop }

pub struct SafeStepConfig { pub eps_vt: f64 }

pub fn safe_step(prev_vt: f64, next_vt: f64, cfg: &SafeStepConfig, rx: &[f64]) -> CorridorDecision {
    if rx.iter().any(|&r| r >= 1.0) { return CorridorDecision::Stop; }
    if next_vt <= prev_vt + cfg.eps_vt { CorridorDecision::Ok }
    else { CorridorDecision::Derate }
}
