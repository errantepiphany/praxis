-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")
local FilterAction = require("praxis.filter.FilterAction")
local BodyAccess = require("praxis.filter.http.BodyAccess")
local BodyMode = require("praxis.filter.http.BodyMode")

local log = require("praxis.log.Log").new("praxis.filter.http.HttpFilter")

---Base class for HTTP filters.
---
---Subclass `praxis.filter.http.HttpFilter` and override the `on_*` callback
---stubs to inspect requests and responses as they stream through, and the
---`*_body_access`/`*_body_mode` declarations to opt into body hooks. The proxy
---builds fresh context objects for each callback, so mutations do not persist
---between calls.
---
---The proxy dispatches all four callbacks when the registered filter defines
---them, and the inherited stubs count as defined. The default filter that
---`praxis.filter.http` registers at load time is a plain `HttpFilter`, whose
---stubs all return `CONTINUE` and whose body declarations are `none`/`stream`.
---@class praxis.filter.http.HttpFilter: praxis.util.Object
local HttpFilter = setmetatable({}, { __index = Object })
HttpFilter.__index = HttpFilter

---Creates a new `praxis.filter.http.HttpFilter`.
---@return praxis.filter.http.HttpFilter The new filter.
function HttpFilter.new()
  local self = setmetatable(Object.new(), HttpFilter)
  ---@cast self praxis.filter.http.HttpFilter
  return self
end

---Declares what access this filter needs to request bodies. Override in
---subclasses. The default returns `BodyAccess.none()`.
---@return praxis.filter.http.BodyAccess The request body access.
function HttpFilter:request_body_access()
  log:trace("default request_body_access returning none")
  return BodyAccess.none()
end

---Declares the delivery mode for request body chunks. Override in subclasses.
---The default returns `BodyMode.stream()`.
---@return praxis.filter.http.BodyMode The request body mode.
function HttpFilter:request_body_mode()
  log:trace("default request_body_mode returning stream")
  return BodyMode.stream()
end

---Runs when the request headers arrive. Override in subclasses.
---`ctx:response()` is nil in this phase. The default returns `CONTINUE`.
---@param ctx praxis.filter.http.HttpFilterContext The filter context.
---@return praxis.filter.FilterAction? The action to take; nil behaves as `CONTINUE`.
---@diagnostic disable-next-line: unused-local
function HttpFilter:on_request(ctx)
  log:trace("default on_request returning continue")
  return FilterAction.continue()
end

---Runs for each chunk of the request body. Override in subclasses.
---`ctx:response()` is nil in this phase; in `stream_buffer` mode the
---accumulated body arrives as the end-of-stream chunk. The default returns
---`CONTINUE`.
---@param ctx praxis.filter.http.HttpFilterContext The filter context.
---@param body praxis.util.Bytes The request body chunk.
---@param end_of_stream boolean Whether this chunk is the last one.
---@return praxis.filter.FilterAction? The action to take; nil behaves as `CONTINUE`.
---@diagnostic disable-next-line: unused-local
function HttpFilter:on_request_body(ctx, body, end_of_stream)
  log:trace("default on_request_body returning continue")
  return FilterAction.continue()
end

---Declares what access this filter needs to response bodies. Override in
---subclasses. The default returns `BodyAccess.none()`.
---@return praxis.filter.http.BodyAccess The response body access.
function HttpFilter:response_body_access()
  log:trace("default response_body_access returning none")
  return BodyAccess.none()
end

---Declares the delivery mode for response body chunks. Override in subclasses.
---The default returns `BodyMode.stream()`.
---@return praxis.filter.http.BodyMode The response body mode.
function HttpFilter:response_body_mode()
  log:trace("deault response_body_mode returning stream")
  return BodyMode.stream()
end

---Runs when the response headers arrive. Override in subclasses. The default
---returns `CONTINUE`.
---@param ctx praxis.filter.http.HttpFilterContext The filter context, with the response available.
---@return praxis.filter.FilterAction? The action to take; nil behaves as `CONTINUE`.
---@diagnostic disable-next-line: unused-local
function HttpFilter:on_response(ctx)
  log:trace("default on_response returning continue")
  return FilterAction.continue()
end

---Runs for each chunk of the response body. Override in subclasses. In
---`stream_buffer` mode the accumulated body arrives as the end-of-stream
---chunk. The default returns `CONTINUE`.
---@param ctx praxis.filter.http.HttpFilterContext The filter context, with the response available.
---@param body praxis.util.Bytes The response body chunk.
---@param end_of_stream boolean Whether this chunk is the last one.
---@return praxis.filter.FilterAction? The action to take; nil behaves as `CONTINUE`.
---@diagnostic disable-next-line: unused-local
function HttpFilter:on_response_body(ctx, body, end_of_stream)
  log:trace("default on_response_body returning continue")
  return FilterAction.continue()
end

return HttpFilter
