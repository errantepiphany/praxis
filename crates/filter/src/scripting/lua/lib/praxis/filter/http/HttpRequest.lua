-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local HttpMessage = require("praxis.filter.http.HttpMessage")

---An HTTP request: a `praxis.filter.http.HttpMessage` plus a method, a URI,
---and a body.
---
---The URI's authority is nil when neither the target nor the `Host` header
---provides one, and the query is nil when the target has none; the scheme is
---always derived from the connection by the host.
---@class praxis.filter.http.HttpRequest: praxis.filter.http.HttpMessage
---@field private _method string The request method (for example `"GET"`).
---@field private _uri praxis.net.Uri The request URI.
---@field private _body praxis.util.Bytes The body; always present, possibly empty.
local HttpRequest = setmetatable({}, { __index = HttpMessage })
HttpRequest.__index = HttpRequest

---Creates a new `praxis.filter.http.HttpRequest`.
---
---The `headers` and `body` arguments are passed to the
---`praxis.filter.http.HttpMessage` parent class.
---@param headers praxis.filter.http.HttpHeaders Passed to the `HttpMessage` parent class.
---@param method string The request method.
---@param uri praxis.net.Uri The request URI.
---@param body praxis.util.Bytes The body.
---@return praxis.filter.http.HttpRequest The new request.
function HttpRequest.new(headers, method, uri, body)
  local self = setmetatable(HttpMessage.new(headers), HttpRequest)
  ---@cast self praxis.filter.http.HttpRequest
  self._method = method
  self._uri = uri
  self._body = body
  return self
end

---Returns the request method.
---@return string The request method.
function HttpRequest:method()
  return self._method
end

---Returns the request URI.
---@return praxis.net.Uri The request URI.
function HttpRequest:uri()
  return self._uri
end

---Returns the body.
---@return praxis.util.Bytes The body; always present, possibly empty.
function HttpRequest:body()
  return self._body
end

return HttpRequest
