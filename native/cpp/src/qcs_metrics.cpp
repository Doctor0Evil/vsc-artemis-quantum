#include "qcs_metrics.hpp"
#include <cmath>
#include <limits>
#include <algorithm>

namespace vsc {
namespace qcs {

static constexpr double EPSILON = 1e-12;
static constexpr double MAX_DOUBLE = std::numeric_limits<double>::max();
static constexpr double JOULES_PER_GATE_1Q = 1.5e-15;
static constexpr double JOULES_PER_GATE_2Q = 4.2e-14;
static constexpr double JOULES_PER_MEASUREMENT = 8.7e-13;
static constexpr double COHERENCE_DECAY_FACTOR = 0.001;
static constexpr double FIDELITY_PENALTY_EXPONENT = 2.0;

static bool validate_pointer(const void* ptr) {
    return ptr != nullptr;
}

static bool validate_finite(double value) {
    return std::isfinite(value) && !std::isnan(value);
}

static bool validate_lyapunov_state(const LyapunovState_C* state) {
    if (!validate_pointer(state)) return false;
    if (!validate_finite(state->vt)) return false;
    if (!validate_finite(state->vt_prev)) return false;
    if (!validate_finite(state->dVdt)) return false;
    if (state->vt < 0.0 || state->vt > 1.0) return false;
    if (state->vt_prev < 0.0 || state->vt_prev > 1.0) return false;
    return true;
}

static bool validate_ker_score(const KERScore_C* score) {
    if (!validate_pointer(score)) return false;
    if (!validate_finite(score->roh_contrib)) return false;
    if (!validate_finite(score->veco_contrib)) return false;
    if (!validate_finite(score->energy_estimate)) return false;
    if (score->roh_contrib < 0.0 || score->roh_contrib > QCS_ROH_CAP_GLOBAL) return false;
    if (score->veco_contrib < 0.0 || score->veco_contrib > QCS_VECO_CAP_GLOBAL) return false;
    return true;
}

static double compute_lyapunov_residual(double vt, double vt_prev) {
    if (vt_prev <= EPSILON) return vt;
    double residual = (vt - vt_prev) / vt_prev;
    return std::max(-1.0, std::min(1.0, residual));
}

static double compute_dvdt(double vt, double vt_prev, uint64_t delta_ns) {
    if (delta_ns == 0) return 0.0;
    double delta_t_seconds = static_cast<double>(delta_ns) / 1e9;
    double delta_v = vt - vt_prev;
    return delta_v / delta_t_seconds;
}

static double compute_roh_contribution(double gate_error, double coherence_ratio) {
    double gate_penalty = std::pow(1.0 - gate_error, FIDELITY_PENALTY_EXPONENT);
    double coherence_penalty = std::exp(-COHERENCE_DECAY_FACTOR * (1.0 - coherence_ratio));
    double base_roh = (1.0 - gate_penalty) * 0.15;
    double coherence_roh = (1.0 - coherence_penalty) * 0.10;
    return std::min(QCS_ROH_CAP_GLOBAL, base_roh + coherence_roh);
}

static double compute_veco_contribution(double energy_joules, double circuit_depth) {
    double energy_factor = std::min(1.0, energy_joules / 1e-9);
    double depth_factor = std::min(1.0, circuit_depth / 1000.0);
    return std::min(QCS_VECO_CAP_GLOBAL, energy_factor * 0.20 + depth_factor * 0.08);
}

}
}

extern "C" {

int vsc::qcs::qcs_compute_lyapunov(
    const LyapunovState_C* state_in,
    LyapunovState_C* state_out,
    KERScore_C* score_out
) {
    if (!validate_pointer(state_in) || !validate_pointer(state_out) || !validate_pointer(score_out)) {
        return QCS_ERROR_NULL_PTR;
    }
    if (!validate_lyapunov_state(state_in)) {
        return QCS_ERROR_INVALID_STATE;
    }
    double vt_raw = state_in->vt;
    double vt_prev = state_in->vt_prev;
    if (!validate_finite(vt_raw) || !validate_finite(vt_prev)) {
        return QCS_ERROR_NUMERIC_OVERFLOW;
    }
    double vt_clamped = std::max(0.0, std::min(1.0, vt_raw));
    double residual = compute_lyapunov_residual(vt_clamped, vt_prev);
    state_out->vt = vt_clamped;
    state_out->vt_prev = vt_prev;
    state_out->dVdt = compute_dvdt(vt_clamped, vt_prev, state_in->evaluation_timestamp_ns);
    state_out->evaluation_timestamp_ns = state_in->evaluation_timestamp_ns;
    state_out->status = (residual <= QCS_LYAPUNOV_DELTA_MAX) ? QCS_SUCCESS : QCS_ERROR_INVALID_STATE;
    double coherence_ratio = std::max(0.0, std::min(1.0, 1.0 - std::abs(residual)));
    double gate_error = std::max(0.0, 1.0 - QCS_GATE_FIDELITY_MIN);
    score_out->roh_contrib = compute_roh_contribution(gate_error, coherence_ratio);
    score_out->veco_contrib = 0.0;
    score_out->energy_estimate = 0.0;
    score_out->gate_error_estimate = gate_error;
    score_out->status = (score_out->roh_contrib <= QCS_ROH_CAP_GLOBAL) ? QCS_SUCCESS : QCS_ERROR_INVALID_STATE;
    return QCS_SUCCESS;
}

int vsc::qcs::qcs_estimate_energy(
    const CircuitDescriptor_C* circuit_desc,
    double* out_energy_estimate,
    double* out_gate_error
) {
    if (!validate_pointer(circuit_desc) || !validate_pointer(out_energy_estimate) || !validate_pointer(out_gate_error)) {
        return QCS_ERROR_NULL_PTR;
    }
    if (!validate_finite(circuit_desc->circuit_depth) ||
        !validate_finite(circuit_desc->qubit_count) ||
        !validate_finite(circuit_desc->gate_count_1q) ||
        !validate_finite(circuit_desc->gate_count_2q) ||
        !validate_finite(circuit_desc->measurement_count)) {
        return QCS_ERROR_NUMERIC_OVERFLOW;
    }
    if (circuit_desc->circuit_depth < 0 || circuit_desc->qubit_count < 0 ||
        circuit_desc->gate_count_1q < 0 || circuit_desc->gate_count_2q < 0 ||
        circuit_desc->measurement_count < 0) {
        return QCS_ERROR_INVALID_STATE;
    }
    double energy_1q = circuit_desc->gate_count_1q * JOULES_PER_GATE_1Q;
    double energy_2q = circuit_desc->gate_count_2q * JOULES_PER_GATE_2Q;
    double energy_meas = circuit_desc->measurement_count * JOULES_PER_MEASUREMENT;
    double energy_total = energy_1q + energy_2q + energy_meas;
    if (!validate_finite(energy_total) || energy_total < 0) {
        return QCS_ERROR_NUMERIC_OVERFLOW;
    }
    *out_energy_estimate = energy_total;
    double fidelity = std::max(0.0, std::min(1.0, circuit_desc->gate_fidelity));
    double gate_error_raw = 1.0 - fidelity;
    double coherence_factor = std::exp(-COHERENCE_DECAY_FACTOR * circuit_desc->circuit_depth);
    *out_gate_error = gate_error_raw + (1.0 - coherence_factor) * 0.05;
    if (fidelity < QCS_GATE_FIDELITY_MIN) {
        return QCS_ERROR_FIDELITY_THRESHOLD;
    }
    if (circuit_desc->coherence_time_us < circuit_desc->circuit_depth * 10.0) {
        return QCS_ERROR_COHERENCE_EXCEEDED;
    }
    return QCS_SUCCESS;
}

int vsc::qcs::qcs_compute_safety_metrics(
    const LyapunovState_C* state,
    const KERScore_C* score,
    double roh_budget,
    double veco_budget,
    QcsSafetyMetrics_C* metrics_out
) {
    if (!validate_pointer(state) || !validate_pointer(score) || !validate_pointer(metrics_out)) {
        return QCS_ERROR_NULL_PTR;
    }
    if (!validate_lyapunov_state(state) || !validate_ker_score(score)) {
        return QCS_ERROR_INVALID_STATE;
    }
    if (!validate_finite(roh_budget) || !validate_finite(veco_budget)) {
        return QCS_ERROR_NUMERIC_OVERFLOW;
    }
    double residual = compute_lyapunov_residual(state->vt, state->vt_prev);
    metrics_out->lyapunov_residual = residual;
    metrics_out->roh_budget_used = score->roh_contrib / std::max(EPSILON, roh_budget);
    metrics_out->veco_budget_used = score->veco_contrib / std::max(EPSILON, veco_budget);
    metrics_out->energy_joules = score->energy_estimate;
    double lyapunov_score = (residual <= QCS_LYAPUNOV_DELTA_MAX) ? 1.0 : 0.0;
    double roh_score = (metrics_out->roh_budget_used <= 1.0) ? 1.0 : 0.0;
    double veco_score = (metrics_out->veco_budget_used <= 1.0) ? 1.0 : 0.0;
    metrics_out->admissibility_score = static_cast<int32_t>(
        (lyapunov_score + roh_score + veco_score) / 3.0 * 100.0
    );
    return QCS_SUCCESS;
}

int vsc::qcs::qcs_validate_state_structs(
    const LyapunovState_C* state,
    const KERScore_C* score,
    int* validation_errors
) {
    if (!validate_pointer(state) || !validate_pointer(score) || !validate_pointer(validation_errors)) {
        return QCS_ERROR_NULL_PTR;
    }
    *validation_errors = 0;
    if (!validate_pointer(state)) (*validation_errors)++;
    if (!validate_finite(state->vt)) (*validation_errors)++;
    if (!validate_finite(state->vt_prev)) (*validation_errors)++;
    if (!validate_finite(state->dVdt)) (*validation_errors)++;
    if (!validate_pointer(score)) (*validation_errors)++;
    if (!validate_finite(score->roh_contrib)) (*validation_errors)++;
    if (!validate_finite(score->veco_contrib)) (*validation_errors)++;
    if (!validate_finite(score->energy_estimate)) (*validation_errors)++;
    return (*validation_errors == 0) ? QCS_SUCCESS : QCS_ERROR_INVALID_STATE;
}

}
