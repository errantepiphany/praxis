-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---A header value: a string when the header appears once, or a one-based list
---of strings when it repeats.
---@alias praxis.filter.http.HeaderValue string|string[]

---An HTTP header table, keyed by lowercase header name.
---@alias praxis.filter.http.HeaderTable table<string, praxis.filter.http.HeaderValue>

---An HTTP header map.
---
---This is a simplified Lua counterpart of the Rust `http::HeaderMap`. It holds
---a nested table whose keys are lowercase header names and whose values are
---either a string (one value) or a one-based list of strings (repeated
---headers). The map is a read-only snapshot built by the host for each hook.
---
---    local HeaderMap = require("praxis.filter.http.HeaderMap")
---
---    local headers = HeaderMap.new({ ["x-tag"] = { "a", "b" } })
---    assert(headers:get("X-Tag")[1] == "a")
---    assert(#headers:get_all("x-tag") == 2)
---@class praxis.filter.http.HeaderMap: praxis.util.Object
---@field private _headers praxis.filter.http.HeaderTable The underlying header table.
local HeaderMap = setmetatable({}, { __index = Object })
HeaderMap.__index = HeaderMap

---Creates a new `praxis.filter.http.HeaderMap`.
---@param headers? praxis.filter.http.HeaderTable The header table; defaults to an empty table.
---@return praxis.filter.http.HeaderMap The new header map.
function HeaderMap.new(headers)
  local self = setmetatable(Object.new(), HeaderMap)
  ---@cast self praxis.filter.http.HeaderMap
  self._headers = headers or {}
  return self
end

---Returns the underlying header table.
---@return praxis.filter.http.HeaderTable The header table.
function HeaderMap:headers()
  return self._headers
end

---Returns a header's value, looked up case-insensitively.
---@param name string The header name.
---@return praxis.filter.http.HeaderValue? The header value; nil when the header is absent.
function HeaderMap:get(name)
  if type(name) ~= "string" then
    return nil
  end
  return self._headers[string.lower(name)]
end

---Returns all values of a header as a one-based list.
---
---A header that appears once is returned as a single-element list, so callers
---never have to branch on the stored shape. An absent header returns an empty
---list.
---@param name string The header name.
---@return string[] The header values; empty when the header is absent.
function HeaderMap:get_all(name)
  local value = self:get(name)
  if value == nil then
    return {}
  end
  if type(value) == "table" then
    return value
  end
  return { value }
end

---Returns whether a header is present.
---@param name string The header name.
---@return boolean Whether the header is present.
function HeaderMap:contains(name)
  return self:get(name) ~= nil
end

return HeaderMap
