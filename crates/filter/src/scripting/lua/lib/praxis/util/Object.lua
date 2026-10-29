-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---The root class of the praxis class hierarchy.
---
---Every other praxis class inherits from `praxis.util.Object`, which provides
---the `new` constructor and a `to_string` representation built on
---`praxis.util.inspect`.
---@class praxis.util.Object
local Object = {}
Object.__index = Object

---Creates a new `praxis.util.Object`.
---@return praxis.util.Object The new object.
function Object.new()
  local self = setmetatable({}, Object)
  return self
end

---Returns a flat, single-line string representation of `self` (no indents or
---newlines), using `praxis.util.inspect`.
---@return string The string representation.
function Object:to_string()
  return require("praxis.util").inspect(self, { indent = "", newline = "" })
end

return Object
