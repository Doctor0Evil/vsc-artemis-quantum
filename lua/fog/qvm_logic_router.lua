-- qvm_logic_router.lua
-- QVM-specific eco-aware routing predicates for FOG routers
-- Pure Lua, no external dependencies

local QVM_LOGIC_LANE = "QVM-LOGIC-RESILIENCE"
local ENERGY_SURPLUS_THRESHOLD = 0.7
local RF_SAFE_MAX = 0.6
local RF_GOLD_MAX = 0.9
local EPSILON = 1e-6

local function clamp(val, min_val, max_val)
    return math.max(min_val, math.min(max_val, val))
end

local function evaluate_node_tailwind(qvm_region, node_metrics)
    if node_metrics.lane ~= QVM_LOGIC_LANE then return false, "invalid_lane" end
    local energy_surplus = clamp(node_metrics.energy_surplus or 0, 0, 1)
    local rf_residual = clamp(node_metrics.rf_residual or 0, 0, 1)
    if energy_surplus < ENERGY_SURPLUS_THRESHOLD then return false, "energy_insufficient" end
    if rf_residual > RF_GOLD_MAX then return false, "rf_corridor_breach" end
    return true, "ok"
end

local function check_vt_nonincreasing(prev_vt, next_vt, epsilon)
    epsilon = epsilon or EPSILON
    if next_vt > prev_vt + epsilon then return false, "vt_increase" end
    return true, "ok"
end

local function route_qvm_job(job_profile, candidate_nodes, prev_logic_vt)
    if not candidate_nodes or #candidate_nodes == 0 then return {}, "no_candidates" end
    local valid_nodes = {}
    local reject_reasons = {}
    for _, node in ipairs(candidate_nodes) do
        local tailwind_ok, tailwind_reason = evaluate_node_tailwind(job_profile.region, node.metrics)
        if not tailwind_ok then
            reject_reasons[node.nodeid] = tailwind_reason
        else
            local vt_ok, vt_reason = check_vt_nonincreasing(prev_logic_vt, node.metrics.vtlogic, EPSILON)
            if not vt_ok then
                reject_reasons[node.nodeid] = vt_reason
            else
                if node.metrics.roh_band_max <= (job_profile.roh_budget or 0.30) and
                   node.metrics.veco_band_max <= (job_profile.veco_budget or 0.30) then
                    table.insert(valid_nodes, node)
                else
                    reject_reasons[node.nodeid] = "budget_exceeded"
                end
            end
        end
    end
    if #valid_nodes == 0 then return {}, "all_rejected", reject_reasons end
    return valid_nodes, "ok", reject_reasons
end

return {
    evaluate_node_tailwind = evaluate_node_tailwind,
    check_vt_nonincreasing = check_vt_nonincreasing,
    route_qvm_job = route_qvm_job,
    QVM_LOGIC_LANE = QVM_LOGIC_LANE,
}
