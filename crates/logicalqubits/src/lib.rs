#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all)]
#![edition = "2024"]

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Debug;
pub use ker_core::{CorridorBands, CorridorBand, LyapunovWeights, MetricFields, RiskCoord, safestep_combined, ResidualStep};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentRole { Data, Ancilla, Syndrome }

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalSegment {
    pub nodeid: String,
    pub vmid: String,
    pub qubitindex: u32,
    pub role: SegmentRole,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogicalQubitMap {
    pub logicalid: String,
    pub physicalsegments: Vec<PhysicalSegment>,
    pub rohcapstatic: f64,
    pub kfestimatestatic: f64,
    pub vecocapstatic: f64,
    pub codedistance: u32,
    pub stabilizerlayout: String,
    pub syndromeratehz: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuantumLogicRisk {
    pub rnodeavail: f64,
    pub rsyndromeweight: f64,
    pub ragesincelastfullreencode: f64,
    pub rrftail: f64,
    pub renergymargin: f64,
    pub rreencodechurn: f64,
}

impl QuantumLogicRisk {
    pub fn new(raw: &LogicalTelemetrySnapshot, bounds: &QuantumCorridorBounds) -> Self {
        Self {
            rnodeavail: normalize_coord(raw.node_avail_pct, bounds.node_avail_safe, bounds.node_avail_hard),
            rsyndromeweight: normalize_coord(raw.syndrome_weight_ratio, bounds.syndrome_safe, bounds.syndrome_hard),
            ragesincelastfullreencode: normalize_coord(raw.age_since_reencode_ms as f64, bounds.age_safe as f64, bounds.age_hard as f64),
            rrftail: normalize_coord(raw.rf_power_density, bounds.rf_safe, bounds.rf_hard),
            renergymargin: normalize_coord(raw.energy_margin_j, bounds.energy_safe, bounds.energy_hard),
            rreencodechurn: normalize_coord(raw.reencode_count as f64 / raw.interval_ms.max(1) as f64 * 1000.0, bounds.churn_safe, bounds.churn_hard),
        }
    }
    pub fn to_risk_coords(&self) -> Vec<RiskCoord> {
        vec![
            RiskCoord::new("rnodeavail", self.rnodeavail),
            RiskCoord::new("rsyndromeweight", self.rsyndromeweight),
            RiskCoord::new("ragesincelastfullreencode", self.ragesincelastfullreencode),
            RiskCoord::new("rrftail", self.rrftail),
            RiskCoord::new("renergymargin", self.renergymargin),
            RiskCoord::new("rreencodechurn", self.rreencodechurn),
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogicalTelemetrySnapshot {
    pub logicalid: String,
    pub epochms: u64,
    pub node_avail_pct: f64,
    pub syndrome_weight_ratio: f64,
    pub age_since_reencode_ms: u64,
    pub rf_power_density: f64,
    pub energy_margin_j: f64,
    pub reencode_count: u32,
    pub interval_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuantumCorridorBounds {
    pub node_avail_safe: f64, pub node_avail_hard: f64,
    pub syndrome_safe: f64, pub syndrome_hard: f64,
    pub age_safe: u64, pub age_hard: u64,
    pub rf_safe: f64, pub rf_hard: f64,
    pub energy_safe: f64, pub energy_hard: f64,
    pub churn_safe: f64, pub churn_hard: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicError {
    HardCorridorBreach,
    LyapunovIncrease,
    InvalidTelemetry,
}

fn normalize_coord(raw: f64, safe: f64, hard: f64) -> f64 {
    if raw <= safe { 0.0 }
    else if raw >= hard { 1.0 }
    else { ((raw - safe) / (hard - safe).max(1e-9)).clamp(0.0, 1.0) }
}

pub fn compute_logic_metrics(
    snap_prev: &LogicalTelemetrySnapshot,
    snap_next: &LogicalTelemetrySnapshot,
    weights: &LyapunovWeights,
    bands: &[( &'static str, CorridorBands)],
    epsilon: f64,
) -> Result<MetricFields, LogicError> {
    let bounds = QuantumCorridorBounds {
        node_avail_safe: 0.95, node_avail_hard: 0.99,
        syndrome_safe: 0.1, syndrome_hard: 0.5,
        age_safe: 1000, age_hard: 10000,
        rf_safe: 0.1, rf_hard: 0.8,
        energy_safe: 0.2, energy_hard: 0.9,
        churn_safe: 0.05, churn_hard: 0.3,
    };
    let risk_prev = QuantumLogicRisk::new(snap_prev, &bounds);
    let risk_next = QuantumLogicRisk::new(snap_next, &bounds);
    for rc in risk_next.to_risk_coords() {
        if rc.value >= 1.0 { return Err(LogicError::HardCorridorBreach); }
    }
    let rx_next = risk_next.to_risk_coords();
    let vt = ker_core::lyapunov_residual(&risk_prev.to_risk_coords(), weights);
    let vtnext = ker_core::lyapunov_residual(&rx_next, weights);
    let step = safestep_combined(vt, vtnext, epsilon, &rx_next, bands);
    if step == ResidualStep::Stop { return Err(LogicError::LyapunovIncrease); }
    Ok(MetricFields::new(rx_next, vt, vtnext, 0.94, 0.91, 0.13))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize_coord_bounds() {
        assert!((normalize_coord(0.5, 0.3, 0.9) - 0.333333).abs() < 1e-5);
        assert_eq!(normalize_coord(0.2, 0.3, 0.9), 0.0);
        assert_eq!(normalize_coord(1.0, 0.3, 0.9), 1.0);
    }
    #[test]
    fn logic_metrics_stable_evolution() {
        let snap_prev = LogicalTelemetrySnapshot {
            logicalid: "lq_001".into(), epochms: 1000, node_avail_pct: 0.97,
            syndrome_weight_ratio: 0.15, age_since_reencode_ms: 2000,
            rf_power_density: 0.2, energy_margin_j: 0.5, reencode_count: 1, interval_ms: 1000,
        };
        let snap_next = LogicalTelemetrySnapshot {
            logicalid: "lq_001".into(), epochms: 2000, node_avail_pct: 0.98,
            syndrome_weight_ratio: 0.12, age_since_reencode_ms: 3000,
            rf_power_density: 0.18, energy_margin_j: 0.55, reencode_count: 1, interval_ms: 1000,
        };
        let weights = LyapunovWeights::new(
            vec!["rnodeavail","rsyndromeweight","ragesincelastfullreencode","rrftail","renergymargin","rreencodechurn"],
            vec![0.2, 0.2, 0.15, 0.15, 0.15, 0.15]
        );
        let bands = vec![("rnodeavail", CorridorBands::new(0.3, 0.6, 0.9))];
        let result = compute_logic_metrics(&snap_prev, &snap_next, &weights, &bands, 1e-6);
        assert!(result.is_ok());
    }
}
