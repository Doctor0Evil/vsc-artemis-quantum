// Filename: include/artemis/carbon_health/furnace_kernel.hpp
// Destination: ./include/artemis/carbon_health/furnace_kernel.hpp
#pragma once
#include <string>
#include "artemis/metrics.hpp"
#include "artemis/corridors.hpp"

namespace artemis::carbon_health {

struct FurnaceTierInput {
    std::string node_id;
    double co2_kg_per_ton;
    double nox_g_per_m3;
    double pm10_mg_per_m3;
    double dioxin_ngTEQ_per_m3;
};

struct FurnaceCorridors {
    CorridorBand r_co2;
    CorridorBand r_nox;
    CorridorBand r_pm;
    CorridorBand r_dioxin_who;   // WHO guideline
    CorridorBand r_dioxin_legal; // local legal limit
};

struct FurnaceOutput {
    MetricFields metrics;
};

FurnaceOutput evaluate_node(const FurnaceTierInput& in,
                            const FurnaceCorridors& cb,
                            const MetricFields& basin_before);

} // namespace artemis::carbon_health
