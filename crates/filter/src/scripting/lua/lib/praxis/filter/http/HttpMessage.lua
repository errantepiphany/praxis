-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---An HTTP message's headers, keyed by lowercase header name.
---
---A header that appears once maps to a string; a header that repeats maps to a
---one-based list of strings.
---@alias praxis.filter.http.HttpHeaders table<string, string|string[]>

---An HTTP message, consisting of a headers table and a body.
---@class praxis.filter.http.HttpMessage: praxis.util.Object
---@field private _headers praxis.filter.http.HttpHeaders The headers.
---@field private _body praxis.filter.http.HttpBody The body.
local HttpMessage = setmetatable({}, { __index = Object })
HttpMessage.__index = HttpMessage

---Creates a new `praxis.filter.http.HttpMessage`.
---@param headers praxis.filter.http.HttpHeaders The headers.
---@param body praxis.filter.http.HttpBody The body.
---@return praxis.filter.http.HttpMessage The new message.
function HttpMessage.new(headers, body)
  local self = setmetatable(Object.new(), HttpMessage)
  ---@cast self praxis.filter.http.HttpMessage
  self._headers = headers
  self._body = body
  return self
end

---Returns the headers.
---@return praxis.filter.http.HttpHeaders The headers.
function HttpMessage:headers()
  return self._headers
end

---Returns the body.
---@return praxis.filter.http.HttpBody The body.
function HttpMessage:body()
  return self._body
end

return HttpMessage
