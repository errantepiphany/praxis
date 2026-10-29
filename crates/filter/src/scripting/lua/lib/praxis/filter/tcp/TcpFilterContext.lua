-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---The fields accepted by `praxis.filter.tcp.TcpFilterContext.new`.
---
---All fields are optional except `remote_addr`, `local_addr`, `connect_time`,
---`bytes_in` and `bytes_out`, which fall back to empty strings, `0` and `0`.
---@alias praxis.filter.tcp.TcpFilterContextFields
---| { remote_addr: string, local_addr: string, sni?: string, upstream_addr?: string, lb_selection?: praxis.filter.tcp.TcpLoadSelection, cluster?: string, connect_time: number, bytes_in: integer, bytes_out: integer }

---Per-connection state passed to TCP filter hooks.
---
---This is the Lua counterpart of the Rust `TcpFilterContext`: the addresses,
---SNI, selected endpoint, cluster, connect time and byte counters for one
---connection.
---
---    local ctx = require("praxis.filter.tcp.TcpFilterContext")
---    local current = ctx.new({ remote_addr = "127.0.0.1:1234", local_addr = "0.0.0.0:5432", connect_time = 0, bytes_in = 0, bytes_out = 0 })
---    assert(current:remote_addr() == "127.0.0.1:1234")
---@class praxis.filter.tcp.TcpFilterContext: praxis.util.Object
---@field private _remote_addr string The remote client address.
---@field private _local_addr string The local listener address.
---@field private _sni? string The SNI hostname, if present.
---@field private _upstream_addr? string The upstream address, if selected.
---@field private _lb_selection? praxis.filter.tcp.TcpLoadSelection The load-balancer selection, if any.
---@field private _cluster? string The cluster name, if any.
---@field private _connect_time number The connection accept time in Unix-epoch seconds.
---@field private _bytes_in integer Bytes received from the client.
---@field private _bytes_out integer Bytes sent to the client.
local TcpFilterContext = setmetatable({}, { __index = Object })
TcpFilterContext.__index = TcpFilterContext

---Creates a new `praxis.filter.tcp.TcpFilterContext`.
---@param fields? praxis.filter.tcp.TcpFilterContextFields The context fields.
---@return praxis.filter.tcp.TcpFilterContext The new context.
function TcpFilterContext.new(fields)
  fields = fields or {}
  local self = setmetatable(Object.new(), TcpFilterContext)
  ---@cast self praxis.filter.tcp.TcpFilterContext
  self._remote_addr = fields.remote_addr or ""
  self._local_addr = fields.local_addr or ""
  self._sni = fields.sni
  self._upstream_addr = fields.upstream_addr
  self._lb_selection = fields.lb_selection
  self._cluster = fields.cluster
  self._connect_time = fields.connect_time or 0
  self._bytes_in = fields.bytes_in or 0
  self._bytes_out = fields.bytes_out or 0
  return self
end

---Returns the remote client address.
---@return string The remote address.
function TcpFilterContext:remote_addr()
  return self._remote_addr
end

---Returns the local listener address.
---@return string The local address.
function TcpFilterContext:local_addr()
  return self._local_addr
end

---Returns the SNI hostname, if present.
---@return string? The SNI hostname; nil when absent.
function TcpFilterContext:sni()
  return self._sni
end

---Returns the upstream address, if selected.
---@return string? The upstream address; nil before one is selected.
function TcpFilterContext:upstream_addr()
  return self._upstream_addr
end

---Returns the load-balancer selection, if any.
---@return praxis.filter.tcp.TcpLoadSelection? The selection; nil when none exists.
function TcpFilterContext:lb_selection()
  return self._lb_selection
end

---Returns the cluster name, if any.
---@return string? The cluster name; nil when none is configured.
function TcpFilterContext:cluster()
  return self._cluster
end

---Returns the connection accept time in fractional Unix-epoch seconds.
---@return number The accept time.
function TcpFilterContext:connect_time()
  return self._connect_time
end

---Returns the number of bytes received from the client.
---@return integer The received byte count.
function TcpFilterContext:bytes_in()
  return self._bytes_in
end

---Returns the number of bytes sent to the client.
---@return integer The sent byte count.
function TcpFilterContext:bytes_out()
  return self._bytes_out
end

return TcpFilterContext
