-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---The `praxis.filter` namespace: filter-related classes.
local filter = {}

---The result of a filter's request or response processing.
filter.FilterAction = require("praxis.filter.FilterAction")

---Namespace containing HTTP filter classes (`praxis.filter.http.HttpBody`,
---`praxis.filter.http.HttpMessage`, `praxis.filter.http.HttpRequest`,
---`praxis.filter.http.HttpResponse`, `praxis.filter.http.HttpContext` and
---`praxis.filter.http.HttpFilter`).
filter.http = require("praxis.filter.http")

return filter
