-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---The `praxis.filter` namespace: filter-related classes.
---@class praxis.filter
---@field FilterAction praxis.filter.FilterAction
local filter = {}

---The result of a filter's request or response processing.
filter.FilterAction = require("praxis.filter.FilterAction")

---Namespace containing HTTP filter classes (`praxis.filter.http.HttpMessage`,
---`praxis.filter.http.HttpRequest`, `praxis.filter.http.HttpResponse`,
---`praxis.filter.http.HttpFilterContext` and `praxis.filter.http.HttpFilter`).
filter.http = require("praxis.filter.http")

return filter
