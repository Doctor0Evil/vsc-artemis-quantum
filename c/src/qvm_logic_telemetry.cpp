#include "qvm_logic_telemetry.hpp"
#include <vector>
#include <cmath>

extern "C" {

void qvm_compute_risk(const QvmTelemetrySnapshot* snap, const QvmCorridorBounds* b, QvmLogicRiskCoords* out) {
    out->rnodeavail = normalize_coord(static_cast<float>(snap->nodeavail_raw), 0.9f, 0.99f);
    out->rsyndromeweight = normalize_coord(static_cast<float>(snap->syndromeweight_raw), 0.0f, 0.5f);
    out->ragesincelastfullreencode = normalize_coord(static_cast<float>(snap->agesincelastfullreencode_ms), 0.0f, 10000.0f);
    out->rrftail = normalize_coord(static_cast<float>(snap->rftail_raw), 0.0f, 1.0f);
    out->renergymargin = normalize_coord(static_cast<float>(snap->energymargin_j), 0.0f, 100.0f);
    out->rreencodechurn = normalize_coord(
        static_cast<float>(snap->reencodecount) / static_cast<float>(snap->interval_ms / 1000.0f), 0.0f, 10.0f);
}

float qvm_compute_v(const QvmLogicRiskCoords* r, const QvmLyapunovWeights* w) {
    return w->wnodeavail * r->rnodeavail * r->rnodeavail +
           w->wsyndromeweight * r->rsyndromeweight * r->rsyndromeweight +
           w->wagesincelastfullreencode * r->ragesincelastfullreencode * r->ragesincelastfullreencode +
           w->wrftail * r->rrftail * r->rrftail +
           w->wenergymargin * r->renergymargin * r->renergymargin +
           w->wreencodechurn * r->rreencodechurn * r->rreencodechurn;
}

bool qvm_process_batch(const std::vector<QvmTelemetrySnapshot>& snapshots,
                       const QvmCorridorBounds& corr,
                       const QvmLyapunovWeights& w,
                       std::vector<QvmLogicRiskCoords>* out_r,
                       std::vector<float>* out_v) {
    if (snapshots.empty()) return true;
    out_r->clear();
    out_v->clear();
    out_r->reserve(snapshots.size());
    out_v->reserve(snapshots.size());
    float prev_v = 0.0f;
    const float epsilon = 1e-6f;
    for (size_t i = 0; i < snapshots.size(); ++i) {
        QvmLogicRiskCoords r;
        qvm_compute_risk(&snapshots[i], &corr, &r);
        if (r.rnodeavail >= 1.0f || r.rsyndromeweight >= 1.0f || r.rrftail >= 1.0f) { return false; }
        float v = qvm_compute_v(&r, &w);
        if (i > 0 && v > prev_v + epsilon) { return false; }
        out_r->push_back(r);
        out_v->push_back(v);
        prev_v = v;
    }
    return true;
}

}
