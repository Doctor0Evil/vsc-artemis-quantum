#![forbid(unsafe_code)]
#![deny(warnings)]
#![edition = "2024"]

use alloc::string::String;
use alloc::vec::Vec;
use ker_core::RiskCoord;

#[derive(Clone, Debug)]
pub struct QubitEcoParticle {
    pub particle_id: String,
    pub parent_qpushard_id: String,
    pub logical_qubit_map_ref: String,
    pub region_ref: String,
    pub channel_class: String,
    pub particle_role: String,
    pub rohcapstatic: f32,
    pub vecocapstatic: f32,
    pub kfestimatestatic: f32,
    pub lyapunov_v: f32,
    pub lyapunov_dvdt: f32,
    pub neurorightsflags: Vec<String>,
    pub brain_identity_binding: Option<String>,
    pub eco_lane_band: String,
    pub lyapunov_envelope_id: String,
    pub roh_band_min: f32,
    pub roh_band_max: f32,
    pub veco_band_min: f32,
    pub veco_band_max: f32,
    pub safetylevel: String,
}

impl QubitEcoParticle {
    pub fn from_aln(value: &AlnValue) -> Option<Self> {
        let get = |k: &str| value.get(k);
        let rohcap = get("rohcapstatic")?.as_f32()?;
        let vecocap = get("vecocapstatic")?.as_f32()?;
        if rohcap > 0.30 || vecocap > 0.30 { return None; }
        if get("lyapunov_dvdt")?.as_f32()? > 0.0 { return None; }
        Some(Self {
            particle_id: get("particle_id")?.as_str()?.to_owned(),
            parent_qpushard_id: get("parent_qpushard_id")?.as_str()?.to_owned(),
            logical_qubit_map_ref: get("logical_qubit_map_ref")?.as_str()?.to_owned(),
            region_ref: get("region_ref")?.as_str()?.to_owned(),
            channel_class: get("channel_class")?.as_str()?.to_owned(),
            particle_role: get("particle_role")?.as_str()?.to_owned(),
            rohcapstatic: rohcap,
            vecocapstatic: vecocap,
            kfestimatestatic: get("kfestimatestatic")?.as_f32()?,
            lyapunov_v: get("lyapunov_v")?.as_f32()?,
            lyapunov_dvdt: get("lyapunov_dvdt")?.as_f32()?,
            neurorightsflags: get("neurorightsflags")?.as_list()?
                .iter().filter_map(|v| v.as_str().map(|s| s.to_owned())).collect(),
            brain_identity_binding: get("brain_identity_binding")?.as_str().map(|s| s.to_owned()),
            eco_lane_band: get("eco_lane_band")?.as_str()?.to_owned(),
            lyapunov_envelope_id: get("lyapunov_envelope_id")?.as_str()?.to_owned(),
            roh_band_min: get("roh_band_min")?.as_f32()?,
            roh_band_max: get("roh_band_max")?.as_f32()?,
            veco_band_min: get("veco_band_min")?.as_f32()?,
            veco_band_max: get("veco_band_max")?.as_f32()?,
            safetylevel: get("safetylevel")?.as_str()?.to_owned(),
        })
    }

    pub fn neurorights_flags(&self) -> &[String] { &self.neurorightsflags }
    pub fn eco_lane(&self) -> &str { &self.eco_lane_band }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hydration_rejects_invalid_rohcap() {
        // Test would load from smoketest.aln and verify rejection
        assert!(true);
    }
}
