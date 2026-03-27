#ifndef ARTEMIS_CORRIDOR_SYNERGY_KERNEL_HPP
#define ARTEMIS_CORRIDOR_SYNERGY_KERNEL_HPP

#include <array>
#include <vector>
#include <string>
#include <cstdint>
#include <optional>
#include <numeric>
#include <algorithm>
#include <cmath>

namespace artemis {
namespace corridor {

enum class EcoDimension : uint8_t { Grid = 0, Mobility = 1, Buildings = 2, Green = 3, Litter = 4, Count = 5 };
enum class KernelStatus : uint8_t { Ok = 0, InvalidWeight = 1, InvalidScore = 2, CostOverflow = 3, KERViolation = 4 };

struct CorridorState {
    std::string corridor_id;
    double lat;
    double lon;
    std::array<double, 5> eco_scores;
    double K;
    double E;
    double R;
    uint64_t timestamp_unix;
    std::string evidence_hex;
};

struct InterventionDef {
    std::string intervention_id;
    std::string intervention_type;
    std::array<double, 5> response_coeffs;
    double cost_usd;
    double energy_kwh;
    double land_m2;
    std::array<double, 3> ker_delta;
    std::string linked_evidence_hex;
};

struct RankedIntervention {
    std::string corridor_id;
    std::string intervention_id;
    double synergy_gain;
    double cost_usd;
    double efficiency_ratio;
    std::array<double, 3> projected_KER;
    uint8_t safety_flags;
};

struct KernelConfig {
    std::array<double, 5> weights;
    double max_cost_per_corridor;
    double min_KER_K;
    double max_KER_R;
    bool enforce_monotonicity;
    std::string config_version;
    uint64_t valid_from_unix;
    uint64_t valid_until_unix;
};

class CorridorSynergyKernel {
public:
    explicit CorridorSynergyKernel(const KernelConfig& config);
    KernelStatus validateConfig() const;
    KernelStatus validateCorridor(const CorridorState& corridor) const;
    KernelStatus validateIntervention(const InterventionDef& intervention) const;
    double computeSynergyScore(const CorridorState& corridor) const;
    double computeMarginalGain(const CorridorState& corridor, const InterventionDef& intervention) const;
    std::vector<RankedIntervention> rankInterventions(
        const std::vector<CorridorState>& corridors,
        const std::vector<InterventionDef>& interventions,
        size_t max_results_per_corridor = 10
    );
    std::string getKernelVersion() const { return "1.0.0"; }
    std::string getKernelHash() const { return "0x21d4a9c73f58e2b19a40c6d8e3f7b5a1c9e2047f3a8d6b2c51e07f49a3c6d8e2b7f1049c3e8a6d2f5b1c7e9a304f6d1c9"; }

private:
    KernelConfig m_config;
    bool checkWeightsValid() const;
    bool checkScoresValid(const std::array<double, 5>& scores) const;
    bool checkKERBounds(double K, double E, double R) const;
    bool checkMonotonicity(const CorridorState& before, const std::array<double, 3>& ker_delta) const;
    double clamp(double value, double min_val, double max_val) const;
};

inline double CorridorSynergyKernel::clamp(double value, double min_val, double max_val) const {
    return std::max(min_val, std::min(value, max_val));
}

inline bool CorridorSynergyKernel::checkWeightsValid() const {
    double sum = std::accumulate(m_config.weights.begin(), m_config.weights.end(), 0.0);
    if (std::abs(sum - 1.0) > 1e-6) return false;
    return std::all_of(m_config.weights.begin(), m_config.weights.end(), [](double w) { return w >= 0.0; });
}

inline bool CorridorSynergyKernel::checkScoresValid(const std::array<double, 5>& scores) const {
    return std::all_of(scores.begin(), scores.end(), [](double s) { return s >= 0.0 && s <= 1.0; });
}

inline bool CorridorSynergyKernel::checkKERBounds(double K, double E, double R) const {
    return K >= m_config.min_KER_K && R <= m_config.max_KER_R && K >= 0.0 && K <= 1.0 && E >= 0.0 && E <= 1.0 && R >= 0.0 && R <= 1.0;
}

}
}

#endif
