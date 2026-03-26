// Filename: include/artemis/lyapunov.hpp
// Destination: ./include/artemis/lyapunov.hpp
#pragma once
#include "metrics.hpp"

namespace artemis {

// Compute V_t = sum_j w_j * r_j^2
double compute_residual(const MetricFields& mf);

// Enforce V_{t+1} <= V_t (outside safe interior)
bool residual_nonincreasing(double vt_before,
                            double vt_after);

} // namespace artemis
