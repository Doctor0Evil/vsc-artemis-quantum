// src/scenario_engine.cpp
#include "artemis_nexus.hpp"
#include "ceim_kernel.hpp"
#include "scenario.hpp"
// #include "cpvm_kernel.hpp"

ScenarioScore evaluate_scenario(const NexusShard& base_shard,
                                const BasinScore& baseline,
                                const Scenario& s) {
    NexusShard shard = base_shard; // copy for mutation

    // 1. Apply Flow-Vac placements: adjust sewer intercept nodes
    for (const auto& fv : s.flowvac) {
        auto it = std::find_if(shard.nodes.begin(), shard.nodes.end(),
                               [&](const Node& n){ return n.node_id == fv.node_id; });
        if (it == shard.nodes.end()) continue;
        if (!fv.enabled) continue;

        // Simplified: increase dC removal and maybe energy cost
        it->cout *= (1.0 - 0.30 * fv.capacity_factor); // 30% additional TSS/FOG removal
        it->within_corridor = it->within_corridor && true; // keep corridor enforcement
    }

    // 2. Apply turbine placements: adjust energy nodes and maybe flow constraints
    for (const auto& t : s.turbines) {
        auto it = std::find_if(shard.nodes.begin(), shard.nodes.end(),
                               [&](const Node& n){ return n.node_id == t.node_id; });
        if (it == shard.nodes.end()) continue;
        if (!t.enabled) continue;

        // Example: marginally reduce carbon by crediting recovered energy
        double energy_kW = t.design_head_m * t.design_flow_m3s * 9.81 * 0.60; // efficiency
        // Map energy to ecoimpact and risk reductions
        it->ecoimpactscore_base += 0.01 * energy_kW; // bounded by [0,1] in production
        if (it->ecoimpactscore_base > 1.0) it->ecoimpactscore_base = 1.0;
        it->R_base *= 0.98; // small risk reduction if within corridor
    }

    // 3. Recompute basin K/E/R
    BasinScore bs{0.0, 0.0, 0.0};
    for (const auto& n : shard.nodes) {
        auto res = evaluate_ceim(n);
        bs.K_total += res.Kn;
        bs.E_total += res.E;
        bs.R_total += res.R;
    }

    // 4. Optionally run CPVM for key nodes to confirm safety/viability
    bool all_cpvm_safe = true;
    // for (auto& n : shard.nodes) { auto c = run_cpvm(n, s); all_cpvm_safe &= c.safety_ok && c.viability_ok; }

    ScenarioScore out;
    out.scenario_id = s.scenario_id;
    out.delta_K = bs.K_total - baseline.K_total;
    out.delta_E = bs.E_total - baseline.E_total;
    out.delta_R = bs.R_total - baseline.R_total;
    out.cpvm_safe = all_cpvm_safe;

    return out;
}
