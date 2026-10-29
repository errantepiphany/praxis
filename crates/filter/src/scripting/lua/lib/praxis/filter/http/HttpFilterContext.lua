-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---The filter context: the request and, once a response head exists, the
---response.
---
---The host passes a nil response in request-phase hooks (`on_request` and
---`on_request_body`), so `response()` returns nil until a response is
---available.
---@class praxis.filter.http.HttpFilterContext: praxis.util.Object
---@field private _request praxis.filter.http.HttpRequest The request.
---@field private _response? praxis.filter.http.HttpResponse The response, or nil before one exists.
local HttpFilterContext = setmetatable({}, { __index = Object })
HttpFilterContext.__index = HttpFilterContext

---Creates a new `praxis.filter.http.HttpFilterContext`.
---@param request praxis.filter.http.HttpRequest The request.
---@param response? praxis.filter.http.HttpResponse The response, or nil when none exists yet.
---@return praxis.filter.http.HttpFilterContext The new context.
function HttpFilterContext.new(request, response)
  local self = setmetatable(Object.new(), HttpFilterContext)
  ---@cast self praxis.filter.http.HttpFilterContext
  self._request = request
  self._response = response
  return self
end

---Returns the request.
---@return praxis.filter.http.HttpRequest The request.
function HttpFilterContext:request()
  return self._request
end

---Returns the response, or nil when no response exists yet.
---@return praxis.filter.http.HttpResponse? The response, or nil before one exists.
function HttpFilterContext:response()
  return self._response
end

return HttpFilterContext
