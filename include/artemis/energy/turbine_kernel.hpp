// Filename: include/artemis/energy/turbine_kernel.hpp
// Destination: ./include/artemis/energy/turbine_kernel.hpp
#pragma once
#include <string>
#include "artemis/metrics.hpp"
#include "artemis/corridors.hpp"

namespace artemis::energy {

struct TurbineInput {
    std::string node_id;
    double head_m;
    double flow_m3s;
    double efficiency;
    double fish_mortality_rate;
};

struct TurbineCorridors {
    CorridorBand r_energy;
    CorridorBand r_carbon;
    CorridorBand r_fish;
};

struct TurbineOutput {
    MetricFields metrics;
    double power_kw;
};

TurbineOutput evaluate_node(const TurbineInput& in,
                            const TurbineCorridors& cb,
                            const MetricFields& basin_before);

} // namespace artemis::energy
