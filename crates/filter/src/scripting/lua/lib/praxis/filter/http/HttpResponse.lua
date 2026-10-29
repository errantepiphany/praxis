-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local HttpMessage = require("praxis.filter.http.HttpMessage")

---An HTTP response: a `praxis.filter.http.HttpMessage` plus a status code.
---@class praxis.filter.http.HttpResponse: praxis.filter.http.HttpMessage
---@field private _status integer The HTTP status code (for example `200`).
local HttpResponse = setmetatable({}, { __index = HttpMessage })
HttpResponse.__index = HttpResponse

---Creates a new `praxis.filter.http.HttpResponse`.
---
---The `headers` and `body` arguments are passed to the
---`praxis.filter.http.HttpMessage` parent class.
---@param headers praxis.filter.http.HttpHeaders Passed to the `HttpMessage` parent class.
---@param body praxis.filter.http.HttpBody Passed to the `HttpMessage` parent class.
---@param status integer The HTTP status code.
---@return praxis.filter.http.HttpResponse The new response.
function HttpResponse.new(headers, body, status)
  local self = setmetatable(HttpMessage.new(headers, body), HttpResponse)
  ---@cast self praxis.filter.http.HttpResponse
  self._status = status
  return self
end

---Returns the HTTP status code.
---@return integer The status code.
function HttpResponse:status()
  return self._status
end

return HttpResponse
