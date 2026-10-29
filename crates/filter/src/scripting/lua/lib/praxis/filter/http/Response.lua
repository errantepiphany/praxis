-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---An HTTP response head: headers and status code.
---
---This is the Lua counterpart of the Rust `Response` struct. It carries no
---body: response chunks reach `on_response_body` as a separate
---`praxis.util.Bytes` argument.
---@class praxis.filter.http.Response: praxis.util.Object
---@field private _headers praxis.filter.http.HeaderMap The response headers.
---@field private _status integer The HTTP status code (for example `200`).
local Response = setmetatable({}, { __index = Object })
Response.__index = Response

---Creates a new `praxis.filter.http.Response`.
---@param headers praxis.filter.http.HeaderMap The response headers.
---@param status integer The HTTP status code.
---@return praxis.filter.http.Response The new response.
function Response.new(headers, status)
  local self = setmetatable(Object.new(), Response)
  ---@cast self praxis.filter.http.Response
  self._headers = headers
  self._status = status
  return self
end

---Returns the response headers.
---@return praxis.filter.http.HeaderMap The response headers.
function Response:headers()
  return self._headers
end

---Returns the HTTP status code.
---@return integer The status code.
function Response:status()
  return self._status
end

return Response
