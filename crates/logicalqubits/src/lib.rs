#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Debug;

use kercore::{
    CorridorBands,
    MetricFields,
    LyapunovWeights,
    SafeStepConfig,
    CorridorDecision,
    compute_residual,
    lyapunov_residual,
    safe_step,
};

/// Role of a physical segment in a logical qubit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentRole {
    Data,
    Ancilla,
    Syndrome,
}

/// Mapping from logical qubit to physical segments.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalSegment {
    pub node_id: String,
    pub vm_id: String,
    pub qubit_index: u32,
    pub role: SegmentRole,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogicalQubitMap {
    pub logical_id: String,
    pub physical_segments: Vec<PhysicalSegment>,
    pub roh_cap_static: f64,
    pub kf_estimate_static: f64,
    pub veco_cap_static: f64,
    pub code_distance: u32,
    pub stabilizer_layout: String,
    pub syndrome_rate_hz: f64,
    pub corridor_bands: LogicalQubitCorridorBands,
}

/// Per-coordinate corridor bands for logical-qubit risk.
#[derive(Clone, Debug)]
pub struct LogicalQubitCorridorBands {
    pub node_avail: CorridorBands,
    pub syndrome_weight: CorridorBands,
    pub age_reencode: CorridorBands,
    pub rf_exposure: CorridorBands,
    pub energy_margin: CorridorBands,
    pub reencode_churn: CorridorBands,
}

impl LogicalQubitCorridorBands {
    /// Phoenix v1 calibration for normalized risk bands.
    pub fn phoenix_v1() -> Self {
        Self {
            node_avail: CorridorBands::new(
                "R_NODE_AVAIL",
                "dimless",
                0.0,
                0.10,
                0.25,
                1.0,
                3,
                true,
            ),
            syndrome_weight: CorridorBands::new(
                "R_SYNDROME",
                "dimless",
                0.0,
                0.15,
                0.30,
                1.5,
                3,
                true,
            ),
            age_reencode: CorridorBands::new(
                "R_AGE_REENC",
                "dimless",
                0.0,
                0.20,
                0.40,
                1.0,
                3,
                true,
            ),
            rf_exposure: CorridorBands::new(
                "R_RF_EXPOSURE",
                "dimless",
                0.0,
                0.12,
                0.25,
                2.0,
                3,
                true,
            ),
            energy_margin: CorridorBands::new(
                "R_ENERGY_MARG",
                "dimless",
                0.0,
                0.18,
                0.35,
                1.5,
                3,
                true,
            ),
            reencode_churn: CorridorBands::new(
                "R_REENC_CHURN",
                "dimless",
                0.0,
                0.14,
                0.28,
                1.5,
                3,
                true,
            ),
        }
    }
}

/// Raw telemetry snapshot for a logical-qubit controller epoch.
#[derive(Clone, Debug, PartialEq)]
pub struct LogicalTelemetrySnapshot {
    pub logical_id: String,
    pub epoch_ms: u64,
    pub node_avail_pct: f64,
    pub syndrome_weight_ratio: f64,
    pub age_since_reencode_ms: u64,
    pub rf_power_density: f64,
    pub energy_margin_j: f64,
    pub reencode_count: u32,
    pub interval_ms: u64,
}

/// Corridor bounds for normalizing raw coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuantumCorridorBounds {
    pub node_avail_safe: f64,
    pub node_avail_hard: f64,
    pub syndrome_safe: f64,
    pub syndrome_hard: f64,
    pub age_safe: u64,
    pub age_hard: u64,
    pub rf_safe: f64,
    pub rf_hard: f64,
    pub energy_safe: f64,
    pub energy_hard: f64,
    pub churn_safe: f64,
    pub churn_hard: f64,
}

/// Normalized risk coordinates for logical-qubit plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuantumLogicRisk {
    pub r_node_avail: f64,
    pub r_syndrome_weight: f64,
    pub r_age_since_reencode: f64,
    pub r_rf_tail: f64,
    pub r_energy_margin: f64,
    pub r_reencode_churn: f64,
}

impl QuantumLogicRisk {
    pub fn new(raw: &LogicalTelemetrySnapshot, bounds: &QuantumCorridorBounds) -> Self {
        Self {
            r_node_avail: normalize_coord(
                raw.node_avail_pct,
                bounds.node_avail_safe,
                bounds.node_avail_hard,
            ),
            r_syndrome_weight: normalize_coord(
                raw.syndrome_weight_ratio,
                bounds.syndrome_safe,
                bounds.syndrome_hard,
            ),
            r_age_since_reencode: normalize_coord(
                raw.age_since_reencode_ms as f64,
                bounds.age_safe as f64,
                bounds.age_hard as f64,
            ),
            r_rf_tail: normalize_coord(
                raw.rf_power_density,
                bounds.rf_safe,
                bounds.rf_hard,
            ),
            r_energy_margin: normalize_coord(
                raw.energy_margin_j,
                bounds.energy_safe,
                bounds.energy_hard,
            ),
            r_reencode_churn: normalize_coord(
                raw.reencode_count as f64 / raw.interval_ms.max(1) as f64 * 1000.0,
                bounds.churn_safe,
                bounds.churn_hard,
            ),
        }
    }

    pub fn as_vec(&self) -> Vec<f64> {
        vec![
            self.r_node_avail,
            self.r_syndrome_weight,
            self.r_age_since_reencode,
            self.r_rf_tail,
            self.r_energy_margin,
            self.r_reencode_churn,
        ]
    }
}

/// Logical-qubit risk mapped into MetricFields with corridors.
pub struct QuantumLogicRiskCoord {
    pub r_node_avail: f64,
    pub r_syndrome_weight: f64,
    pub r_age_since_reencode: f64,
    pub r_rf_exposure: f64,
    pub r_energy_margin: f64,
    pub r_reencode_churn: f64,
}

impl QuantumLogicRiskCoord {
    pub fn from_risk(r: &QuantumLogicRisk) -> Self {
        Self {
            r_node_avail: r.r_node_avail,
            r_syndrome_weight: r.r_syndrome_weight,
            r_age_since_reencode: r.r_age_since_reencode,
            r_rf_exposure: r.r_rf_tail,
            r_energy_margin: r.r_energy_margin,
            r_reencode_churn: r.r_reencode_churn,
        }
    }

    pub fn to_metric_fields(&self, bands: &LogicalQubitCorridorBands) -> MetricFields {
        let rx = vec![
            bands.node_avail.normalize(self.r_node_avail),
            bands.syndrome_weight.normalize(self.r_syndrome_weight),
            bands.age_reencode.normalize(self.r_age_since_reencode),
            bands.rf_exposure.normalize(self.r_rf_exposure),
            bands.energy_margin.normalize(self.r_energy_margin),
            bands.reencode_churn.normalize(self.r_reencode_churn),
        ];
        // Provisional weights; to be calibrated from telemetry.
        let weights = [1.0, 1.5, 1.0, 2.0, 1.5, 1.5];
        let vt = compute_residual(&weights, &rx);
        MetricFields::new(rx, vt, vt, 0.0, 0.0, 0.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicError {
    HardCorridorBreach,
    LyapunovIncrease,
    InvalidTelemetry,
}

fn normalize_coord(raw: f64, safe: f64, hard: f64) -> f64 {
    if raw <= safe {
        0.0
    } else if raw >= hard {
        1.0
    } else {
        let span = (hard - safe).max(1e-9);
        ((raw - safe) / span).clamp(0.0, 1.0)
    }
}

/// Lyapunov-safe evolution step for logical-qubit controller.
pub fn safesteplogic(
    prev: &MetricFields,
    next: &MetricFields,
    cfg: &SafeStepConfig,
) -> CorridorDecision {
    safe_step(prev.vt, next.vt, cfg, &next.rx)
}

/// Compute MetricFields for a logical-qubit step and enforce corridors + Lyapunov non-increase.
pub fn compute_logic_metrics(
    snap_prev: &LogicalTelemetrySnapshot,
    snap_next: &LogicalTelemetrySnapshot,
    weights: &LyapunovWeights,
    bands: &LogicalQubitCorridorBands,
    epsilon: f64,
) -> Result<MetricFields, LogicError> {
    // Static Phoenix v1 bounds; can be parameterized.
    let bounds = QuantumCorridorBounds {
        node_avail_safe: 0.95,
        node_avail_hard: 0.99,
        syndrome_safe: 0.1,
        syndrome_hard: 0.5,
        age_safe: 1000,
        age_hard: 10_000,
        rf_safe: 0.1,
        rf_hard: 0.8,
        energy_safe: 0.2,
        energy_hard: 0.9,
        churn_safe: 0.05,
        churn_hard: 0.3,
    };

    let risk_prev = QuantumLogicRisk::new(snap_prev, &bounds);
    let risk_next = QuantumLogicRisk::new(snap_next, &bounds);

    // Hard corridor breach check.
    for r in risk_next.as_vec().iter() {
        if *r >= 1.0 {
            return Err(LogicError::HardCorridorBreach);
        }
    }

    let rx_prev = risk_prev.as_vec();
    let rx_next = risk_next.as_vec();

    let vt = lyapunov_residual(&rx_prev, weights);
    let vtnext = lyapunov_residual(&rx_next, weights);

    let mut prev_fields = QuantumLogicRiskCoord::from_risk(&risk_prev).to_metric_fields(bands);
    prev_fields.vt = vt;
    prev_fields.vtnext = vt;

    let mut next_fields = QuantumLogicRiskCoord::from_risk(&risk_next).to_metric_fields(bands);
    next_fields.vt = vt;
    next_fields.vtnext = vtnext;

    let cfg = SafeStepConfig { epsilon };
    let decision = safesteplogic(&prev_fields, &next_fields, &cfg);

    if let CorridorDecision::Stop = decision {
        return Err(LogicError::LyapunovIncrease);
    }

    // K/E/R placeholders – to be filled by kercore KER scorer.
    Ok(MetricFields::new(rx_next, vt, vtnext, 0.94, 0.91, 0.13))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_coord_bounds() {
        let v = normalize_coord(0.5, 0.3, 0.9);
        assert!((v - 0.333_333).abs() < 1e-5);
        assert_eq!(normalize_coord(0.2, 0.3, 0.9), 0.0);
        assert_eq!(normalize_coord(1.0, 0.3, 0.9), 1.0);
    }

    #[test]
    fn logic_metrics_stable_evolution() {
        let snap_prev = LogicalTelemetrySnapshot {
            logical_id: "lq_001".into(),
            epoch_ms: 1000,
            node_avail_pct: 0.97,
            syndrome_weight_ratio: 0.15,
            age_since_reencode_ms: 2000,
            rf_power_density: 0.2,
            energy_margin_j: 0.5,
            reencode_count: 1,
            interval_ms: 1000,
        };
        let snap_next = LogicalTelemetrySnapshot {
            logical_id: "lq_001".into(),
            epoch_ms: 2000,
            node_avail_pct: 0.98,
            syndrome_weight_ratio: 0.12,
            age_since_reencode_ms: 3000,
            rf_power_density: 0.18,
            energy_margin_j: 0.55,
            reencode_count: 1,
            interval_ms: 1000,
        };

        let weights = LyapunovWeights::new(
            vec![
                "r_node_avail",
                "r_syndrome_weight",
                "r_age_since_reencode",
                "r_rf_tail",
                "r_energy_margin",
                "r_reencode_churn",
            ],
            vec![0.2, 0.2, 0.15, 0.15, 0.15, 0.15],
        );

        let bands = LogicalQubitCorridorBands::phoenix_v1();

        let result = compute_logic_metrics(&snap_prev, &snap_next, &weights, &bands, 1e-6);
        assert!(result.is_ok());
    }
}
