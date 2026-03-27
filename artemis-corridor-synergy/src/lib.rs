#![deny(warnings, unsafe_code, clippy::all)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_uint, c_ulong, c_uchar};
use std::ptr;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelStatus { Ok = 0, InvalidWeight = 1, InvalidScore = 2, CostOverflow = 3, KERViolation = 4 }

#[repr(C)]
pub struct CorridorStateFFI {
    pub corridor_id: *const c_char,
    pub lat: c_double,
    pub lon: c_double,
    pub eco_scores: [c_double; 5],
    pub K: c_double,
    pub E: c_double,
    pub R: c_double,
    pub timestamp_unix: c_ulong,
    pub evidence_hex: *const c_char,
}

#[repr(C)]
pub struct InterventionDefFFI {
    pub intervention_id: *const c_char,
    pub intervention_type: *const c_char,
    pub response_coeffs: [c_double; 5],
    pub cost_usd: c_double,
    pub energy_kwh: c_double,
    pub land_m2: c_double,
    pub ker_delta: [c_double; 3],
    pub linked_evidence_hex: *const c_char,
}

#[repr(C)]
pub struct RankedInterventionFFI {
    pub corridor_id: *const c_char,
    pub intervention_id: *const c_char,
    pub synergy_gain: c_double,
    pub cost_usd: c_double,
    pub efficiency_ratio: c_double,
    pub projected_KER: [c_double; 3],
    pub safety_flags: c_uchar,
}

#[repr(C)]
pub struct KernelConfigFFI {
    pub weights: [c_double; 5],
    pub max_cost_per_corridor: c_double,
    pub min_KER_K: c_double,
    pub max_KER_R: c_double,
    pub enforce_monotonicity: bool,
    pub config_version: *const c_char,
    pub valid_from_unix: c_ulong,
    pub valid_until_unix: c_ulong,
}

#[repr(C)]
pub struct KernelHandle { _private: [u8; 0] }

extern "C" {
    fn artemis_kernel_create(config: *const KernelConfigFFI) -> *mut KernelHandle;
    fn artemis_kernel_destroy(handle: *mut KernelHandle);
    fn artemis_kernel_validate_config(handle: *const KernelHandle) -> c_uint;
    fn artemis_kernel_validate_corridor(handle: *const KernelHandle, corridor: *const CorridorStateFFI) -> c_uint;
    fn artemis_kernel_validate_intervention(handle: *const KernelHandle, intervention: *const InterventionDefFFI) -> c_uint;
    fn artemis_kernel_compute_synergy(handle: *const KernelHandle, corridor: *const CorridorStateFFI) -> c_double;
    fn artemis_kernel_compute_marginal(handle: *const KernelHandle, corridor: *const CorridorStateFFI, intervention: *const InterventionDefFFI) -> c_double;
    fn artemis_kernel_rank_interventions(
        handle: *const KernelHandle,
        corridors: *const CorridorStateFFI,
        corridor_count: c_uint,
        interventions: *const InterventionDefFFI,
        intervention_count: c_uint,
        max_results: c_uint,
        out_count: *mut c_uint,
    ) -> *mut RankedInterventionFFI;
    fn artemis_kernel_get_version() -> *const c_char;
    fn artemis_kernel_get_hash() -> *const c_char;
}

pub struct CorridorSynergyKernel {
    handle: *mut KernelHandle,
}

unsafe impl Send for CorridorSynergyKernel {}
unsafe impl Sync for CorridorSynergyKernel {}

impl CorridorSynergyKernel {
    pub fn new(config: KernelConfig) -> Result<Self, KernelStatus> {
        let config_ffi = config.to_ffi()?;
        let handle = unsafe { artemis_kernel_create(&config_ffi) };
        if handle.is_null() { return Err(KernelStatus::InvalidWeight); }
        let kernel = Self { handle };
        let status = unsafe { artemis_kernel_validate_config(handle) };
        if status != KernelStatus::Ok as u32 { return Err(status as KernelStatus); }
        Ok(kernel)
    }

    pub fn validate_corridor(&self, corridor: &CorridorState) -> Result<(), KernelStatus> {
        let corridor_ffi = corridor.to_ffi()?;
        let status = unsafe { artemis_kernel_validate_corridor(self.handle, &corridor_ffi) };
        if status != KernelStatus::Ok as u32 { return Err(status as KernelStatus); }
        Ok(())
    }

    pub fn validate_intervention(&self, intervention: &InterventionDef) -> Result<(), KernelStatus> {
        let intervention_ffi = intervention.to_ffi()?;
        let status = unsafe { artemis_kernel_validate_intervention(self.handle, &intervention_ffi) };
        if status != KernelStatus::Ok as u32 { return Err(status as KernelStatus); }
        Ok(())
    }

    pub fn compute_synergy_score(&self, corridor: &CorridorState) -> Result<f64, KernelStatus> {
        let corridor_ffi = corridor.to_ffi()?;
        let score = unsafe { artemis_kernel_compute_synergy(self.handle, &corridor_ffi) };
        if !score.is_finite() { return Err(KernelStatus::InvalidScore); }
        Ok(score)
    }

    pub fn compute_marginal_gain(&self, corridor: &CorridorState, intervention: &InterventionDef) -> Result<f64, KernelStatus> {
        let corridor_ffi = corridor.to_ffi()?;
        let intervention_ffi = intervention.to_ffi()?;
        let gain = unsafe { artemis_kernel_compute_marginal(self.handle, &corridor_ffi, &intervention_ffi) };
        if !gain.is_finite() { return Err(KernelStatus::InvalidScore); }
        Ok(gain)
    }

    pub fn rank_interventions(
        &self,
        corridors: &[CorridorState],
        interventions: &[InterventionDef],
        max_results_per_corridor: usize,
    ) -> Result<Vec<RankedIntervention>, KernelStatus> {
        if corridors.is_empty() || interventions.is_empty() { return Ok(Vec::new()); }
        let corridor_ffis: Vec<CorridorStateFFI> = corridors.iter().map(|c| c.to_ffi()).collect::<Result<Vec<_>, _>>()?;
        let intervention_ffis: Vec<InterventionDefFFI> = interventions.iter().map(|i| i.to_ffi()).collect::<Result<Vec<_>, _>>()?;
        let mut out_count: c_uint = 0;
        let results_ptr = unsafe {
            artemis_kernel_rank_interventions(
                self.handle,
                corridor_ffis.as_ptr(),
                corridors.len() as c_uint,
                intervention_ffis.as_ptr(),
                interventions.len() as c_uint,
                max_results_per_corridor as c_uint,
                &mut out_count,
            )
        };
        if results_ptr.is_null() { return Err(KernelStatus::InvalidWeight); }
        let results = unsafe { std::slice::from_raw_parts(results_ptr, out_count as usize) };
        let ranked: Vec<RankedIntervention> = results.iter().map(|r| RankedIntervention::from_ffi(r)).collect();
        unsafe { artemis_kernel_free_results(results_ptr, out_count); }
        Ok(ranked)
    }

    pub fn get_kernel_version() -> String {
        unsafe {
            let ptr = artemis_kernel_get_version();
            if ptr.is_null() { return String::from("unknown"); }
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }

    pub fn get_kernel_hash() -> String {
        unsafe {
            let ptr = artemis_kernel_get_hash();
            if ptr.is_null() { return String::from("unknown"); }
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }
}

impl Drop for CorridorSynergyKernel {
    fn drop(&mut self) {
        unsafe { artemis_kernel_destroy(self.handle); }
    }
}

extern "C" {
    fn artemis_kernel_free_results(ptr: *mut RankedInterventionFFI, count: c_uint);
}

#[derive(Debug, Clone)]
pub struct KernelConfig {
    pub weights: [f64; 5],
    pub max_cost_per_corridor: f64,
    pub min_KER_K: f64,
    pub max_KER_R: f64,
    pub enforce_monotonicity: bool,
    pub config_version: String,
    pub valid_from_unix: u64,
    pub valid_until_unix: u64,
}

impl KernelConfig {
    pub fn new_phoenix_default() -> Self {
        Self {
            weights: [0.18, 0.22, 0.18, 0.24, 0.18],
            max_cost_per_corridor: 5000000.0,
            min_KER_K: 0.85,
            max_KER_R: 0.20,
            enforce_monotonicity: true,
            config_version: String::from("phoenix-2026-v1"),
            valid_from_unix: 1735689600,
            valid_until_unix: 1767225600,
        }
    }

    fn to_ffi(&self) -> Result<KernelConfigFFI, KernelStatus> {
        if self.weights.iter().any(|&w| w < 0.0) { return Err(KernelStatus::InvalidWeight); }
        let sum: f64 = self.weights.iter().sum();
        if (sum - 1.0).abs() > 1e-6 { return Err(KernelStatus::InvalidWeight); }
        Ok(KernelConfigFFI {
            weights: self.weights,
            max_cost_per_corridor: self.max_cost_per_corridor,
            min_KER_K: self.min_KER_K,
            max_KER_R: self.max_KER_R,
            enforce_monotonicity: self.enforce_monotonicity,
            config_version: CString::new(self.config_version.as_str()).map_err(|_| KernelStatus::InvalidWeight)?.as_ptr(),
            valid_from_unix: self.valid_from_unix,
            valid_until_unix: self.valid_until_unix,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CorridorState {
    pub corridor_id: String,
    pub lat: f64,
    pub lon: f64,
    pub eco_scores: [f64; 5],
    pub K: f64,
    pub E: f64,
    pub R: f64,
    pub timestamp_unix: u64,
    pub evidence_hex: String,
}

impl CorridorState {
    fn to_ffi(&self) -> Result<CorridorStateFFI, KernelStatus> {
        if self.eco_scores.iter().any(|&s| s < 0.0 || s > 1.0) { return Err(KernelStatus::InvalidScore); }
        if self.lat < -90.0 || self.lat > 90.0 { return Err(KernelStatus::InvalidScore); }
        if self.lon < -180.0 || self.lon > 180.0 { return Err(KernelStatus::InvalidScore); }
        Ok(CorridorStateFFI {
            corridor_id: CString::new(self.corridor_id.as_str()).map_err(|_| KernelStatus::InvalidScore)?.as_ptr(),
            lat: self.lat,
            lon: self.lon,
            eco_scores: self.eco_scores,
            K: self.K,
            E: self.E,
            R: self.R,
            timestamp_unix: self.timestamp_unix,
            evidence_hex: CString::new(self.evidence_hex.as_str()).map_err(|_| KernelStatus::InvalidScore)?.as_ptr(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct InterventionDef {
    pub intervention_id: String,
    pub intervention_type: String,
    pub response_coeffs: [f64; 5],
    pub cost_usd: f64,
    pub energy_kwh: f64,
    pub land_m2: f64,
    pub ker_delta: [f64; 3],
    pub linked_evidence_hex: String,
}

impl InterventionDef {
    fn to_ffi(&self) -> Result<InterventionDefFFI, KernelStatus> {
        if self.cost_usd <= 0.0 { return Err(KernelStatus::CostOverflow); }
        if self.energy_kwh < 0.0 { return Err(KernelStatus::CostOverflow); }
        if self.land_m2 < 0.0 { return Err(KernelStatus::CostOverflow); }
        if self.response_coeffs.iter().any(|&c| c < 0.0 || c > 1.0) { return Err(KernelStatus::InvalidScore); }
        Ok(InterventionDefFFI {
            intervention_id: CString::new(self.intervention_id.as_str()).map_err(|_| KernelStatus::InvalidScore)?.as_ptr(),
            intervention_type: CString::new(self.intervention_type.as_str()).map_err(|_| KernelStatus::InvalidScore)?.as_ptr(),
            response_coeffs: self.response_coeffs,
            cost_usd: self.cost_usd,
            energy_kwh: self.energy_kwh,
            land_m2: self.land_m2,
            ker_delta: self.ker_delta,
            linked_evidence_hex: CString::new(self.linked_evidence_hex.as_str()).map_err(|_| KernelStatus::InvalidScore)?.as_ptr(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct RankedIntervention {
    pub corridor_id: String,
    pub intervention_id: String,
    pub synergy_gain: f64,
    pub cost_usd: f64,
    pub efficiency_ratio: f64,
    pub projected_KER: [f64; 3],
    pub safety_flags: u8,
}

impl RankedIntervention {
    fn from_ffi(ffi: &RankedInterventionFFI) -> Self {
        Self {
            corridor_id: unsafe { CStr::from_ptr(ffi.corridor_id).to_string_lossy().into_owned() },
            intervention_id: unsafe { CStr::from_ptr(ffi.intervention_id).to_string_lossy().into_owned() },
            synergy_gain: ffi.synergy_gain,
            cost_usd: ffi.cost_usd,
            efficiency_ratio: ffi.efficiency_ratio,
            projected_KER: ffi.projected_KER,
            safety_flags: ffi.safety_flags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_creation() {
        let config = KernelConfig::new_phoenix_default();
        let kernel = CorridorSynergyKernel::new(config);
        assert!(kernel.is_ok());
    }

    #[test]
    fn test_invalid_weights() {
        let mut config = KernelConfig::new_phoenix_default();
        config.weights[0] = -0.1;
        let kernel = CorridorSynergyKernel::new(config);
        assert_eq!(kernel.unwrap_err(), KernelStatus::InvalidWeight);
    }

    #[test]
    fn test_kernel_version() {
        let version = CorridorSynergyKernel::get_kernel_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_kernel_hash() {
        let hash = CorridorSynergyKernel::get_kernel_hash();
        assert!(hash.starts_with("0x"));
    }
}
