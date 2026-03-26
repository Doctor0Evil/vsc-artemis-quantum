// include/artemis_nexus.hpp
#pragma once
#include <string>
#include <vector>

enum class NodeType {
    CyboquaticReservoir,
    FlowVacSewerIntercept,
    CyboquaticCanalEnergyNode,
    CybocindricTier
};

struct Node {
    std::string node_id;
    NodeType    node_type;
    std::string waterbody;
    std::string region;
    double      lat;
    double      lon;

    std::string ceim_profile;
    std::string cpvm_profile;
    std::string corridor_band_id;

    double      cin;
    double      cout;
    double      flow_m3s;
    double      cref;
    double      hazard_weight;

    double      ecoimpactscore_base;
    double      karmaperton_base;
    double      K_base;    // baseline CEIM Kn
    double      E_base;    // baseline ecoimpact
    double      R_base;    // baseline risk index

    bool        within_corridor;
};

enum class EdgeType { HYDRAULIC, ENERGY, MASS_WASTE };

struct Edge {
    std::string from_node;
    std::string to_node;
    EdgeType    edge_type;
    double      capacity;
};

struct NexusShard {
    std::string shard_id;
    std::string producer_bostrom;
    std::vector<Node> nodes;
    std::vector<Edge> edges;
};
