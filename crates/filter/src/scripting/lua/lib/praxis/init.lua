-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---praxis — an object-oriented Lua library for writing HTTP filters.
---
---The library targets Lua 5.4, the version the proxy embeds (mlua's `lua54`
---feature). The proxy serves these modules from a custom `package.searchers`
---entry, which also provides the bundled MIT-licensed `inspect` library
---(`inspect.lua` 3.1.0). Outside the proxy, `inspect` must be available on
---`package.path` (for example via `luarocks install inspect`) because
---`praxis.util.inspect` requires it.
---
---Requiring this module returns the root `praxis` namespace, from which every
---class in the library can be reached:
---
---    local praxis = require("praxis")
---    local uri = praxis.net.Uri.new("https", "/", "")
---
---Inside the proxy, `praxis` is also set as a global before the filter script
---runs, so scripts may use it without requiring it.
local praxis = {}

---Namespace containing general-purpose utilities (`praxis.util.Object` and
---`praxis.util.inspect`).
praxis.util = require("praxis.util")

---Namespace containing logging classes and functions (`praxis.log.Log`,
---`praxis.log.info`, and so on).
praxis.log = require("praxis.log")

---Namespace containing network-related classes (`praxis.net.Uri`).
praxis.net = require("praxis.net")

---Namespace containing filter classes (`praxis.filter.FilterAction`) and HTTP
---filter classes (`praxis.filter.http.HttpBody`,
---`praxis.filter.http.HttpMessage`, `praxis.filter.http.HttpRequest`,
---`praxis.filter.http.HttpResponse`, `praxis.filter.http.HttpContext` and
---`praxis.filter.http.HttpFilter`).
praxis.filter = require("praxis.filter")

return praxis
