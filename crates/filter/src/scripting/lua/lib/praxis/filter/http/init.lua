-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---The `praxis.filter.http` namespace: classes for writing HTTP filters.
local http = {}

---An HTTP message body: raw bytes plus UTF-8 detection.
http.HttpBody = require("praxis.filter.http.HttpBody")

---An HTTP message: a headers table plus a body.
http.HttpMessage = require("praxis.filter.http.HttpMessage")

---An HTTP request: a message plus method and URI.
http.HttpRequest = require("praxis.filter.http.HttpRequest")

---An HTTP response: a message plus a status code.
http.HttpResponse = require("praxis.filter.http.HttpResponse")

---The filter context: the request and, once available, the response.
http.HttpContext = require("praxis.filter.http.HttpContext")

---Base class for HTTP filters.
http.HttpFilter = require("praxis.filter.http.HttpFilter")

---Sets the HTTP filter to use.
---@param filter praxis.filter.http.HttpFilter The HTTP filter to use.
---@return nil
http.set = function(filter)
  http._filter = filter
end

---Gets the HTTP filter to use.
---@return praxis.filter.http.HttpFilter The HTTP filter to use.
http.get = function()
  return http._filter
end

return http
