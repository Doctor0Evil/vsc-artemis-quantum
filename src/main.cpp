// src/main.cpp
#include "artemis_nexus.hpp"
#include "scenario.hpp"
#include <algorithm>
#include <iostream>

int main(int argc, char** argv) {
    if (argc < 3) {
        std::cerr << "Usage: basin_planner <nexus_aln_path> <scenarios_aln_or_json>\n";
        return 1;
    }
    NexusShard shard = load_nexus_shard(argv[1]);
    BasinScore baseline = compute_basin_score(shard);

    std::vector<Scenario> scenarios = load_scenarios(argv[2]);

    std::vector<ScenarioScore> scores;
    scores.reserve(scenarios.size());
    for (const auto& s : scenarios) {
        scores.push_back(evaluate_scenario(shard, baseline, s));
    }

    std::sort(scores.begin(), scores.end(),
              [](const ScenarioScore& a, const ScenarioScore& b) {
                  // Rank primarily by ΔK (impact), then ΔE, minimize ΔR
                  if (a.delta_K != b.delta_K) return a.delta_K > b.delta_K;
                  if (a.delta_E != b.delta_E) return a.delta_E > b.delta_E;
                  return a.delta_R < b.delta_R;
              });

    // Output CSV or ALN results
    std::cout << "scenario_id,delta_K,delta_E,delta_R,cpvm_safe\n";
    for (const auto& s : scores) {
        std::cout << s.scenario_id << ","
                  << s.delta_K << ","
                  << s.delta_E << ","
                  << s.delta_R << ","
                  << (s.cpvm_safe ? "true" : "false") << "\n";
    }
    return 0;
}
