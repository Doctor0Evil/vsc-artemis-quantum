#ifndef QVM_LOGIC_TELEMETRY_HPP
#define QVM_LOGIC_TELEMETRY_HPP

#include <cstdint>

#ifdef __cplusplus
extern "C" {
#endif

struct QvmTelemetrySnapshot {
    char logicalid[64];
    uint64_t epochms;
    double nodeavail_raw;
    double syndromeweight_raw;
    uint64_t agesincelastfullreencode_ms;
    double rftail_raw;
    double energymargin_j;
    uint32_t reencodecount;
    uint64_t interval_ms;
};

struct QvmLogicRiskCoords {
    float rnodeavail;
    float rsyndromeweight;
    float ragesincelastfullreencode;
    float rrftail;
    float renergymargin;
    float rreencodechurn;
};

struct QvmLyapunovWeights {
    float wnodeavail;
    float wsyndromeweight;
    float wagesincelastfullreencode;
    float wrftail;
    float wenergymargin;
    float wreencodechurn;
};

struct QvmCorridorBounds {
    float safe;
    float gold;
    float hard;
};

inline float normalize_coord(float raw, float safe, float hard) {
    if (raw <= safe) return 0.0f;
    if (raw >= hard) return 1.0f;
    return (raw - safe) / (hard - safe);
}

void qvm_compute_risk(const QvmTelemetrySnapshot* snap, const QvmCorridorBounds* b, QvmLogicRiskCoords* out);
float qvm_compute_v(const QvmLogicRiskCoords* r, const QvmLyapunovWeights* w);

#ifdef __cplusplus
}
#endif

#endif
