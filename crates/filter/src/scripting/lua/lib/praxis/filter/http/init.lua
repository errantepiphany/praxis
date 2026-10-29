-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---The `praxis.filter.http` namespace: classes for writing HTTP filters.
---@class praxis.filter.http
---@field HttpMessage praxis.filter.http.HttpMessage
---@field HttpRequest praxis.filter.http.HttpRequest
---@field HttpResponse praxis.filter.http.HttpResponse
---@field HttpFilterContext praxis.filter.http.HttpFilterContext
---@field HttpFilter praxis.filter.http.HttpFilter
---@field private _filter? praxis.filter.http.HttpFilter The registered filter.
local http = {}

---Base HTTP message class: a headers table.
http.HttpMessage = require("praxis.filter.http.HttpMessage")

---An HTTP request: a message plus a method, URI and body.
http.HttpRequest = require("praxis.filter.http.HttpRequest")

---An HTTP response: a message plus a status code.
http.HttpResponse = require("praxis.filter.http.HttpResponse")

---The filter context: the request and, once available, the response.
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

-- Set the default filter.
http.set(http.HttpFilter.new())

---Gets the HTTP filter to use.
---@return praxis.filter.http.HttpFilter? The HTTP filter to use, or nil
---after `set(nil)`.
http.get = function()
  return http._filter
end

return http
