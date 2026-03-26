// include/ceim_kernel.hpp
#pragma once
#include "artemis_nexus.hpp"

struct CeimResult {
    double Kn;
    double E;
    double R;
};

CeimResult evaluate_ceim(const Node& n);
