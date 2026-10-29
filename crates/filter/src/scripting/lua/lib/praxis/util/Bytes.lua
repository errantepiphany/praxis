-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---Bytes wrapper.
---
---Wraps a byte string (i.e.: a request body  or a body chunk) and can inspect
---it: its byte and character lengths, whether it is empty or `nil`, and
---whether it is valid UTF-8 text.
---
---Every accessor is nil-safe. A `Bytes` built with `Bytes.new(nil)` reports a
---length of `-1`, counts as empty, and is not valid UTF-8.
---@class praxis.util.Bytes: praxis.util.Object
---@field private _data string? The body data; `nil` for an absent body.
local Bytes = setmetatable({}, { __index = Object })
Bytes.__index = Bytes

---Creates a new `praxis.util.Bytes`.
---@param data string? The body data; `""` for an empty body, `nil` for an
---absent one.
---@return praxis.util.Bytes The new body.
function Bytes.new(data)
  local self = setmetatable(Object.new(), Bytes)
  ---@cast self praxis.util.Bytes
  self._data = data
  return self
end

---Returns the body data.
---@return string? The body data, or `nil` when there is none.
function Bytes:data()
  return self._data
end

---Returns whether the body data is absent or an empty string.
---
---A `nil` body counts as empty; use `is_nil` to tell the two cases apart.
---@return boolean Whether the body data is `nil` or an empty string.
function Bytes:is_empty()
  return self._data == nil or self._data == ""
end

---Returns whether the body data is `nil`.
---
---The proxy passes `""` for an absent body, so this is `true` only for a
---`Bytes` a script built itself with `praxis.util.Bytes.new(nil)`.
---@return boolean Whether the body data is `nil`.
function Bytes:is_nil()
  return self._data == nil
end

---Returns whether the body data is valid UTF-8 text.
---
---A `nil` body is not text, so this returns `false` for it.
---
---`utf8.len` (a Lua 5.3+ standard library) returns nil for any string containing
---ill-formed UTF-8, such as truncated sequences, overlong encodings, UTF-16
---surrogates or code points above U+10FFFF.
---@return boolean Whether the body data is valid UTF-8; `false` when it is `nil`.
function Bytes:is_utf8()
  if self._data == nil then
    return false
  end
  return utf8.len(self._data) ~= nil
end

---Returns the number of UTF-8 characters in the body data, or `-1`.
---
---This is a character count, not a byte count: a multi-byte UTF-8 character
---counts once. Use `len` for the byte count.
---
---A `nil` body, or one that is not valid UTF-8, has no character count, so
---this returns `-1`; use `is_nil` and `is_utf8` to tell the cases apart.
---@return integer The UTF-8 character count; `-1` when the body data is `nil`
---or not valid UTF-8.
function Bytes:utf8_len()
  if self._data == nil then
    return -1
  end
  return utf8.len(self._data) or -1
end

---Returns the length of the body data, or `-1` when there is none.
---
---This is a byte count, not a character count: a multi-byte UTF-8 character
---contributes several bytes.
---
---A `nil` body has no length, so this returns `-1`; use `is_nil` to tell it
---apart from a zero-length body.
---@return integer The length of the body data; `-1` when it is `nil`.
function Bytes:len()
  if self._data == nil then
    return -1
  end
  return #self._data
end

return Bytes
