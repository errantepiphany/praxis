-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

local log = require("praxis.log.Log").new("praxis.filter.http.BodyMode")

---The kind of body delivery mode, named after the corresponding Rust
---`BodyMode` variant.
---@alias praxis.filter.http.BodyModeKind
---| "stream"
---| "stream_buffer"
---| "size_limit"

---Controls how body chunks are delivered to a filter.
---
---This is the Lua counterpart of the Rust `BodyMode` enum. `Stream` delivers
---chunks as they arrive; `StreamBuffer` accumulates them and defers upstream
---forwarding until the filter returns `release()` or end-of-stream is reached;
---`SizeLimit` streams chunks through while enforcing a byte ceiling. For the
---two buffering variants, `max_bytes` is nil when the configured buffering cap
---applies.
---
---    local BodyMode = require("praxis.filter.http.BodyMode")
---
---    local mode = BodyMode.stream_buffer(1024)
---    assert(mode:kind() == BodyMode.STREAM_BUFFER)
---    assert(mode:max_bytes() == 1024)
---@class praxis.filter.http.BodyMode: praxis.util.Object
---@field private _kind praxis.filter.http.BodyModeKind The variant kind.
---@field private _max_bytes? integer The byte ceiling or buffering limit; nil for `stream` and unbounded `stream_buffer`.
local BodyMode = setmetatable({}, { __index = Object })
BodyMode.__index = BodyMode

---The `Stream` variant: deliver chunks as they arrive.
---@type praxis.filter.http.BodyModeKind
BodyMode.STREAM = "stream"

---The `StreamBuffer` variant: accumulate chunks and defer forwarding.
---@type praxis.filter.http.BodyModeKind
BodyMode.STREAM_BUFFER = "stream_buffer"

---The `SizeLimit` variant: stream chunks through while enforcing a byte ceiling.
---@type praxis.filter.http.BodyModeKind
BodyMode.SIZE_LIMIT = "size_limit"

---Creates a new `praxis.filter.http.BodyMode`.
---
---`size_limit` requires a non-negative integer `max_bytes`; `stream` ignores
---it; `stream_buffer` accepts nil to inherit the configured buffering cap.
---@param kind praxis.filter.http.BodyModeKind The variant kind.
---@param max_bytes? integer The byte ceiling or buffering limit; required for `size_limit`.
---@return praxis.filter.http.BodyMode The new body mode.
function BodyMode.new(kind, max_bytes)
  local self = setmetatable(Object.new(), BodyMode)
  ---@cast self praxis.filter.http.BodyMode
  self._kind = kind
  if max_bytes ~= nil and (math.type(max_bytes) ~= "integer" or max_bytes < 0) then
    local emsg = string.format(
      "max_bytes must be a non-negative integer but received type: %s, math.type: %s",
      type(max_bytes),
      math.type(max_bytes)
    )
    log:error(emsg)
    error(emsg)
  end
  if kind == BodyMode.SIZE_LIMIT and max_bytes == nil then
    local emsg = string.format("size_limit requires a non-negative integer max_bytes")
    log:error(emsg)
    error(emsg)
  end
  self._max_bytes = max_bytes
  return self
end

---Creates a new `Stream` body mode.
---@return praxis.filter.http.BodyMode The new body mode.
function BodyMode.stream()
  return BodyMode.new(BodyMode.STREAM)
end

---Creates a new `StreamBuffer` body mode.
---@param max_bytes? integer The buffering limit; nil inherits the configured cap.
---@return praxis.filter.http.BodyMode The new body mode.
function BodyMode.stream_buffer(max_bytes)
  return BodyMode.new(BodyMode.STREAM_BUFFER, max_bytes)
end

---Creates a new `SizeLimit` body mode.
---@param max_bytes integer The maximum body size in bytes.
---@return praxis.filter.http.BodyMode The new body mode.
function BodyMode.size_limit(max_bytes)
  return BodyMode.new(BodyMode.SIZE_LIMIT, max_bytes)
end

---Returns the body mode's variant kind.
---@return praxis.filter.http.BodyModeKind The variant kind.
function BodyMode:kind()
  return self._kind
end

---Returns the byte ceiling or buffering limit, or nil when none applies.
---@return integer? The byte limit; nil for `stream` and unbounded `stream_buffer`.
function BodyMode:max_bytes()
  return self._max_bytes
end

return BodyMode
