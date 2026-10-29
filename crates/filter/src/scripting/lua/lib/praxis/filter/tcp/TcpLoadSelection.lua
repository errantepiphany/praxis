-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---The load-balancer selection for a TCP connection.
---
---This mirrors the cluster and endpoint captured when the TCP load balancer
---selected an endpoint. The host builds it only when a selection exists; the
---endpoint is nil when the host cannot read one.
---
---    local selection = require("praxis.filter.tcp.TcpLoadSelection")
---    local claim = selection.new("backend", "10.0.0.1:5432")
---@class praxis.filter.tcp.TcpLoadSelection: praxis.util.Object
---@field private _cluster string The cluster captured when the endpoint was selected.
---@field private _endpoint? string The selected endpoint.
local TcpLoadSelection = setmetatable({}, { __index = Object })
TcpLoadSelection.__index = TcpLoadSelection

---Creates a new `praxis.filter.tcp.TcpLoadSelection`.
---@param cluster string The cluster captured when the endpoint was selected.
---@param endpoint? string The selected endpoint.
---@return praxis.filter.tcp.TcpLoadSelection The new selection.
function TcpLoadSelection.new(cluster, endpoint)
  local self = setmetatable(Object.new(), TcpLoadSelection)
  ---@cast self praxis.filter.tcp.TcpLoadSelection
  self._cluster = cluster
  self._endpoint = endpoint
  return self
end

---Returns the cluster captured when the endpoint was selected.
---@return string The cluster name.
function TcpLoadSelection:cluster()
  return self._cluster
end

---Returns the selected endpoint.
---@return string? The endpoint; nil when none is available.
function TcpLoadSelection:endpoint()
  return self._endpoint
end

return TcpLoadSelection
