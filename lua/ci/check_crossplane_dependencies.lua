-- check_crossplane_dependencies.lua
-- Static analyzer for cross-plane dependency enforcement
-- Integrates with GitHub Actions CI

local json = require("dkjson") or { decode = function(s) return load("return " .. s)() end }

local QVM_RESILIENCE_CRATES = {
    ["logicalqubits"] = true,
    ["qpushards"] = true,
    ["qvm-resilience"] = true,
}

local RESTRICTED_ROLES = { ["reward"] = true, ["bi"] = true }

local function load_manifest(path)
    local f = io.open(path, "r")
    if not f then error("Cannot open manifest: " .. path) end
    local content = f:read("*all")
    f:close()
    return json.decode(content)
end

local function dfs(node, graph, visited, rec_stack, path, violations)
    visited[node] = true
    rec_stack[node] = true
    table.insert(path, node)
    for _, dep in ipairs(graph[node].dependencies or {}) do
        if not visited[dep] then
            dfs(dep, graph, visited, rec_stack, path, violations)
        elseif rec_stack[dep] then
            if QVM_RESILIENCE_CRATES[dep] and RESTRICTED_ROLES[graph[node].role] then
                local violation_path = {}
                for i, p in ipairs(path) do violation_path[i] = p end
                table.insert(violation_path, dep)
                table.insert(violations, violation_path)
            end
        end
    end
    rec_stack[node] = nil
    table.remove(path)
end

local function check_dependencies(manifest_path)
    local manifest = load_manifest(manifest_path)
    local graph = manifest.crates or {}
    local visited = {}
    local violations = {}
    for crate_name, crate_info in pairs(graph) do
        if not visited[crate_name] then
            dfs(crate_name, graph, visited, {}, {}, violations)
        end
    end
    if #violations > 0 then
        print("DEPENDENCY VIOLATIONS DETECTED:")
        for i, v in ipairs(violations) do
            print(string.format("  [%d] %s", i, table.concat(v, " -> ")))
        end
        os.exit(1)
    else
        print("No cross-plane dependency violations found.")
        os.exit(0)
    end
end

if arg and arg[1] then
    check_dependencies(arg[1])
else
    print("Usage: lua check_crossplane_dependencies.lua <manifest_path>")
    os.exit(1)
end
