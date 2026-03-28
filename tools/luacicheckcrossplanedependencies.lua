#!/usr/bin/env lua

local cargo_lock_path = arg[1] or "Cargo.lock"
local manifest = {}

local plane_tags = {
    ["kercore"] = "shared",
    ["qpushards"] = "quantum",
    ["logicalqubits"] = "quantum",
    ["econet-reward"] = "econet",
    ["identity-core"] = "identity",
}

local forbidden_paths = {
    ["econet"] = { "qpushards", "logicalqubits" },
    ["identity"] = { "qpushards", "logicalqubits" },
    ["quantum"] = { "econet-reward", "identity-core" },
}

local function parse_cargo_lock(path)
    local file = io.open(path, "r")
    if not file then error("Cannot open " .. path) end
    local content = file:read("*all")
    file:close()
    return content
end

local function extract_dependencies(content)
    local deps = {}
    for name, plane in pairs(plane_tags) do
        deps[name] = { plane = plane, depends_on = {} }
    end
    for from, to in string.gmatch(content, "name = \"([%w%-]+)\".-dependencies = %[(.-)%]") do
        if plane_tags[from] then
            for dep in string.gmatch(to, "\"([%w%-]+)\"") do
                if plane_tags[dep] then
                    table.insert(deps[from].depends_on, dep)
                end
            end
        end
    end
    return deps
end

local function check_orthogonality(deps)
    local violations = {}
    for crate, info in pairs(deps) do
        local crate_plane = info.plane
        if forbidden_paths[crate_plane] then
            for _, dep in ipairs(info.depends_on) do
                for _, forbidden in ipairs(forbidden_paths[crate_plane]) do
                    if dep == forbidden then
                        table.insert(violations, string.format("VIOLATION: %s (%s) -> %s", crate, crate_plane, dep))
                    end
                end
            end
        end
    end
    return violations
end

local content = parse_cargo_lock(cargo_lock_path)
local deps = extract_dependencies(content)
local violations = check_orthogonality(deps)

if #violations > 0 then
    print("ORTHOGONALITY CHECK FAILED")
    for _, v in ipairs(violations) do print(v) end
    os.exit(1)
else
    print("ORTHOGONALITY CHECK PASSED")
    os.exit(0)
end
