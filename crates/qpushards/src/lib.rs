#![forbid(unsafe_code)]
#![deny(warnings)]
#![edition = "2024"]

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Clone, Debug, PartialEq)]
pub struct QuantumVMRegionShard {
    pub regionid: String,
    pub chipbinding: String,
    pub maxqubits: u32,
    pub maxcircuitdepth: u32,
    pub maxsessions: u32,
    pub joulespergatebase: f64,
    pub rohcaplocal: f64,
    pub vecocaplocal: f64,
    pub rfpowercaplocal: f64,
    pub lane: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuantumCircuitProfileShard {
    pub profileid: String,
    pub regionid: String,
    pub augcitizenid: String,
    pub neuralstateref: String,
    pub energythresholdnominal: f64,
    pub energythresholdcritical: f64,
    pub rohbudgetjob: f64,
    pub vecobudgetjob: f64,
    pub financeforbidden: bool,
    pub tradeforbidden: bool,
    pub weaponsforbidden: bool,
    pub lane: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuantumLogicShard {
    pub logicalid: String,
    pub step_index: u64,
    pub rnodeavail: f64,
    pub rsyndromeweight: f64,
    pub ragesincelastfullreencode: f64,
    pub rrftail: f64,
    pub renergymargin: f64,
    pub rreencodechurn: f64,
    pub vtlogic: f64,
    pub lane: String,
}

impl QuantumLogicShard {
    pub fn new_logicsafe(
        logicalid: String, step_index: u64,
        rnodeavail: f64, rsyndromeweight: f64, ragesincelastfullreencode: f64,
        rrftail: f64, renergymargin: f64, rreencodechurn: f64, vtlogic: f64,
    ) -> Self {
        Self {
            logicalid, step_index, rnodeavail, rsyndromeweight, ragesincelastfullreencode,
            rrftail, renergymargin, rreencodechurn, vtlogic,
            lane: "QVM-LOGIC-RESILIENCE".to_string(),
        }
    }
    pub fn validate_caps(&self) -> bool {
        self.rnodeavail <= 1.0 && self.rsyndromeweight <= 1.0 && self.ragesincelastfullreencode <= 1.0
            && self.rrftail <= 1.0 && self.renergymargin <= 1.0 && self.rreencodechurn <= 1.0
            && self.vtlogic >= 0.0 && self.vtlogic <= 1.0
    }
    pub fn to_csv_row(&self) -> String {
        format!("{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{}",
            self.logicalid, self.step_index, self.rnodeavail, self.rsyndromeweight,
            self.ragesincelastfullreencode, self.rrftail, self.renergymargin,
            self.rreencodechurn, self.vtlogic, self.lane)
    }
    pub fn from_csv_row(line: &str) -> Result<Self, ParseError> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != 10 { return Err(ParseError::InvalidColumnCount); }
        let lane = parts[9].to_string();
        if lane != "QVM-LOGIC-RESILIENCE" { return Err(ParseError::InvalidLane); }
        Ok(Self {
            logicalid: parts[0].to_string(),
            step_index: parts[1].parse().map_err(|_| ParseError::InvalidStepIndex)?,
            rnodeavail: parts[2].parse().map_err(|_| ParseError::InvalidFloat)?,
            rsyndromeweight: parts[3].parse().map_err(|_| ParseError::InvalidFloat)?,
            ragesincelastfullreencode: parts[4].parse().map_err(|_| ParseError::InvalidFloat)?,
            rrftail: parts[5].parse().map_err(|_| ParseError::InvalidFloat)?,
            renergymargin: parts[6].parse().map_err(|_| ParseError::InvalidFloat)?,
            rreencodechurn: parts[7].parse().map_err(|_| ParseError::InvalidFloat)?,
            vtlogic: parts[8].parse().map_err(|_| ParseError::InvalidFloat)?,
            lane,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    InvalidColumnCount,
    InvalidStepIndex,
    InvalidFloat,
    InvalidLane,
}

impl Display for QuantumLogicShard {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "QuantumLogicShard {{ logicalid: {}, vtlogic: {:.6}, lane: {} }}",
            self.logicalid, self.vtlogic, self.lane)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_roundtrip() {
        let shard = QuantumLogicShard::new_logicsafe(
            "lq_001".into(), 1, 0.3, 0.2, 0.15, 0.18, 0.5, 0.05, 0.45);
        let csv = shard.to_csv_row();
        let parsed = QuantumLogicShard::from_csv_row(&csv).unwrap();
        assert_eq!(shard, parsed);
    }
    #[test]
    fn invalid_lane_rejected() {
        let csv = "lq_001,1,0.3,0.2,0.15,0.18,0.5,0.05,0.45,INVALID-LANE";
        assert!(QuantumLogicShard::from_csv_row(csv).is_err());
    }
}
