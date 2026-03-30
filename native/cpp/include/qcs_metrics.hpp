#ifndef QCS_METRICS_HPP
#define QCS_METRICS_HPP

#include <cstdint>
#include <cstddef>

namespace vsc {
namespace qcs {

typedef struct {
    double vt;
    double vt_prev;
    double dVdt;
    uint64_t evaluation_timestamp_ns;
    int32_t status;
} LyapunovState_C;

typedef struct {
    double roh_contrib;
    double veco_contrib;
    double energy_estimate;
    double gate_error_estimate;
    int32_t status;
} KERScore_C;

typedef struct {
    double circuit_depth;
    double qubit_count;
    double gate_count_1q;
    double gate_count_2q;
    double measurement_count;
    double coherence_time_us;
    double gate_fidelity;
} CircuitDescriptor_C;

typedef struct {
    double lyapunov_residual;
    double roh_budget_used;
    double veco_budget_used;
    double energy_joules;
    int32_t admissibility_score;
} QcsSafetyMetrics_C;

#ifdef __cplusplus
extern "C" {
#endif

int qcs_compute_lyapunov(
    const LyapunovState_C* state_in,
    LyapunovState_C* state_out,
    KERScore_C* score_out
);

int qcs_estimate_energy(
    const CircuitDescriptor_C* circuit_desc,
    double* out_energy_estimate,
    double* out_gate_error
);

int qcs_compute_safety_metrics(
    const LyapunovState_C* state,
    const KERScore_C* score,
    double roh_budget,
    double veco_budget,
    QcsSafetyMetrics_C* metrics_out
);

int qcs_validate_state_structs(
    const LyapunovState_C* state,
    const KERScore_C* score,
    int* validation_errors
);

#ifdef __cplusplus
}
#endif

constexpr int QCS_SUCCESS = 0;
constexpr int QCS_ERROR_NULL_PTR = -1;
constexpr int QCS_ERROR_INVALID_STATE = -2;
constexpr int QCS_ERROR_NUMERIC_OVERFLOW = -3;
constexpr int QCS_ERROR_COHERENCE_EXCEEDED = -4;
constexpr int QCS_ERROR_FIDELITY_THRESHOLD = -5;

constexpr double QCS_LYAPUNOV_THRESHOLD_MIN = 0.85;
constexpr double QCS_LYAPUNOV_DELTA_MAX = 0.01;
constexpr double QCS_ROH_CAP_GLOBAL = 0.30;
constexpr double QCS_VECO_CAP_GLOBAL = 0.30;
constexpr double QCS_GATE_FIDELITY_MIN = 0.99;

}
}

#endif
