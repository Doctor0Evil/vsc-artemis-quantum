// Filename: include/artemis/hydraulic/sewer_kernel.hpp
// Destination: ./include/artemis/hydraulic/sewer_kernel.hpp
#pragma once
#include <string>
#include "artemis/metrics.hpp"
#include "artemis/corridors.hpp"

namespace artemis::hydraulic {

struct SewerNodeInput {
    std::string node_id;
    double flow_m3s;
    double cin_tss_mgL;
    double cout_tss_mgL;
    double cin_fog_mgL;
    double cout_fog_mgL;
    double residence_time_min;
    double climate_index; // e.g. 0–1 humidity/temperature factor
};

struct SewerNodeCorridors {
    CorridorBand r_tss;
    CorridorBand r_fog;
    CorridorBand r_res_time;
};

struct SewerNodeOutput {
    MetricFields metrics;
};

SewerNodeOutput evaluate_node(const SewerNodeInput& in,
                              const SewerNodeCorridors& cb,
                              const MetricFields& basin_before);

} // namespace artemis::hydraulic
