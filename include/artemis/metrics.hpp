// Filename: include/artemis/metrics.hpp
// Destination: ./include/artemis/metrics.hpp
#pragma once
#include <string>
#include <vector>

namespace artemis {

struct KerTriple {
    double k; // Knowledge 0–1
    double e; // Eco-impact 0–1
    double r; // Risk-of-harm 0–1
};

struct RiskCoord {
    std::string var_id;
    double r;      // normalized 0–1
    double weight; // contribution to V_t
};

struct MetricFields {
    KerTriple ker;
    std::vector<RiskCoord> rx;
    double vt; // Lyapunov residual
};

} // namespace artemis
