-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")
local Log = require("praxis.log.Log")
local FilterAction = require("praxis.filter.FilterAction")

---Base class for HTTP filters.
---
---Subclass `praxis.filter.http.HttpFilter` and override the `on_*` callback
---stubs to inspect requests and responses as they stream through. Each callback
---receives the objects the host builds for that call; mutating them does not
---persist between calls.
---
---The proxy dispatches `on_request`; the other hooks are not yet dispatched,
---so overriding them currently has no effect.
---@class praxis.filter.http.HttpFilter: praxis.util.Object
---@field private _log praxis.log.Log The log.
local HttpFilter = setmetatable({}, { __index = Object })
HttpFilter.__index = HttpFilter

---Creates a new `praxis.filter.http.HttpFilter`.
---@return praxis.filter.http.HttpFilter The new filter.
function HttpFilter.new()
  local self = setmetatable(Object.new(), HttpFilter)
  ---@cast self praxis.filter.http.HttpFilter
  self._log = Log.new("praxis.filter.http.HttpFilter")
  return self
end

---Hook stub, intended to run when the request headers arrive. Override in
---subclasses. `ctx:response()` is nil in this phase.
---@param ctx praxis.filter.http.HttpContext The filter context.
---@return praxis.filter.FilterAction
---@diagnostic disable-next-line: unused-local
function HttpFilter:on_request(ctx)
  self._log:trace("default on_request returning continue")
  return FilterAction.continue()
end

---Hook stub, intended to run for each chunk of the request body. Override in
---subclasses. `ctx:response()` is nil in this phase; in `stream_buffer` mode
---the complete body arrives as the end-of-stream chunk.
---@param ctx praxis.filter.http.HttpContext The filter context.
---@param body praxis.filter.http.HttpBody The request body chunk.
---@param end_of_stream boolean Whether this chunk is the last one.
---@return praxis.filter.FilterAction
---@diagnostic disable-next-line: unused-local
function HttpFilter:on_request_body(ctx, body, end_of_stream)
  self._log:trace("default on_request_body returning continue")
  return FilterAction.continue()
end

---Hook stub, intended to run when the response headers arrive. Override in
---subclasses.
---@param ctx praxis.filter.http.HttpContext The filter context, with the response available.
---@return praxis.filter.FilterAction
---@diagnostic disable-next-line: unused-local
function HttpFilter:on_response(ctx)
  self._log:trace("default on_response returning continue")
  return FilterAction.continue()
end

---Hook stub, intended to run for each chunk of the response body. Override in
---subclasses. In `stream_buffer` mode the complete body arrives as the
---end-of-stream chunk.
---@param ctx praxis.filter.http.HttpContext The filter context, with the response available.
---@param body praxis.filter.http.HttpBody The response body chunk.
---@param end_of_stream boolean Whether this chunk is the last one.
---@return praxis.filter.FilterAction
---@diagnostic disable-next-line: unused-local
function HttpFilter:on_response_body(ctx, body, end_of_stream)
  self._log:trace("default on_response_body returning continue")
  return FilterAction.continue()
end

return HttpFilter
