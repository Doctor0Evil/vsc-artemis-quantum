use std::mem;
use std::ffi::CStr;
use std::os::raw::{c_char, c_double, c_int, c_longlong};

const KERNEL_ID: &[u8] = b"QPUKERNEL-2026-03-28-V1\0";
const POLICY_HASH: &[u8] = b"0x454E56504C414E45535F43414E4F4E4943414C5F4752414D4D4152\0";
const ROH_CAP: c_double = 0.30;
const VECO_CAP: c_double = 0.30;
const KER_THRESHOLD: c_double = 0.85;
const LYAPUNOV_BOUND: c_double = 0.95;
const MATRIX_DIM: usize = 4;
const COMPILE_TIMESTAMP: c_longlong = 1711670400;

#[repr(C)]
pub struct KerFields {
    pub ker_score: c_double,
    pub roh_residual: c_double,
    pub veco_residual: c_double,
    pub lyapunov_p: [[c_double; MATRIX_DIM]; MATRIX_DIM],
    pub vt_current: c_double,
    pub vt_baseline: c_double,
    pub kf_estimate: c_double,
    pub energy_metric: c_double,
    pub entanglement_depth: c_int,
    pub qubit_count: c_int,
    pub circuit_depth: c_int,
    pub gate_count: c_int,
    pub hexproof_anchor: [c_char; 65],
    pub kernel_signature: [c_char; 128],
    pub validation_timestamp: c_longlong,
}

#[repr(C)]
pub struct NodeCore {
    pub nodeid: [c_char; 65],
    pub node_type: [c_char; 64],
    pub latitude: c_double,
    pub longitude: c_double,
    pub altitude_meters: c_double,
    pub jurisdiction: [c_char; 3],
    pub sovereign_owner: [c_char; 64],
    pub did: [c_char; 64],
    pub kernel_version: [c_char; 64],
    pub policy_header_hash: [c_char; 64],
    pub created_timestamp: c_longlong,
    pub last_validated_timestamp: c_longlong,
    pub operational_state: [c_char; 32],
    pub capability_set: [c_char; 1024],
    pub shard_id: [c_char; 65],
    pub region_id: [c_char; 64],
}

#[repr(C)]
pub struct QPUShard {
    pub shard_id: [c_char; 65],
    pub producer_bostrom: [c_char; 64],
    pub plane_type: [c_char; 32],
    pub node_core: NodeCore,
    pub ker_fields: KerFields,
    pub ceim_value: c_double,
    pub cpvm_value: c_double,
    pub input_hash: [c_char; 65],
    pub output_hash: [c_char; 65],
    pub replay_verified: bool,
    pub ledger_anchored: bool,
    pub anchor_tx_hash: [c_char; 65],
    pub artifact_attestation: [c_char; 256],
}

#[repr(C)]
pub struct PolicyHeader {
    pub kernel_id: [c_char; 64],
    pub policy_hash: [c_char; 64],
    pub max_roh: c_double,
    pub max_veco: c_double,
    pub min_ker: c_double,
    pub lyapunov_bound: c_double,
    pub compile_timestamp: c_longlong,
}

#[inline]
fn clamp(val: c_double, min_val: c_double, max_val: c_double) -> c_double {
    if val < min_val { min_val } else if val > max_val { max_val } else { val }
}

#[inline]
fn is_positive_definite(mat: &[[c_double; MATRIX_DIM]; MATRIX_DIM]) -> bool {
    for i in 0..MATRIX_DIM {
        if mat[i][i] <= 0.0 { return false; }
        let sum: c_double = (0..MATRIX_DIM).filter(|&j| j != i).map(|j| mat[i][j].abs()).sum();
        if mat[i][i] <= sum { return false; }
    }
    true
}

#[no_mangle]
pub extern "C" fn get_policy_header(out: *mut PolicyHeader) {
    if out.is_null() { return; }
    unsafe {
        std::ptr::write_bytes(out as *mut u8, 0, mem::size_of::<PolicyHeader>());
        let mut id_idx = 0;
        while id_idx < KERNEL_ID.len() - 1 && id_idx < 63 {
            (*out).kernel_id[id_idx] = KERNEL_ID[id_idx] as c_char;
            id_idx += 1;
        }
        let mut hash_idx = 0;
        while hash_idx < POLICY_HASH.len() - 1 && hash_idx < 63 {
            (*out).policy_hash[hash_idx] = POLICY_HASH[hash_idx] as c_char;
            hash_idx += 1;
        }
        (*out).max_roh = ROH_CAP;
        (*out).max_veco = VECO_CAP;
        (*out).min_ker = KER_THRESHOLD;
        (*out).lyapunov_bound = LYAPUNOV_BOUND;
        (*out).compile_timestamp = COMPILE_TIMESTAMP;
    }
}

#[no_mangle]
pub extern "C" fn compute_ker_quantum(
    ceim: c_double, cpvm: c_double, roh: c_double, veco: c_double,
    lyap_p: *const [c_double; MATRIX_DIM],
    qubit_count: c_int, circuit_depth: c_int
) -> c_double {
    if roh > ROH_CAP || veco > VECO_CAP { return 0.0; }
    if qubit_count > 10000 || circuit_depth > 100000 { return 0.0; }
    let lyap_matrix = unsafe { &*lyap_p };
    if !is_positive_definite(lyap_matrix) { return 0.0; }
    let base = (ceim * 0.4) + (cpvm * 0.4);
    let risk_penalty = (roh + veco) * 0.1;
    let mut lyap_factor = 1.0;
    let trace: c_double = (0..MATRIX_DIM).map(|i| lyap_matrix[i][i]).sum();
    if trace > LYAPUNOV_BOUND { lyap_factor = 0.5; }
    let ker = (base - risk_penalty) * lyap_factor;
    clamp(ker, 0.0, 1.0)
}

#[no_mangle]
pub extern "C" fn validate_quantum_shard(
    shard: *mut QPUShard, corridor_metrics: *const c_double, metric_count: usize
) -> bool {
    if shard.is_null() || corridor_metrics.is_null() { return false; }
    let s = unsafe { &mut *shard };
    let metrics = unsafe { std::slice::from_raw_parts(corridor_metrics, metric_count) };
    let kernel_version = unsafe { CStr::from_ptr(s.node_core.kernel_version.as_ptr()) }.to_bytes();
    if kernel_version != &KERNEL_ID[..KERNEL_ID.len()-1] { return false; }
    let policy_hash = unsafe { CStr::from_ptr(s.node_core.policy_header_hash.as_ptr()) }.to_bytes();
    if policy_hash != &POLICY_HASH[..POLICY_HASH.len()-1] { return false; }
    let mut sum = 0.0;
    let mut weight_sum = 0.0;
    for (i, &m) in metrics.iter().enumerate() {
        let w = 1.0 / (1.0 + i as c_double);
        sum += m * w;
        weight_sum += w;
    }
    let ceim = if weight_sum > 0.0 { clamp(sum / weight_sum, 0.0, 1.0) } else { 0.0 };
    let variance = if metric_count > 0 {
        let mean = sum / metric_count as c_double;
        metrics.iter().map(|&x| (x - mean).powi(2)).sum::<c_double>() / metric_count as c_double
    } else { 0.0 };
    let stability = 1.0 / (1.0 + variance.sqrt());
    let energy_factor = 1.0 / (1.0 + s.ker_fields.energy_metric.abs());
    let cpvm = clamp(stability * energy_factor, 0.0, 1.0);
    let ker = compute_ker_quantum(
        ceim, cpvm, s.ker_fields.roh_residual, s.ker_fields.veco_residual,
        &s.ker_fields.lyapunov_p, s.ker_fields.qubit_count, s.ker_fields.circuit_depth
    );
    if ker < KER_THRESHOLD { return false; }
    if s.ker_fields.vt_current > s.ker_fields.vt_baseline { return false; }
    s.ceim_value = ceim;
    s.cpvm_value = cpvm;
    s.ker_fields.ker_score = ker;
    s.replay_verified = true;
    true
}

#[no_mangle]
pub extern "C" fn get_kernel_timestamp() -> c_longlong { COMPILE_TIMESTAMP }

#[no_mangle]
pub extern "C" fn get_kernel_id_str() -> *const c_char { KERNEL_ID.as_ptr() as *const c_char }
