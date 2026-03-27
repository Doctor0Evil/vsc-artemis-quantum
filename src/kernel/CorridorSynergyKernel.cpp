#include "CorridorSynergyKernel.hpp"
#include <stdexcept>

namespace artemis {
namespace corridor {

CorridorSynergyKernel::CorridorSynergyKernel(const KernelConfig& config) : m_config(config) {}

double CorridorSynergyKernel::clampValue(double value, double min_val, double max_val) {
    return std::max(min_val, std::min(value, max_val));
}

bool CorridorSynergyKernel::checkWeightsValid() const {
    double sum = std::accumulate(m_config.weights.begin(), m_config.weights.end(), 0.0);
    if (std::abs(sum - 1.0) > 1e-6) return false;
    return std::all_of(m_config.weights.begin(), m_config.weights.end(), [](double w) { return w >= 0.0; });
}

bool CorridorSynergyKernel::checkScoresValid(const std::array<double, 5>& scores) const {
    return std::all_of(scores.begin(), scores.end(), [](double s) { return s >= 0.0 && s <= 1.0; });
}

bool CorridorSynergyKernel::checkKERBounds(double K, double E, double R) const {
    return K >= m_config.min_KER_K && R <= m_config.max_KER_R && 
           K >= 0.0 && K <= 1.0 && E >= 0.0 && E <= 1.0 && R >= 0.0 && R <= 1.0;
}

bool CorridorSynergyKernel::checkMonotonicity(const CorridorState& before, const std::array<double, 3>& ker_delta) const {
    if (!m_config.enforce_monotonicity) return true;
    double after_K = before.K + ker_delta[0];
    double after_E = before.E + ker_delta[1];
    double after_R = before.R + ker_delta[2];
    if (after_K < before.K) return false;
    if (after_E < before.E) return false;
    if (after_R > before.R) return false;
    return checkKERBounds(after_K, after_E, after_R);
}

KernelStatus CorridorSynergyKernel::validateConfig() const {
    if (!checkWeightsValid()) return KernelStatus::InvalidWeight;
    if (m_config.max_cost_per_corridor <= 0.0) return KernelStatus::CostOverflow;
    if (m_config.min_KER_K < 0.0 || m_config.min_KER_K > 1.0) return KernelStatus::KERViolation;
    if (m_config.max_KER_R < 0.0 || m_config.max_KER_R > 1.0) return KernelStatus::KERViolation;
    if (m_config.valid_from_unix >= m_config.valid_until_unix) return KernelStatus::InvalidWeight;
    return KernelStatus::Ok;
}

KernelStatus CorridorSynergyKernel::validateCorridor(const CorridorState& corridor) const {
    if (!checkScoresValid(corridor.eco_scores)) return KernelStatus::InvalidScore;
    if (!checkKERBounds(corridor.K, corridor.E, corridor.R)) return KernelStatus::KERViolation;
    if (corridor.lat < -90.0 || corridor.lat > 90.0) return KernelStatus::InvalidScore;
    if (corridor.lon < -180.0 || corridor.lon > 180.0) return KernelStatus::InvalidScore;
    return KernelStatus::Ok;
}

KernelStatus CorridorSynergyKernel::validateIntervention(const InterventionDef& intervention) const {
    if (intervention.cost_usd <= 0.0) return KernelStatus::CostOverflow;
    if (intervention.energy_kwh < 0.0) return KernelStatus::CostOverflow;
    if (intervention.land_m2 < 0.0) return KernelStatus::CostOverflow;
    if (!checkScoresValid(intervention.response_coeffs)) return KernelStatus::InvalidScore;
    return KernelStatus::Ok;
}

double CorridorSynergyKernel::computeSynergyScore(const CorridorState& corridor) const {
    double score = 0.0;
    for (size_t i = 0; i < 5; ++i) {
        score += m_config.weights[i] * corridor.eco_scores[i];
    }
    return clampValue(score, 0.0, 1.0);
}

double CorridorSynergyKernel::computeMarginalGain(const CorridorState& corridor, const InterventionDef& intervention) const {
    double gain = 0.0;
    for (size_t i = 0; i < 5; ++i) {
        double projected = clampValue(corridor.eco_scores[i] + intervention.response_coeffs[i], 0.0, 1.0);
        gain += m_config.weights[i] * (projected - corridor.eco_scores[i]);
    }
    return gain;
}

std::vector<RankedIntervention> CorridorSynergyKernel::rankInterventions(
    const std::vector<CorridorState>& corridors,
    const std::vector<InterventionDef>& interventions,
    size_t max_results_per_corridor
) {
    std::vector<RankedIntervention> results;
    if (validateConfig() != KernelStatus::Ok) return results;
    
    for (const auto& corridor : corridors) {
        if (validateCorridor(corridor) != KernelStatus::Ok) continue;
        
        std::vector<RankedIntervention> corridor_rankings;
        corridor_rankings.reserve(interventions.size());
        
        for (const auto& intervention : interventions) {
            if (validateIntervention(intervention) != KernelStatus::Ok) continue;
            if (intervention.cost_usd > m_config.max_cost_per_corridor) continue;
            
            double marginal_gain = computeMarginalGain(corridor, intervention);
            if (marginal_gain <= 0.0) continue;
            
            double efficiency = marginal_gain / intervention.cost_usd;
            
            std::array<double, 3> projected_KER = {
                clampValue(corridor.K + intervention.ker_delta[0], 0.0, 1.0),
                clampValue(corridor.E + intervention.ker_delta[1], 0.0, 1.0),
                clampValue(corridor.R + intervention.ker_delta[2], 0.0, 1.0)
            };
            
            uint8_t safety_flags = 0;
            if (!checkKERBounds(projected_KER[0], projected_KER[1], projected_KER[2])) {
                safety_flags |= 0x01;
            }
            if (!checkMonotonicity(corridor, intervention.ker_delta)) {
                safety_flags |= 0x02;
            }
            if (safety_flags != 0) continue;
            
            RankedIntervention ranked;
            ranked.corridor_id = corridor.corridor_id;
            ranked.intervention_id = intervention.intervention_id;
            ranked.synergy_gain = marginal_gain;
            ranked.cost_usd = intervention.cost_usd;
            ranked.efficiency_ratio = efficiency;
            ranked.projected_KER = projected_KER;
            ranked.safety_flags = safety_flags;
            corridor_rankings.push_back(ranked);
        }
        
        std::sort(corridor_rankings.begin(), corridor_rankings.end(),
            [](const RankedIntervention& a, const RankedIntervention& b) {
                return a.efficiency_ratio > b.efficiency_ratio;
            });
        
        size_t count = std::min(corridor_rankings.size(), max_results_per_corridor);
        results.insert(results.end(), corridor_rankings.begin(), corridor_rankings.begin() + count);
    }
    
    return results;
}

}
}
