-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---A URI, split into its scheme, authority, path and query parts.
---
---The host builds these from the request: proxy requests normally carry an
---origin-form target, so the scheme comes from the connection (`https` when
---TLS terminated downstream, `http` otherwise) and the authority from the
---request URI when it has one, falling back to the `Host` header. The query
---is nil when the URI has no query string.
---@class praxis.net.Uri: praxis.util.Object
---@field private _scheme string The scheme; never nil.
---@field private _authority string? The authority, or nil when the request has none.
---@field private _path string The path; never nil (at least the empty string).
---@field private _query string? The query string, or nil when the URI has none.
local Uri = setmetatable({}, { __index = Object })
Uri.__index = Uri

---Creates a new `praxis.net.Uri`.
---@param scheme string The scheme.
---@param authority? string The authority, or nil when the request has none.
---@param path string The path.
---@param query? string The query string, or nil when the URI has none.
---@return praxis.net.Uri The new URI.
function Uri.new(scheme, authority, path, query)
  local self = setmetatable(Object.new(), Uri)
  ---@cast self praxis.net.Uri
  self._scheme = scheme
  self._authority = authority
  self._path = path
  self._query = query
  return self
end

---Returns the scheme.
---@return string The scheme; never nil.
function Uri:scheme()
  return self._scheme
end

---Returns the authority.
---@return string? The authority, or nil when the request has none.
function Uri:authority()
  return self._authority
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

return Uri
