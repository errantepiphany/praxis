-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---The `praxis.filter.http` namespace: classes for writing HTTP filters.
---@class praxis.filter.http
---@field BodyAccess praxis.filter.http.BodyAccess
---@field BodyMode praxis.filter.http.BodyMode
---@field HeaderMap praxis.filter.http.HeaderMap
---@field Request praxis.filter.http.Request
---@field Response praxis.filter.http.Response
---@field HttpFilterContext praxis.filter.http.HttpFilterContext
---@field HttpFilter praxis.filter.http.HttpFilter
---@field private _filter? praxis.filter.http.HttpFilter The registered filter.
local http = {}

---Body access declarations (`none`, `read_only`, `read_write`).
http.BodyAccess = require("praxis.filter.http.BodyAccess")

---Body delivery modes (`stream`, `stream_buffer`, `size_limit`).
http.BodyMode = require("praxis.filter.http.BodyMode")

---An HTTP header map with case-insensitive lookups.
http.HeaderMap = require("praxis.filter.http.HeaderMap")

---An HTTP request head: headers, method and URI.
http.Request = require("praxis.filter.http.Request")

---An HTTP response head: headers and status code.
http.Response = require("praxis.filter.http.Response")

---The filter context: request, response and per-request state.
http.HttpFilterContext = require("praxis.filter.http.HttpFilterContext")

---Base class for HTTP filters.
http.HttpFilter = require("praxis.filter.http.HttpFilter")

---Sets the HTTP filter to use. Passing nil clears it; the proxy then
---substitutes an empty filter that defines no hooks.
---@param filter? praxis.filter.http.HttpFilter The HTTP filter to use.
---@return nil
http.set = function(filter)
  http._filter = filter
end

-- Set a default HTTP filter.
http.set(http.HttpFilter.new())

---Gets the HTTP filter to use.
---@return praxis.filter.http.HttpFilter? The HTTP filter to use, or nil
---after `set(nil)`.
http.get = function()
  return http._filter
end

return http
