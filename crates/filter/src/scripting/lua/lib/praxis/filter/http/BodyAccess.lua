-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---The kind of body access a filter declares, named after the corresponding Rust
---`BodyAccess` variant.
---@alias praxis.filter.http.BodyAccessKind
---| "none"
---| "read_only"
---| "read_write"

---Whether a filter may skip declared body processing for an admitted request,
---named after the corresponding Rust `BodyRequirement` variant.
---@alias praxis.filter.http.BodyRequirement
---| "optional"
---| "required"

---Declares body access rights and whether that processing is required.
---
---This is the Lua counterpart of the Rust `BodyAccess` enum: the `kind` selects
---the variant and, for the two reading variants, `requirement` says whether the
---pipeline may skip this processing for an admitted request.
---
---    local BodyAccess = require("praxis.filter.http.BodyAccess")
---
---    local access = BodyAccess.read_only(BodyAccess.REQUIRED)
---    assert(access:kind() == BodyAccess.READ_ONLY)
---    assert(access:requirement() == BodyAccess.REQUIRED)
---@class praxis.filter.http.BodyAccess: praxis.util.Object
---@field private _kind praxis.filter.http.BodyAccessKind The variant kind.
---@field private _requirement? praxis.filter.http.BodyRequirement The processing requirement; nil for `none`.
local BodyAccess = setmetatable({}, { __index = Object })
BodyAccess.__index = BodyAccess

---The `None` variant: no body access needed.
---@type praxis.filter.http.BodyAccessKind
BodyAccess.NONE = "none"

---The `ReadOnly` variant: read-only access with an explicit requirement.
---@type praxis.filter.http.BodyAccessKind
BodyAccess.READ_ONLY = "read_only"

---The `ReadWrite` variant: read-write access with an explicit requirement.
---@type praxis.filter.http.BodyAccessKind
BodyAccess.READ_WRITE = "read_write"

---The `Optional` requirement: the filter can safely miss the body for an
---admitted request.
---@type praxis.filter.http.BodyRequirement
BodyAccess.OPTIONAL = "optional"

---The `Required` requirement: the request must not be admitted when this
---processing would be skipped.
---@type praxis.filter.http.BodyRequirement
BodyAccess.REQUIRED = "required"

---Creates a new `praxis.filter.http.BodyAccess`.
---
---A `requirement` is only meaningful for `read_only` and `read_write`; for
---`none` it is ignored and the accessor returns nil. A missing requirement
---defaults to `optional`.
---@param kind praxis.filter.http.BodyAccessKind The variant kind.
---@param requirement? praxis.filter.http.BodyRequirement The processing requirement; defaults to `optional` for the reading variants.
---@return praxis.filter.http.BodyAccess The new body access.
function BodyAccess.new(kind, requirement)
  local self = setmetatable(Object.new(), BodyAccess)
  ---@cast self praxis.filter.http.BodyAccess
  self._kind = kind
  if kind == BodyAccess.NONE then
    self._requirement = nil
  else
    self._requirement = requirement or BodyAccess.OPTIONAL
  end
  return self
end

---Creates a new `None` body access.
---@return praxis.filter.http.BodyAccess The new body access.
function BodyAccess.none()
  return BodyAccess.new(BodyAccess.NONE)
end

---Creates a new `ReadOnly` body access.
---@param requirement? praxis.filter.http.BodyRequirement The processing requirement; defaults to `optional`.
---@return praxis.filter.http.BodyAccess The new body access.
function BodyAccess.read_only(requirement)
  return BodyAccess.new(BodyAccess.READ_ONLY, requirement)
end

---Creates a new `ReadWrite` body access.
---@param requirement? praxis.filter.http.BodyRequirement The processing requirement; defaults to `optional`.
---@return praxis.filter.http.BodyAccess The new body access.
function BodyAccess.read_write(requirement)
  return BodyAccess.new(BodyAccess.READ_WRITE, requirement)
end

---Returns the body access's variant kind.
---@return praxis.filter.http.BodyAccessKind The variant kind.
function BodyAccess:kind()
  return self._kind
end

---Returns the processing requirement, or nil for `none`.
---@return praxis.filter.http.BodyRequirement? The processing requirement; nil for `none`.
function BodyAccess:requirement()
  return self._requirement
end

return BodyAccess
