// include/scenario.hpp
#pragma once
#include <string>
#include <vector>

struct FlowVacPlacement {
    std::string node_id;   // candidate sewer node
    bool        enabled;
    double      capacity_factor; // 0-1
};

struct TurbinePlacement {
    std::string node_id;   // candidate canal/energy node
    bool        enabled;
    double      design_head_m;
    double      design_flow_m3s;
};

struct Scenario {
    std::string                 scenario_id;
    std::vector<FlowVacPlacement> flowvac;
    std::vector<TurbinePlacement> turbines;
};

struct ScenarioScore {
    std::string scenario_id;
    double      delta_K;
    double      delta_E;
    double      delta_R;
    bool        cpvm_safe;   // all nodes safety_ok & viability_ok
};
