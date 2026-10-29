-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---Query parameter values, keyed by parameter name.
---
---Each name maps to a one-based list of its values, so a parameter that
---appears more than once keeps every value in order.
---@alias praxis.net.QueryValues table<string, string[]>

---A URI, split into its scheme, host, port, path and query parts.
---
---The host builds these from the request: proxy requests normally carry an
---origin-form target, so the scheme comes from the connection (`https` when
---TLS terminated downstream, `http` otherwise) and the host and port from
---the request URI when it has an authority, falling back to the `Host`
---header. The host keeps an IPv6 literal's square brackets (`[::1]`), and
---the port is nil when the authority names none; both are nil when the
---request has no authority at all. The query is nil when the URI has no
---query string.
---@class praxis.net.Uri: praxis.util.Object
---@field private _scheme string The scheme; never nil.
---@field private _host string? The host, or nil when the request has none.
---@field private _port integer? The port, or nil when the authority names none.
---@field private _path string The path; never nil (at least the empty string).
---@field private _query string? The query string, or nil when the URI has none.
local Uri = setmetatable({}, { __index = Object })
Uri.__index = Uri

---Creates a new `praxis.net.Uri`.
---@param scheme string The scheme.
---@param host? string The host, or nil when the request has none.
---@param port? integer The port, or nil when the authority names none.
---@param path string The path.
---@param query? string The query string, or nil when the URI has none.
---@return praxis.net.Uri The new URI.
function Uri.new(scheme, host, port, path, query)
  local self = setmetatable(Object.new(), Uri)
  ---@cast self praxis.net.Uri
  self._scheme = scheme
  self._host = host
  self._port = port
  self._path = path
  self._query = query
  return self
end

---Returns the scheme.
---@return string The scheme; never nil.
function Uri:scheme()
  return self._scheme
end

---Returns the host.
---@return string? The host, or nil when the request has none.
function Uri:host()
  return self._host
end

---Returns the port.
---@return integer? The port, or nil when the authority names none.
function Uri:port()
  return self._port
end

---Returns the path.
---@return string The path.
function Uri:path()
  return self._path
end

---Returns the query string.
---@return string? The query string, or nil when the URI has none.
function Uri:query()
  return self._query
end

---Decodes one query component.
---
---`+` decodes to a space, following the `application/x-www-form-urlencoded`
---convention. A `%` that does not begin two hex digits is left as-is, so a
---malformed escape survives instead of raising.
---@param input string The encoded component.
---@return string The decoded component.
local function decode_component(input)
  return (input:gsub("%+", " "):gsub("%%(%x%x)", function(hex)
    return string.char(tonumber(hex, 16))
  end))
end

---Returns the query parameters, parsed into a table.
---
---Each parameter name maps to a one-based list of its values, so a repeated
---parameter (for example `tag=a&tag=b`) keeps every value in order, and a
---parameter that appears once still yields a one-element list. A parameter
---with no `=` maps to the empty string, as does one with an empty value.
---
---Names and values are percent-decoded, and `+` decodes to a space, as in an
---`application/x-www-form-urlencoded` payload; a `%` that does not begin a
---valid escape is left as-is.
---
---An absent or empty query string yields an empty table.
---@return praxis.net.QueryValues The query parameters, keyed by name.
function Uri:query_values()
  local values = {}
  if self._query == nil then
    return values
  end
  for pair in self._query:gmatch("[^&]+") do
    local name, value = pair:match("^([^=]*)=(.*)$")
    if name == nil then
      name = pair
      value = ""
    end
    local decoded_name = decode_component(name)
    local list = values[decoded_name]
    if list == nil then
      list = {}
      values[decoded_name] = list
    end
    list[#list + 1] = decode_component(value)
  end
  return values
end

return Uri
