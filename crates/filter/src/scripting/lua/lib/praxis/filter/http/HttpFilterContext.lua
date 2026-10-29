-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")
local Bytes = require("praxis.util.Bytes")
local BodyMode = require("praxis.filter.http.BodyMode")

---A name/value header pair, used by the ordered header lists.
---@alias praxis.filter.http.HeaderPair { [1]: string, [2]: string }

---The fields accepted by `praxis.filter.http.HttpFilterContext.new`.
---
---Only `request` is required. The host supplies every field for each hook;
---omitted optional fields fall back to an empty `Bytes`, `BodyMode.stream()`,
---`0`, or an empty table.
---@alias praxis.filter.http.HttpFilterContextFields
---| { request: praxis.filter.http.Request, response?: praxis.filter.http.Response, buffered_request_body?: praxis.util.Bytes, client_addr?: praxis.net.IpAddr, cluster?: string, extra_request_headers?: praxis.filter.http.HeaderPair[], request_headers_to_remove?: string[], request_headers_to_set?: praxis.filter.http.HeaderPair[], request_body_mode?: praxis.filter.http.BodyMode, request_body_bytes?: integer, response_body_mode?: praxis.filter.http.BodyMode, response_body_bytes?: integer }

---Per-request state passed to HTTP filter hooks.
---
---This is the Lua counterpart of the Rust `HttpFilterContext`. The host builds
---a fresh context for each hook, so edits a script makes to it are discarded.
---`response()` is nil during request-phase hooks.
---
---Header pairs are one-based arrays of two-element arrays:
---`{ { "x-a", "1" }, { "x-b", "2" } }`.
---@class praxis.filter.http.HttpFilterContext: praxis.util.Object
---@field private _buffered_request_body praxis.util.Bytes The buffered request body; empty when none was captured.
---@field private _client_addr? praxis.net.IpAddr The downstream client address.
---@field private _cluster? string The selected cluster name.
---@field private _extra_request_headers praxis.filter.http.HeaderPair[] Extra headers to inject upstream.
---@field private _request_headers_to_remove string[] Headers to remove upstream.
---@field private _request_headers_to_set praxis.filter.http.HeaderPair[] Headers to set upstream.
---@field private _request_body_mode praxis.filter.http.BodyMode The effective request body mode.
---@field private _request_body_bytes integer Accumulated request body bytes.
---@field private _request praxis.filter.http.Request The request.
---@field private _response_body_mode praxis.filter.http.BodyMode The effective response body mode.
---@field private _response_body_bytes integer Accumulated response body bytes.
---@field private _response? praxis.filter.http.Response The response; nil before one exists.
local HttpFilterContext = setmetatable({}, { __index = Object })
HttpFilterContext.__index = HttpFilterContext

---Creates a new `praxis.filter.http.HttpFilterContext`.
---@param fields? praxis.filter.http.HttpFilterContextFields The context fields.
---@return praxis.filter.http.HttpFilterContext The new context.
function HttpFilterContext.new(fields)
  fields = fields or {}
  local self = setmetatable(Object.new(), HttpFilterContext)
  ---@cast self praxis.filter.http.HttpFilterContext
  self._buffered_request_body = fields.buffered_request_body or Bytes.new("")
  self._client_addr = fields.client_addr
  self._cluster = fields.cluster
  self._extra_request_headers = fields.extra_request_headers or {}
  self._request_headers_to_remove = fields.request_headers_to_remove or {}
  self._request_headers_to_set = fields.request_headers_to_set or {}
  self._request_body_mode = fields.request_body_mode or BodyMode.stream()
  self._request_body_bytes = fields.request_body_bytes or 0
  self._request = fields.request
  self._response_body_mode = fields.response_body_mode or BodyMode.stream()
  self._response_body_bytes = fields.response_body_bytes or 0
  self._response = fields.response
  return self
end

---Returns the buffered request body, or an empty `Bytes` when none was captured.
---@return praxis.util.Bytes The buffered request body.
function HttpFilterContext:buffered_request_body()
  return self._buffered_request_body
end

---Returns the downstream client address, or nil when unknown.
---@return praxis.net.IpAddr? The client address.
function HttpFilterContext:client_addr()
  return self._client_addr
end

---Returns the cluster name selected by the router, or nil when none was selected.
---@return string? The cluster name.
function HttpFilterContext:cluster()
  return self._cluster
end

---Returns the extra headers to inject into the upstream request.
---@return praxis.filter.http.HeaderPair[] The extra headers.
function HttpFilterContext:extra_request_headers()
  return self._extra_request_headers
end

---Returns the header names to remove from the upstream request.
---@return string[] The header names.
function HttpFilterContext:request_headers_to_remove()
  return self._request_headers_to_remove
end

---Returns the headers to set on the upstream request.
---@return praxis.filter.http.HeaderPair[] The headers.
function HttpFilterContext:request_headers_to_set()
  return self._request_headers_to_set
end

---Returns the effective request body mode.
---@return praxis.filter.http.BodyMode The request body mode.
function HttpFilterContext:request_body_mode()
  return self._request_body_mode
end

---Returns the accumulated request body bytes.
---@return integer The request body byte count.
function HttpFilterContext:request_body_bytes()
  return self._request_body_bytes
end

---Returns the request.
---@return praxis.filter.http.Request The request.
function HttpFilterContext:request()
  return self._request
end

---Returns the effective response body mode.
---@return praxis.filter.http.BodyMode The response body mode.
function HttpFilterContext:response_body_mode()
  return self._response_body_mode
end

---Returns the accumulated response body bytes.
---@return integer The response body byte count.
function HttpFilterContext:response_body_bytes()
  return self._response_body_bytes
end

---Returns the response, or nil when no response exists yet.
---@return praxis.filter.http.Response? The response.
function HttpFilterContext:response()
  return self._response
end

return HttpFilterContext
