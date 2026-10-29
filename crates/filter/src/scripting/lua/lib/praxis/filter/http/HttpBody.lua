-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---An HTTP message body.
---
---Wraps a byte string — a request or response body, or one body chunk — and
---can tell whether those bytes are valid UTF-8 text. The host supplies an
---empty body when no bytes are available.
---@class praxis.filter.http.HttpBody: praxis.util.Object
---@field private _raw string The raw body bytes.
local HttpBody = setmetatable({}, { __index = Object })
HttpBody.__index = HttpBody

---Creates a new `praxis.filter.http.HttpBody`.
---@param raw string The raw body bytes; `""` for an empty body.
---@return praxis.filter.http.HttpBody The new body.
function HttpBody.new(raw)
  local self = setmetatable(Object.new(), HttpBody)
  ---@cast self praxis.filter.http.HttpBody
  self._raw = raw
  return self
end

---Returns the raw body bytes.
---@return string The raw body bytes.
function HttpBody:raw()
  return self._raw
end

---Returns whether the raw body bytes are valid UTF-8 text.
---
---`utf8.len` (Lua 5.4 standard library) returns nil for any string containing
---ill-formed UTF-8, such as truncated sequences, overlong encodings, UTF-16
---surrogates or code points above U+10FFFF.
---@return boolean Whether the body is valid UTF-8.
function HttpBody:is_utf8()
  return utf8.len(self._raw) ~= nil
end

return HttpBody
