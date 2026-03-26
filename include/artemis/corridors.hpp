// Filename: include/artemis/corridors.hpp
// Destination: ./include/artemis/corridors.hpp
#pragma once
#include <string>
#include <optional>

namespace artemis {

struct CorridorBand {
    std::string var_id;
    double safe;
    double gold;
    double hard;
    double weight;
    bool mandatory;
};

struct CorridorResult {
    bool ok;
    std::string message;
};

double normalize_metric(double value,
                        const CorridorBand& band);

CorridorResult check_corridors(double r_value,
                               const CorridorBand& band);

} // namespace artemis
