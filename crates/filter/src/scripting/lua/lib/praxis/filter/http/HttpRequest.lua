-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local HttpMessage = require("praxis.filter.http.HttpMessage")

---An HTTP request: a `praxis.filter.http.HttpMessage` plus a method and a URI.
---
---The URI's scheme and query are nil when the request target does not carry
---them.
---@class praxis.filter.http.HttpRequest: praxis.filter.http.HttpMessage
---@field private _method string The request method (for example `"GET"`).
---@field private _uri praxis.net.Uri The request URI.
local HttpRequest = setmetatable({}, { __index = HttpMessage })
HttpRequest.__index = HttpRequest

-- TODO: Change headers table to praxis.filter.http.HttpHeaders?

---Creates a new `praxis.filter.http.HttpRequest`.
---
---The `headers` and `body` arguments are passed to the
---`praxis.filter.http.HttpMessage` parent class.
---@param headers table Passed to the `HttpMessage` parent class.
---@param body praxis.filter.http.HttpBody Passed to the `HttpMessage` parent class.
---@param method string The request method.
---@param uri praxis.net.Uri The request URI.
---@return praxis.filter.http.HttpRequest The new request.
function HttpRequest.new(headers, body, method, uri)
  local self = setmetatable(HttpMessage.new(headers, body), HttpRequest)
  ---@cast self praxis.filter.http.HttpRequest
  self._method = method
  self._uri = uri
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

return HttpRequest
