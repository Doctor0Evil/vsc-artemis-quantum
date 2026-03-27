#![forbid(unsafe_code)]
#![deny(warnings)]
#![edition = "2024"]

use crate::qubit_eco_particle::QubitEcoParticle;

pub struct RegionLink {
    pub eco_lane_id: String,
    pub allowed_particle_classes: Vec<String>,
    pub roh_band_max_link: f32,
    pub veco_band_max_link: f32,
    pub lyapunov_envelope_required: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteDecision { Allowed, Denied(&'static str) }

pub fn check_eco_lane_for_route(particle: &QubitEcoParticle, link: &RegionLink) -> RouteDecision {
    if !link.allowed_particle_classes.contains(&particle.channel_class) {
        return RouteDecision::Denied("particle_class_not_allowed");
    }
    if particle.roh_band_max > link.roh_band_max_link {
        return RouteDecision::Denied("roh_band_exceeds_link");
    }
    if particle.veco_band_max > link.veco_band_max_link {
        return RouteDecision::Denied("veco_band_exceeds_link");
    }
    if particle.lyapunov_dvdt > 0.0 {
        return RouteDecision::Denied("lyapunov_not_nonincreasing");
    }
    if particle.eco_lane_band != link.eco_lane_id {
        return RouteDecision::Denied("eco_lane_mismatch");
    }
    RouteDecision::Allowed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_particle_allowed() {
        let particle = QubitEcoParticle {
            particle_id: "qep_001".into(), parent_qpushard_id: "shard_001".into(),
            logical_qubit_map_ref: "lqmap_001".into(), region_ref: "phx".into(),
            channel_class: "QPUJOBDESCRIPTORV1".into(), particle_role: "data".into(),
            rohcapstatic: 0.25, vecocapstatic: 0.25, kfestimatestatic: 0.85,
            lyapunov_v: 0.5, lyapunov_dvdt: -0.01, neurorightsflags: vec!["no-repurpose".into()],
            brain_identity_binding: None, eco_lane_band: "lane.roh-mid".into(),
            lyapunov_envelope_id: "inv_001".into(), roh_band_min: 0.0, roh_band_max: 0.20,
            veco_band_min: 0.0, veco_band_max: 0.20, safetylevel: "eco-qubit-l2".into(),
        };
        let link = RegionLink {
            eco_lane_id: "lane.roh-mid".into(),
            allowed_particle_classes: vec!["QPUJOBDESCRIPTORV1".into()],
            roh_band_max_link: 0.25, veco_band_max_link: 0.25,
            lyapunov_envelope_required: "inv_001".into(),
        };
        assert_eq!(check_eco_lane_for_route(&particle, &link), RouteDecision::Allowed);
    }
}
