-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---The `praxis.util` namespace: general-purpose utilities and the root class of
---the praxis class hierarchy.
---
---`praxis.util.inspect` renders Lua values as strings using the `inspect`
---library (bundled with the proxy as `inspect.lua` 3.1.0).
---`praxis.util.Object` is the root class every other praxis class inherits
---from.
local util = {}

---Options accepted by `inspect.inspect` in `inspect.lua` 3.1.0.
---
---`indent` and `newline` control formatting; passing `""` for both produces a
---single flat line. `depth` limits recursion. `process` transforms each key
---and value before rendering; returning nil omits that item from the output.
---@alias praxis.util.InspectOptions
---| { depth?: integer, newline?: string, indent?: string, process?: fun(item: any, path: string): any }

---The root class of the praxis class hierarchy.
util.Object = require("praxis.util.Object")

---Renders `root` as a string using the bundled `inspect` library.
---
---This is the function behind `praxis.log.Log:inspect` and
---`praxis.util.Object:to_string`, both of which pass `{ indent = "",
---newline = "" }` to produce a flat, single-line string.
---@param root any The value to render.
---@param options? praxis.util.InspectOptions Formatting options; defaults to the library's own defaults (two-space indents, newline separators).
---@return string The rendered value, or `tostring(root)` if the `inspect` module resolves to a falsy value.
function util.inspect(root, options)
  local inspect_lib = require("inspect")
  return inspect_lib and inspect_lib.inspect(root, options) or tostring(root)
end

return util
