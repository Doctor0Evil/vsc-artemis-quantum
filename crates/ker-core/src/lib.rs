#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all)]
#![edition = "2024"]

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiskCoord {
    pub name: &'static str,
    pub value: f64,
}

impl RiskCoord {
    #[inline]
    pub fn new(name: &'static str, raw: f64) -> Self {
        Self { name, value: raw.clamp(0.0, 1.0) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CorridorBands {
    pub safe: f64,
    pub gold: f64,
    pub hard: f64,
}

impl CorridorBands {
    #[inline]
    pub fn new(safe: f64, gold: f64, hard: f64) -> Self {
        assert!(safe >= 0.0 && safe <= 1.0);
        assert!(gold >= safe && gold <= 1.0);
        assert!(hard >= gold && hard <= 1.0);
        Self { safe, gold, hard }
    }

    #[inline]
    pub fn band(&self, r: f64) -> CorridorBand {
        let r = r.clamp(0.0, 1.0);
        if r <= self.safe { CorridorBand::Safe }
        else if r <= self.gold { CorridorBand::Gold }
        else if r <= self.hard { CorridorBand::NearHard }
        else { CorridorBand::HardBreach }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorridorBand { Safe, Gold, NearHard, HardBreach }

#[derive(Clone, Debug)]
pub struct LyapunovWeights {
    pub names: Vec<&'static str>,
    pub weights: Vec<f64>,
}

impl LyapunovWeights {
    #[inline]
    pub fn new(names: Vec<&'static str>, weights: Vec<f64>) -> Self {
        assert_eq!(names.len(), weights.len());
        for w in &weights { assert!(*w >= 0.0); }
        Self { names, weights }
    }

    #[inline]
    pub fn weight_for(&self, name: &str) -> f64 {
        self.names.iter().zip(self.weights.iter())
            .find(|(n, _)| **n == name).map(|(_, w)| *w).unwrap_or(0.0)
    }
}

#[derive(Clone, Debug)]
pub struct MetricFields {
    pub rx: Vec<RiskCoord>,
    pub vt: f64,
    pub vtnext: f64,
    pub k: f64,
    pub e: f64,
    pub r: f64,
}

impl MetricFields {
    #[inline]
    pub fn new(rx: Vec<RiskCoord>, vt: f64, vtnext: f64, k: f64, e: f64, r: f64) -> Self {
        Self { rx, vt, vtnext, k, e, r }
    }
}

#[inline]
pub fn lyapunov_residual(rx: &[RiskCoord], weights: &LyapunovWeights) -> f64 {
    rx.iter().map(|c| { let w = weights.weight_for(c.name); w * c.value * c.value }).sum()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidualStep { Accept, Derate, Stop }

#[inline]
pub fn safestep_residual(vt: f64, vtnext: f64, epsilon: f64) -> ResidualStep {
    if vtnext > vt + epsilon { ResidualStep::Stop }
    else if (vtnext - vt).abs() <= epsilon { ResidualStep::Derate }
    else { ResidualStep::Accept }
}

#[inline]
pub fn check_corridors(rx: &[RiskCoord], bands: &[(&'static str, CorridorBands)]) -> ResidualStep {
    for coord in rx {
        if let Some((_, b)) = bands.iter().find(|(n, _)| *n == coord.name) {
            if b.band(coord.value) == CorridorBand::HardBreach { return ResidualStep::Stop; }
        }
    }
    ResidualStep::Accept
}

#[inline]
pub fn safestep_combined(vt: f64, vtnext: f64, epsilon: f64, rx: &[RiskCoord], bands: &[(&'static str, CorridorBands)]) -> ResidualStep {
    if check_corridors(rx, bands) == ResidualStep::Stop { return ResidualStep::Stop; }
    safestep_residual(vt, vtnext, epsilon)
}

#[inline]
pub fn update_ker(k_old: f64, e_old: f64, r_old: f64, k_new: f64, e_new: f64, r_new: f64) -> (f64, f64, f64) {
    (k_new.max(k_old), e_new.max(e_old), r_new.min(r_old))
}

impl fmt::Display for MetricFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "MetricFields")?;
        writeln!(f, "  vt     = {:.6}", self.vt)?;
        writeln!(f, "  vtnext = {:.6}", self.vtnext)?;
        writeln!(f, "  KER    = {:.4}, {:.4}, {:.4}", self.k, self.e, self.r)?;
        for coord in &self.rx { writeln!(f, "    {:>16} = {:.6}", coord.name, coord.value)?; }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn residual_nonnegative() {
        let rx = vec![RiskCoord::new("renergy", 0.3), RiskCoord::new("rrf", 0.2)];
        let weights = LyapunovWeights::new(vec!["renergy", "rrf"], vec![1.0, 0.5]);
        assert!(lyapunov_residual(&rx, &weights) >= 0.0);
    }
    #[test]
    fn safestep_combined_behavior() {
        let rx = vec![RiskCoord::new("renergy", 0.8)];
        let bands = vec![("renergy", CorridorBands::new(0.3, 0.6, 0.9))];
        assert_eq!(safestep_combined(0.5, 0.49, 1e-6, &rx, &bands), ResidualStep::Accept);
        assert_eq!(safestep_combined(0.5, 0.6, 1e-6, &rx, &bands), ResidualStep::Stop);
    }
}
