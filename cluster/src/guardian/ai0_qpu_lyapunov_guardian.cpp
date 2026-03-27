// Destination: vsc-artemis-quantum/cluster/src/guardian/ai0_qpu_lyapunov_guardian.cpp

#include <cmath>
#include <cstdint>
#include <string>
#include <vector>
#include <unordered_map>
#include <stdexcept>
#include <chrono>

namespace artemis {
namespace quantum {
namespace ai0 {

struct QpuFragmentContract {
    std::string fragment_id;          // e.g. "AI0-CONV-DEPTH32"
    double roh_cap;                   // <= 0.30
    double veco_cap;                  // <= 0.30
    double vt_decay_min;              // alpha_V in [0.01, 0.05]
    double delta_k_min;               // min K gain
    double delta_r_max;               // max allowed R increase
    uint32_t depth_max;
    uint32_t qubits_min;
    uint32_t qubits_max;
};

struct Ai0State {
    double k_before;
    double e_before;
    double r_before;
    double vt_before;
    double roh_before;
    double veco_before;
    uint32_t depth_before;
    uint32_t qubits_before;
};

struct Ai0StateAfter {
    double k_after;
    double e_after;
    double r_after;
    double vt_after;
    double roh_after;
    double veco_after;
};

struct GuardianConfig {
    double vt_epsilon;    // ~1e-9, non-expansion slack
    double roh_global_cap;
    double veco_global_cap;
    double vt_global_hard; // e.g. 0.85 from quantum invariants
};

struct GuardianDecision {
    bool allowed;
    std::string reason;
    double vt_before;
    double vt_after;
    double roh_after;
    double veco_after;
    double k_delta;
    double r_delta;
};

class Ai0QpuLyapunovGuardian {
public:
    Ai0QpuLyapunovGuardian(GuardianConfig cfg)
        : cfg_(cfg) {}

    void register_fragment_contract(const QpuFragmentContract& c) {
        contracts_[c.fragment_id] = c;
    }

    bool has_fragment(const std::string& id) const {
        return contracts_.find(id) != contracts_.end();
    }

    GuardianDecision evaluate_step(const std::string& fragment_id,
                                   const Ai0State& before,
                                   const Ai0StateAfter& after) const
    {
        auto it = contracts_.find(fragment_id);
        if (it == contracts_.end()) {
            return reject("unknown_fragment", before, after);
        }
        const QpuFragmentContract& c = it->second;

        if (!check_global_caps(after)) {
            return reject("roh_or_veco_global_cap_breach", before, after);
        }
        if (!check_fragment_caps(c, after)) {
            return reject("roh_or_veco_fragment_cap_breach", before, after);
        }
        if (!check_vt_monotone(before, after)) {
            return reject("vt_non_monotone", before, after);
        }
        if (!check_vt_decay(c, before, after)) {
            return reject("vt_decay_too_weak", before, after);
        }
        if (!check_ker_delta(c, before, after)) {
            return reject("ker_delta_violation", before, after);
        }

        GuardianDecision ok;
        ok.allowed = true;
        ok.reason = "ok";
        ok.vt_before = before.vt_before;
        ok.vt_after = after.vt_after;
        ok.roh_after = after.roh_after;
        ok.veco_after = after.veco_after;
        ok.k_delta = after.k_after - before.k_before;
        ok.r_delta = after.r_after - before.r_before;
        return ok;
    }

private:
    GuardianConfig cfg_;
    std::unordered_map<std::string,QpuFragmentContract> contracts_;

    bool check_global_caps(const Ai0StateAfter& after) const {
        if (after.roh_after > cfg_.roh_global_cap) return false;
        if (after.veco_after > cfg_.veco_global_cap) return false;
        if (after.vt_after > cfg_.vt_global_hard) return false;
        return true;
    }

    bool check_fragment_caps(const QpuFragmentContract& c,
                             const Ai0StateAfter& after) const
    {
        if (after.roh_after > c.roh_cap) return false;
        if (after.veco_after > c.veco_cap) return false;
        return true;
    }

    bool check_vt_monotone(const Ai0State& before,
                           const Ai0StateAfter& after) const
    {
        double vt_before = before.vt_before;
        double vt_after = after.vt_after;
        return vt_after <= vt_before + cfg_.vt_epsilon;
    }

    bool check_vt_decay(const QpuFragmentContract& c,
                        const Ai0State& before,
                        const Ai0StateAfter& after) const
    {
        double vt_before = before.vt_before;
        double vt_after = after.vt_after;
        if (vt_before <= 0.0) return true;
        double ratio = vt_after / vt_before;
        double max_ratio = 1.0 - c.vt_decay_min;
        return ratio <= max_ratio + cfg_.vt_epsilon;
    }

    bool check_ker_delta(const QpuFragmentContract& c,
                         const Ai0State& before,
                         const Ai0StateAfter& after) const
    {
        double dk = after.k_after - before.k_before;
        double dr = after.r_after - before.r_before;
        if (dk < c.delta_k_min - 1e-9) return false;
        if (dr > c.delta_r_max + 1e-9) return false;
        return true;
    }

    GuardianDecision reject(const std::string& reason,
                            const Ai0State& before,
                            const Ai0StateAfter& after) const
    {
        GuardianDecision d;
        d.allowed = false;
        d.reason = reason;
        d.vt_before = before.vt_before;
        d.vt_after = after.vt_after;
        d.roh_after = after.roh_after;
        d.veco_after = after.veco_after;
        d.k_delta = after.k_after - before.k_before;
        d.r_delta = after.r_after - before.r_before;
        return d;
    }
}

; // class Ai0QpuLyapunovGuardian

} // namespace ai0
} // namespace quantum
} // namespace artemis

extern "C" {

struct Ai0GuardianConfigC {
    double vt_epsilon;
    double roh_global_cap;
    double veco_global_cap;
    double vt_global_hard;
};

struct Ai0FragmentContractC {
    const char* fragment_id;
    double roh_cap;
    double veco_cap;
    double vt_decay_min;
    double delta_k_min;
    double delta_r_max;
    uint32_t depth_max;
    uint32_t qubits_min;
    uint32_t qubits_max;
};

struct Ai0StateC {
    double k_before;
    double e_before;
    double r_before;
    double vt_before;
    double roh_before;
    double veco_before;
    uint32_t depth_before;
    uint32_t qubits_before;
};

struct Ai0StateAfterC {
    double k_after;
    double e_after;
    double r_after;
    double vt_after;
    double roh_after;
    double veco_after;
};

struct Ai0GuardianDecisionC {
    int allowed;
    const char* reason;
    double vt_before;
    double vt_after;
    double roh_after;
    double veco_after;
    double k_delta;
    double r_delta;
};

static artemis::quantum::ai0::Ai0QpuLyapunovGuardian* g_guardian = nullptr;
static std::string g_last_reason;

int artemis_ai0_qpu_guardian_init(const Ai0GuardianConfigC* cfg) {
    if (!cfg) return -1;
    artemis::quantum::ai0::GuardianConfig gc;
    gc.vt_epsilon = cfg->vt_epsilon;
    gc.roh_global_cap = cfg->roh_global_cap;
    gc.veco_global_cap = cfg->veco_global_cap;
    gc.vt_global_hard = cfg->vt_global_hard;
    delete g_guardian;
    g_guardian = new artemis::quantum::ai0::Ai0QpuLyapunovGuardian(gc);
    return 0;
}

int artemis_ai0_qpu_guardian_register_fragment(const Ai0FragmentContractC* fc) {
    if (!g_guardian || !fc || !fc->fragment_id) return -1;
    artemis::quantum::ai0::QpuFragmentContract c;
    c.fragment_id = fc->fragment_id;
    c.roh_cap = fc->roh_cap;
    c.veco_cap = fc->veco_cap;
    c.vt_decay_min = fc->vt_decay_min;
    c.delta_k_min = fc->delta_k_min;
    c.delta_r_max = fc->delta_r_max;
    c.depth_max = fc->depth_max;
    c.qubits_min = fc->qubits_min;
    c.qubits_max = fc->qubits_max;
    g_guardian->register_fragment_contract(c);
    return 0;
}

int artemis_ai0_qpu_guardian_evaluate(const char* fragment_id,
                                      const Ai0StateC* before,
                                      const Ai0StateAfterC* after,
                                      Ai0GuardianDecisionC* out_decision)
{
    if (!g_guardian || !fragment_id || !before || !after || !out_decision) return -1;

    artemis::quantum::ai0::Ai0State b;
    b.k_before = before->k_before;
    b.e_before = before->e_before;
    b.r_before = before->r_before;
    b.vt_before = before->vt_before;
    b.roh_before = before->roh_before;
    b.veco_before = before->veco_before;
    b.depth_before = before->depth_before;
    b.qubits_before = before->qubits_before;

    artemis::quantum::ai0::Ai0StateAfter a;
    a.k_after = after->k_after;
    a.e_after = after->e_after;
    a.r_after = after->r_after;
    a.vt_after = after->vt_after;
    a.roh_after = after->roh_after;
    a.veco_after = after->veco_after;

    auto d = g_guardian->evaluate_step(std::string(fragment_id), b, a);
    g_last_reason = d.reason;

    out_decision->allowed = d.allowed ? 1 : 0;
    out_decision->reason = g_last_reason.c_str();
    out_decision->vt_before = d.vt_before;
    out_decision->vt_after = d.vt_after;
    out_decision->roh_after = d.roh_after;
    out_decision->veco_after = d.veco_after;
    out_decision->k_delta = d.k_delta;
    out_decision->r_delta = d.r_delta;
    return 0;
}

} // extern "C"
