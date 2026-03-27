#!/usr/bin/env lua5.4
-- Artemis Corridor Synergy Nexus Connector v1.0.0
-- Repository: https://github.com/Doctor0Evil/vsc-artemis-quantum
-- Contract: 0x21d4a9c73f58e2b19a40c6d8e3f7b5a1c9e2047f3a8d6b2c51e07f49a3c6d8e2b7f1049c3e8a6d2f5b1c7e9a304f6d1c9

local ffi = require("ffi")
local json = require("json")
local http = require("socket.http")
local ltn12 = require("ltn12")
local sha3 = require("sha3")
local strict = require("strict")
strict()

local NexusConnector = {}
NexusConnector.__index = NexusConnector

local KERNEL_LIB = "libartemis_corridor_kernel.so"
local API_BASE = "https://api.artemis.econet.io/v1"
local LEDGER_ENDPOINT = "https://ledger.artemis.econet.io/commit"
local KARMA_NAMESPACE = "artemis.econet"
local ACTION_RADIUS_LAT = {33.44, 33.45}
local ACTION_RADIUS_LON = {-112.08, -112.07}
local MAX_RETRY_COUNT = 3
local REQUEST_TIMEOUT = 30

ffi.cdef([[
    typedef struct {
        const char* corridor_id;
        double lat;
        double lon;
        double eco_scores[5];
        double K;
        double E;
        double R;
        uint64_t timestamp_unix;
        const char* evidence_hex;
    } CorridorStateFFI;

    typedef struct {
        const char* intervention_id;
        const char* intervention_type;
        double response_coeffs[5];
        double cost_usd;
        double energy_kwh;
        double land_m2;
        double ker_delta[3];
        const char* linked_evidence_hex;
    } InterventionDefFFI;

    typedef struct {
        const char* corridor_id;
        const char* intervention_id;
        double synergy_gain;
        double cost_usd;
        double efficiency_ratio;
        double projected_KER[3];
        uint8_t safety_flags;
    } RankedInterventionFFI;

    typedef struct {
        double weights[5];
        double max_cost_per_corridor;
        double min_KER_K;
        double max_KER_R;
        bool enforce_monotonicity;
        const char* config_version;
        uint64_t valid_from_unix;
        uint64_t valid_until_unix;
    } KernelConfigFFI;

    void* artemis_kernel_create(const KernelConfigFFI* config);
    void artemis_kernel_destroy(void* handle);
    int artemis_kernel_validate_config(const void* handle);
    int artemis_kernel_rank_interventions(
        const void* handle,
        const CorridorStateFFI* corridors,
        uint32_t corridor_count,
        const InterventionDefFFI* interventions,
        uint32_t intervention_count,
        uint32_t max_results,
        uint32_t* out_count
    );
    const char* artemis_kernel_get_version();
    const char* artemis_kernel_get_hash();
]])

local function load_kernel_library()
    local ok, lib = pcall(ffi.load, KERNEL_LIB)
    if not ok then error("Failed to load kernel library: " .. tostring(lib)) end
    return lib
end

local function validate_geographic_bounds(lat, lon)
    if lat < ACTION_RADIUS_LAT[1] or lat > ACTION_RADIUS_LAT[2] then return false end
    if lon < ACTION_RADIUS_LON[1] or lon > ACTION_RADIUS_LON[2] then return false end
    return true
end

local function validate_ker_bounds(K, E, R, min_K, max_R)
    if K < min_K or R > max_R then return false end
    if K < 0.0 or K > 1.0 then return false end
    if E < 0.0 or E > 1.0 then return false end
    if R < 0.0 or R > 1.0 then return false end
    return true
end

local function compute_shard_hash(data)
    local hash = sha3.sha3_256(data)
    return "0x" .. hash
end

local function http_get(url, headers)
    local response_body = {}
    local result, code, response_headers = http.request{
        url = url,
        method = "GET",
        headers = headers or {},
        sink = ltn12.sink.table(response_body),
        timeout = REQUEST_TIMEOUT
    }
    if not result then return nil, code end
    return table.concat(response_body), code
end

local function http_post(url, body, headers)
    local response_body = {}
    local result, code, response_headers = http.request{
        url = url,
        method = "POST",
        headers = headers or {["Content-Type"] = "application/json"},
        source = ltn12.source.string(body),
        sink = ltn12.sink.table(response_body),
        timeout = REQUEST_TIMEOUT
    }
    if not result then return nil, code end
    return table.concat(response_body), code
end

function NexusConnector.new(config)
    local self = setmetatable({}, NexusConnector)
    self.kernel_lib = load_kernel_library()
    self.config = config or {}
    self.config.weights = self.config.weights or {0.18, 0.22, 0.18, 0.24, 0.18}
    self.config.max_cost_per_corridor = self.config.max_cost_per_corridor or 5000000.0
    self.config.min_KER_K = self.config.min_KER_K or 0.85
    self.config.max_KER_R = self.config.max_KER_R or 0.20
    self.config.enforce_monotonicity = self.config.enforce_monotonicity or true
    self.config.config_version = self.config.config_version or "phoenix-2026-v1"
    self.config.valid_from_unix = self.config.valid_from_unix or 1735689600
    self.config.valid_until_unix = self.config.valid_until_unix or 1767225600
    self.config.karma_token = self.config.karma_token or nil
    self.config.identity_did = self.config.identity_did or "did:econet:artemis:corridor:synergy:v1"
    self.kernel_handle = self:_create_kernel()
    return self
end

function NexusConnector:_create_kernel()
    local config_ffi = ffi.new("KernelConfigFFI")
    for i = 0, 4 do config_ffi.weights[i] = self.config.weights[i + 1] end
    config_ffi.max_cost_per_corridor = self.config.max_cost_per_corridor
    config_ffi.min_KER_K = self.config.min_KER_K
    config_ffi.max_KER_R = self.config.max_KER_R
    config_ffi.enforce_monotonicity = self.config.enforce_monotonicity
    config_ffi.config_version = self.config.config_version
    config_ffi.valid_from_unix = self.config.valid_from_unix
    config_ffi.valid_until_unix = self.config.valid_until_unix

    local handle = self.kernel_lib.artemis_kernel_create(config_ffi)
    if handle == nil then error("Failed to create kernel handle") end

    local status = self.kernel_lib.artemis_kernel_validate_config(handle)
    if status ~= 0 then error("Kernel config validation failed: " .. tostring(status)) end

    return handle
end

function NexusConnector:load_corridor_shards(file_path)
    local file = io.open(file_path, "r")
    if not file then error("Cannot open corridor shard file: " .. file_path) end

    local corridors = {}
    local header = file:read("*line")
    if not header then error("Empty corridor shard file") end

    for line in file:lines() do
        local fields = {}
        for field in line:gmatch("([^,]+)") do table.insert(fields, field) end
        if #fields >= 9 then
            local corridor = {
                corridor_id = fields[1],
                lat = tonumber(fields[2]),
                lon = tonumber(fields[3]),
                eco_scores = {tonumber(fields[4]), tonumber(fields[5]), tonumber(fields[6]), tonumber(fields[7]), tonumber(fields[8])},
                K = tonumber(fields[9]),
                E = tonumber(fields[10]) or 0.0,
                R = tonumber(fields[11]) or 0.0,
                timestamp_unix = tonumber(fields[12]) or os.time(),
                evidence_hex = fields[13] or ""
            }
            if validate_geographic_bounds(corridor.lat, corridor.lon) then
                table.insert(corridors, corridor)
            end
        end
    end
    file:close()
    return corridors
end

function NexusConnector:load_intervention_shards(file_path)
    local file = io.open(file_path, "r")
    if not file then error("Cannot open intervention shard file: " .. file_path) end

    local interventions = {}
    local header = file:read("*line")
    if not header then error("Empty intervention shard file") end

    for line in file:lines() do
        local fields = {}
        for field in line:gmatch("([^,]+)") do table.insert(fields, field) end
        if #fields >= 8 then
            local intervention = {
                intervention_id = fields[1],
                intervention_type = fields[2],
                response_coeffs = {tonumber(fields[3]), tonumber(fields[4]), tonumber(fields[5]), tonumber(fields[6]), tonumber(fields[7])},
                cost_usd = tonumber(fields[8]),
                energy_kwh = tonumber(fields[9]) or 0.0,
                land_m2 = tonumber(fields[10]) or 0.0,
                ker_delta = {tonumber(fields[11]) or 0.0, tonumber(fields[12]) or 0.0, tonumber(fields[13]) or 0.0},
                linked_evidence_hex = fields[14] or ""
            }
            if intervention.cost_usd > 0.0 then table.insert(interventions, intervention) end
        end
    end
    file:close()
    return interventions
end

function NexusConnector:rank_interventions(corridors, interventions, max_results)
    max_results = max_results or 10

    local corridor_ffi = ffi.new("CorridorStateFFI[?]", #corridors)
    for i, corridor in ipairs(corridors) do
        local c = corridor_ffi[i - 1]
        c.corridor_id = corridor.corridor_id
        c.lat = corridor.lat
        c.lon = corridor.lon
        for j = 0, 4 do c.eco_scores[j] = corridor.eco_scores[j + 1] end
        c.K = corridor.K
        c.E = corridor.E
        c.R = corridor.R
        c.timestamp_unix = corridor.timestamp_unix
        c.evidence_hex = corridor.evidence_hex
    end

    local intervention_ffi = ffi.new("InterventionDefFFI[?]", #interventions)
    for i, intervention in ipairs(interventions) do
        local intf = intervention_ffi[i - 1]
        intf.intervention_id = intervention.intervention_id
        intf.intervention_type = intervention.intervention_type
        for j = 0, 4 do intf.response_coeffs[j] = intervention.response_coeffs[j + 1] end
        intf.cost_usd = intervention.cost_usd
        intf.energy_kwh = intervention.energy_kwh
        intf.land_m2 = intervention.land_m2
        for j = 0, 2 do intf.ker_delta[j] = intervention.ker_delta[j + 1] end
        intf.linked_evidence_hex = intervention.linked_evidence_hex
    end

    local out_count = ffi.new("uint32_t[1]")
    local results_ptr = self.kernel_lib.artemis_kernel_rank_interventions(
        self.kernel_handle,
        corridor_ffi,
        #corridors,
        intervention_ffi,
        #interventions,
        max_results,
        out_count
    )

    local ranked = {}
    if results_ptr ~= nil and out_count[0] > 0 then
        for i = 0, out_count[0] - 1 do
            local r = results_ptr[i]
            table.insert(ranked, {
                corridor_id = ffi.string(r.corridor_id),
                intervention_id = ffi.string(r.intervention_id),
                synergy_gain = r.synergy_gain,
                cost_usd = r.cost_usd,
                efficiency_ratio = r.efficiency_ratio,
                projected_KER = {r.projected_KER[0], r.projected_KER[1], r.projected_KER[2]},
                safety_flags = r.safety_flags
            })
        end
    end

    return ranked, out_count[0]
end

function NexusConnector:submit_to_ledger(shard_data, shard_type)
    local hash = compute_shard_hash(shard_data)
    local payload = {
        shard_hash = hash,
        shard_type = shard_type,
        identity_did = self.config.identity_did,
        karma_namespace = KARMA_NAMESPACE,
        timestamp_unix = os.time(),
        config_version = self.config.config_version
    }

    local headers = {["Content-Type"] = "application/json"}
    if self.config.karma_token then headers["Authorization"] = "Bearer " .. self.config.karma_token end

    local response, code = http_post(LEDGER_ENDPOINT, json.encode(payload), headers)
    if code ~= 200 then return false, "Ledger commit failed: " .. tostring(code) end

    return true, hash
end

function NexusConnector:fetch_corridor_scores(corridor_ids)
    local headers = {["Content-Type"] = "application/json"}
    if self.config.karma_token then headers["Authorization"] = "Bearer " .. self.config.karma_token end

    local url = API_BASE .. "/corridors/synergy/score?ids=" .. table.concat(corridor_ids, ",")
    local response, code = http_get(url, headers)
    if code ~= 200 then return nil, "API request failed: " .. tostring(code) end

    return json.decode(response)
end

function NexusConnector:update_karma(ceim_gain, action_type)
    local payload = {
        identity_did = self.config.identity_did,
        karma_namespace = KARMA_NAMESPACE,
        ceim_gain = ceim_gain,
        action_type = action_type,
        timestamp_unix = os.time()
    }

    local headers = {["Content-Type"] = "application/json"}
    if self.config.karma_token then headers["Authorization"] = "Bearer " .. self.config.karma_token end

    local response, code = http_post(API_BASE .. "/karma/update", json.encode(payload), headers)
    if code ~= 200 then return false, "Karma update failed: " .. tostring(code) end

    return true, json.decode(response)
end

function NexusConnector:get_kernel_version()
    return ffi.string(self.kernel_lib.artemis_kernel_get_version())
end

function NexusConnector:get_kernel_hash()
    return ffi.string(self.kernel_lib.artemis_kernel_get_hash())
end

function NexusConnector:close()
    if self.kernel_handle ~= nil then
        self.kernel_lib.artemis_kernel_destroy(self.kernel_handle)
        self.kernel_handle = nil
    end
end

NexusConnector.__gc = function(self) self:close() end

local function main()
    local connector = NexusConnector.new{
        karma_token = os.getenv("ARTEMIS_KARMA_TOKEN"),
        identity_did = "did:econet:artemis:corridor:synergy:v1"
    }

    print("Kernel Version: " .. connector:get_kernel_version())
    print("Kernel Hash: " .. connector:get_kernel_hash())

    local corridors = connector:load_corridor_shards("data/SmartCorridorEcoImpact2026v1.csv")
    local interventions = connector:load_intervention_shards("data/SmartCorridorInterventions2026v1.csv")

    print("Loaded " .. #corridors .. " corridors and " .. #interventions .. " interventions")

    local ranked, count = connector:rank_interventions(corridors, interventions, 10)
    print("Ranked " .. count .. " interventions")

    for i, r in ipairs(ranked) do
        print(string.format("%d: %s -> %s (efficiency=%.4f, K=%.3f, E=%.3f, R=%.3f)",
            i, r.corridor_id, r.intervention_id, r.efficiency_ratio,
            r.projected_KER[1], r.projected_KER[2], r.projected_KER[3]))
    end

    local shard_data = json.encode(ranked)
    local success, hash = connector:submit_to_ledger(shard_data, "ArtemisCorridorPlans2026v1")
    if success then print("Ledger commit hash: " .. hash) end

    connector:close()
end

if arg and arg[0] and arg[0]:match("nexus_connector.lua$") then main() end

return NexusConnector
