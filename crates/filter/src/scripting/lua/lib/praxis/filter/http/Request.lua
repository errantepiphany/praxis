-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---An HTTP request head: headers, method and URI.
---
---This is the Lua counterpart of the Rust `Request` struct. It carries no body:
---body chunks reach `on_request_body` as a separate `praxis.util.Bytes`
---argument.
---
---The URI's host is nil when neither the target nor the `Host` header names
---one, and its port is nil whenever the authority names none; the query is
---nil when the target has none. The scheme is always derived from the
---connection by the host.
---@class praxis.filter.http.Request: praxis.util.Object
---@field private _headers praxis.filter.http.HeaderMap The request headers.
---@field private _method string The request method (for example `"GET"`).
---@field private _uri praxis.net.Uri The request URI.
local Request = setmetatable({}, { __index = Object })
Request.__index = Request

---Creates a new `praxis.filter.http.Request`.
---@param headers praxis.filter.http.HeaderMap The request headers.
---@param method string The request method.
---@param uri praxis.net.Uri The request URI.
---@return praxis.filter.http.Request The new request.
function Request.new(headers, method, uri)
  local self = setmetatable(Object.new(), Request)
  ---@cast self praxis.filter.http.Request
  self._headers = headers
  self._method = method
  self._uri = uri
  return self
end

---Returns the request headers.
---@return praxis.filter.http.HeaderMap The request headers.
function Request:headers()
  return self._headers
end

---Returns the request method.
---@return string The request method.
function Request:method()
  return self._method
end

---Returns the request URI.
---@return praxis.net.Uri The request URI.
function Request:uri()
  return self._uri
end

return Request
