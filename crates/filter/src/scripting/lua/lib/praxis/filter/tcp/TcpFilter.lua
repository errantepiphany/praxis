-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")
local FilterAction = require("praxis.filter.FilterAction")

local log = require("praxis.log.Log").new("praxis.filter.tcp.TcpFilter")

---Base class for TCP filters.
---
---Subclass `praxis.filter.tcp.TcpFilter` and override the `on_*` callback
---stubs to observe TCP connections. The proxy builds a fresh
---`praxis.filter.tcp.TcpFilterContext` for each callback. The default filter
---that `praxis.filter.tcp` registers at load time is a plain `TcpFilter`, whose
---stubs return `CONTINUE`.
---@class praxis.filter.tcp.TcpFilter: praxis.util.Object
local TcpFilter = setmetatable({}, { __index = Object })
TcpFilter.__index = TcpFilter

---Creates a new `praxis.filter.tcp.TcpFilter`.
---@return praxis.filter.tcp.TcpFilter The new filter.
function TcpFilter.new()
  local self = setmetatable(Object.new(), TcpFilter)
  ---@cast self praxis.filter.tcp.TcpFilter
  return self
end

---Runs when a TCP connection is accepted. Override in subclasses. The default
---returns `CONTINUE`.
---@param ctx praxis.filter.tcp.TcpFilterContext The connection context.
---@return praxis.filter.FilterAction? The action to take; nil behaves as `CONTINUE`.
---@diagnostic disable-next-line: unused-local
function TcpFilter:on_connect(ctx)
  log:trace("default on_connect returning continue")
  return FilterAction.continue()
end

---Runs when a TCP connection is closed. Override in subclasses. The default
---returns `CONTINUE`; the host ignores the action on disconnect.
---@param ctx praxis.filter.tcp.TcpFilterContext The connection context.
---@return praxis.filter.FilterAction? The action to take; nil behaves as `CONTINUE`.
---@diagnostic disable-next-line: unused-local
function TcpFilter:on_disconnect(ctx)
  log:trace("default on_disconnect returning continue")
  return FilterAction.continue()
end

return TcpFilter
