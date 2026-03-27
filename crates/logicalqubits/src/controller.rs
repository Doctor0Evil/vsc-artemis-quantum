#![forbid(unsafe_code)]
#![deny(warnings)]
#![edition = "2024"]

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use crate::{LogicalTelemetrySnapshot, QuantumLogicRisk, QuantumCorridorBounds, compute_logic_metrics, LogicError};
use ker_core::{CorridorBands, LyapunovWeights, MetricFields, safestep_combined, ResidualStep, CorridorBand};

pub trait SyndromeStream {
    fn next(&mut self, logical_id: &str) -> Option<SyndromeSample>;
}

pub trait NodeHealthStream {
    fn next(&mut self, nodeid: &str) -> Option<NodeHealthSample>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct SyndromeSample {
    pub logicalid: String,
    pub epochms: u64,
    pub weight_ratio: f64,
    pub syndrome_count: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeHealthSample {
    pub nodeid: String,
    pub epochms: u64,
    pub availability_pct: f64,
    pub rf_density: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlDecision { AcceptJob, ScheduleReencode, TriggerFailover }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlError {
    TelemetryFetchFailed,
    LyapunovViolation,
    CorridorBreach,
    LogicalQubitNotFound,
}

pub struct QuantumResilienceController {
    logical_states: BTreeMap<String, MetricFields>,
    corridor_bands: Vec<(&'static str, CorridorBands)>,
    weights: LyapunovWeights,
    epsilon: f64,
}

impl QuantumResilienceController {
    pub fn new(epsilon: f64) -> Self {
        Self {
            logical_states: BTreeMap::new(),
            corridor_bands: vec![
                ("rnodeavail", CorridorBands::new(0.3, 0.6, 0.9)),
                ("rsyndromeweight", CorridorBands::new(0.3, 0.6, 0.9)),
                ("ragesincelastfullreencode", CorridorBands::new(0.3, 0.6, 0.9)),
                ("rrftail", CorridorBands::new(0.3, 0.6, 0.9)),
                ("renergymargin", CorridorBands::new(0.3, 0.6, 0.9)),
                ("rreencodechurn", CorridorBands::new(0.3, 0.6, 0.9)),
            ],
            weights: LyapunovWeights::new(
                vec!["rnodeavail","rsyndromeweight","ragesincelastfullreencode","rrftail","renergymargin","rreencodechurn"],
                vec![0.2, 0.2, 0.15, 0.15, 0.15, 0.15]
            ),
            epsilon,
        }
    }
    pub fn step(&mut self, logical_id: &str, telem: &LogicalTelemetrySnapshot) -> Result<ControlDecision, ControlError> {
        let prev_metrics = self.logical_states.get(logical_id).cloned();
        let bounds = QuantumCorridorBounds {
            node_avail_safe: 0.95, node_avail_hard: 0.99,
            syndrome_safe: 0.1, syndrome_hard: 0.5,
            age_safe: 1000, age_hard: 10000,
            rf_safe: 0.1, rf_hard: 0.8,
            energy_safe: 0.2, energy_hard: 0.9,
            churn_safe: 0.05, churn_hard: 0.3,
        };
        let risk = QuantumLogicRisk::new(telem, &bounds);
        let rx = risk.to_risk_coords();
        let vt = prev_metrics.as_ref().map(|m| m.vt).unwrap_or(0.5);
        let vtnext = ker_core::lyapunov_residual(&rx, &self.weights);
        let step_result = safestep_combined(vt, vtnext, self.epsilon, &rx, &self.corridor_bands);
        let has_hard_breach = rx.iter().any(|rc| {
            self.corridor_bands.iter()
                .find(|(n, _)| *n == rc.name)
                .map(|(_, b)| b.band(rc.value) == CorridorBand::HardBreach)
                .unwrap_or(false)
        });
        let decision = match (step_result, has_hard_breach) {
            (ResidualStep::Stop, _) | (_, true) => ControlDecision::TriggerFailover,
            (ResidualStep::Derate, _) | (_, false) if risk.rreencodechurn > 0.6 => ControlDecision::ScheduleReencode,
            _ => ControlDecision::AcceptJob,
        };
        self.logical_states.insert(logical_id.to_string(), MetricFields::new(rx, vt, vtnext, 0.94, 0.91, 0.13));
        Ok(decision)
    }
    pub fn get_metrics(&self, logical_id: &str) -> Option<&MetricFields> {
        self.logical_states.get(logical_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_evolution_accepts_job() {
        let mut ctrl = QuantumResilienceController::new(1e-6);
        let telem = LogicalTelemetrySnapshot {
            logicalid: "lq_001".into(), epochms: 1000, node_avail_pct: 0.97,
            syndrome_weight_ratio: 0.15, age_since_reencode_ms: 2000,
            rf_power_density: 0.2, energy_margin_j: 0.5, reencode_count: 1, interval_ms: 1000,
        };
        let decision = ctrl.step("lq_001", &telem);
        assert_eq!(decision.unwrap(), ControlDecision::AcceptJob);
    }
    #[test]
    fn rf_breach_triggers_failover() {
        let mut ctrl = QuantumResilienceController::new(1e-6);
        let telem = LogicalTelemetrySnapshot {
            logicalid: "lq_001".into(), epochms: 1000, node_avail_pct: 0.50,
            syndrome_weight_ratio: 0.15, age_since_reencode_ms: 2000,
            rf_power_density: 0.95, energy_margin_j: 0.5, reencode_count: 1, interval_ms: 1000,
        };
        let decision = ctrl.step("lq_001", &telem);
        assert_eq!(decision.unwrap(), ControlDecision::TriggerFailover);
    }
}
