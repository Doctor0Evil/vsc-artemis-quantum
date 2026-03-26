// Filename: include/artemis/neuromorphic/snn_quality_kernel.hpp
// Destination: ./include/artemis/neuromorphic/snn_quality_kernel.hpp
#pragma once
#include <string>
#include "artemis/metrics.hpp"
#include "artemis/corridors.hpp"

namespace artemis::neuromorphic {

struct SnnInput {
    std::string node_id;
    double event_rate_hz;
    double false_positive_rate;
    double false_negative_rate;
    double drift_score;   // 0–1
};

struct SnnCorridors {
    CorridorBand r_sensor;
    CorridorBand r_uncertainty;
};

struct SnnOutput {
    MetricFields metrics;
};

SnnOutput evaluate_node(const SnnInput& in,
                        const SnnCorridors& cb,
                        const MetricFields& basin_before);

} // namespace artemis::neuromorphic
