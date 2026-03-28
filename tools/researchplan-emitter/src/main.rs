#![forbid(unsafe_code)]

use std::{env, fs, path::Path};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct ResearchPlanInput {
    plane: String,
    entity: String,
    hypothesis: String,
    target_metrics: Vec<String>,
    current_ker: KerTriad,
    target_ker: KerTriad,
    evidence_refs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct KerTriad { k: f64, e: f64, r: f64 }

#[derive(Serialize, Debug)]
struct ResearchPlanShard {
    shard_id: String,
    title: String,
    epoch_grammar: String,
    author_did: String,
    created_at_utc: String,
    hexstamp: String,
    hypothesis: HypothesisBlock,
    method: MethodBlock,
    ker_safety: KerSafetyBlock,
    evidence: EvidenceBlock,
}

#[derive(Serialize, Debug)]
struct HypothesisBlock {
    plane: String, entity: String, hypothesis_text: String,
    target_metric_ids: Vec<String>, target_corridor_ids: Vec<String>,
    expected_delta_r: f64, expected_delta_vt: f64, expected_delta_e: f64,
}

#[derive(Serialize, Debug)]
struct MethodBlock {
    experimental_method: String, site_context: Vec<String>,
    sampling_plan: String, control_treatment: String,
}

#[derive(Serialize, Debug)]
struct KerSafetyBlock {
    k_score: f64, e_score: f64, r_score: f64,
    lane_requested: String, corridor_impact: String,
}

#[derive(Serialize, Debug)]
struct EvidenceBlock {
    input_datasets: Vec<String>, telemetry_snapshots: Vec<String>,
    governance_requirements: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 { eprintln!("Usage: researchplan-emitter <input.json> <output.shard>"); std::process::exit(1); }
    
    let input_raw = fs::read_to_string(&args[1])?;
    let input: ResearchPlanInput = serde_json::from_str(&input_raw)?;
    
    let shard = ResearchPlanShard {
        shard_id: format!("rps-{}", sha256_digest(&input.hypothesis)[..16].to_string()),
        title: format!("{} {} corridor refinement", input.plane, input.entity),
        epoch_grammar: "vsc-artemis-2026.1".to_string(),
        author_did: "did:bostrom:mt6883-cybernetic".to_string(),
        created_at_utc: chrono::Utc::now().to_rfc3339(),
        hexstamp: sha256_digest(&input_raw),
        hypothesis: HypothesisBlock {
            plane: input.plane.clone(), entity: input.entity, hypothesis_text: input.hypothesis,
            target_metric_ids: input.target_metrics.clone(), target_corridor_ids: vec![],
            expected_delta_r: input.current_ker.r - input.target_ker.r,
            expected_delta_vt: 0.0, expected_delta_e: input.target_ker.e - input.current_ker.e,
        },
        method: MethodBlock {
            experimental_method: "Phoenix pilot calibration".to_string(),
            site_context: vec!["phx.qvm-core".to_string()],
            sampling_plan: "Q1-Q3 2026 telemetry".to_string(),
            control_treatment: "baseline vs tightened corridors".to_string(),
        },
        ker_safety: KerSafetyBlock {
            k_score: input.target_ker.k, e_score: input.target_ker.e, r_score: input.target_ker.r,
            lane_requested: "RESEARCH".to_string(),
            corridor_impact: "roh_cap_static, rf_exposure goldmax".to_string(),
        },
        evidence: EvidenceBlock {
            input_datasets: input.evidence_refs.clone(),
            telemetry_snapshots: vec![],
            governance_requirements: vec!["corridorpresent".to_string(), "residualsafe".to_string(), "neurorights_ok".to_string()],
        },
    };
    
    let output = serde_json::to_string_pretty(&shard)?;
    fs::write(&args[2], output)?;
    println!("ResearchPlanShard written to {}", &args[2]);
    Ok(())
}

fn sha256_digest(input: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}
