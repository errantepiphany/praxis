-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---The variant of a `praxis.filter.FilterAction`, named after the corresponding
---Rust `FilterAction` variant.
---@alias praxis.filter.FilterActionKind
---| "continue"
---| "reject"
---| "terminal_response"
---| "streaming_terminal_response"
---| "release"
---| "body_done"

---The result of a filter's request or response processing.
---
---This is the Lua counterpart of the Rust `FilterAction` enum: a tagged union
---whose `kind` selects one of the variants and whose `value` carries that
---variant's payload, if the variant has one. Payloads are opaque to this class;
---the host defines and interprets them.
---
---`CONTINUE` and `REJECT` are meaningful in the request- and response-head
---phases. `TERMINAL_RESPONSE` and `STREAMING_TERMINAL_RESPONSE` are valid only
---in request-phase filters. `RELEASE` and `BODY_DONE` are meaningful only while
---processing body chunks; outside body processing each behaves as `CONTINUE`.
---
---    local FilterAction = require("praxis.filter.FilterAction")
---
---    local action = FilterAction.reject(rejection)
---    assert(action:kind() == FilterAction.REJECT)
---    assert(action:value() == rejection)
---@class praxis.filter.FilterAction: praxis.util.Object
---@field private _kind praxis.filter.FilterActionKind The variant kind.
---@field private _value? any The variant payload; nil for variants without one.
local FilterAction = setmetatable({}, { __index = Object })
FilterAction.__index = FilterAction

---The `Continue` variant: continue to the next filter in the pipeline.
---@type praxis.filter.FilterActionKind
FilterAction.CONTINUE = "continue"

---The `Reject` variant: stop processing and respond with the payload, a
---rejection.
---@type praxis.filter.FilterActionKind
FilterAction.REJECT = "reject"

---The `TerminalResponse` variant: return the payload, a complete response, to
---the client and run the response-phase filters.
---@type praxis.filter.FilterActionKind
FilterAction.TERMINAL_RESPONSE = "terminal_response"

---The `StreamingTerminalResponse` variant: return the payload, a streaming
---response, to the client, running the response-phase filters before
---commitment.
---@type praxis.filter.FilterActionKind
FilterAction.STREAMING_TERMINAL_RESPONSE = "streaming_terminal_response"

---The `Release` variant: forward accumulated buffered body data to upstream.
---Outside body processing it behaves as `CONTINUE`.
---@type praxis.filter.FilterActionKind
FilterAction.RELEASE = "release"

---The `BodyDone` variant: skip this filter for the remaining body chunks.
---Outside body processing it behaves as `CONTINUE`.
---@type praxis.filter.FilterActionKind
FilterAction.BODY_DONE = "body_done"

---Creates a new `praxis.filter.FilterAction` with the given variant kind and
---optional payload.
---@param kind praxis.filter.FilterActionKind The variant kind.
---@param value? any The variant payload; omit it for variants without one.
---@return praxis.filter.FilterAction The new action.
function FilterAction.new(kind, value)
  local self = setmetatable(Object.new(), FilterAction)
  ---@cast self praxis.filter.FilterAction
  self._kind = kind
  self._value = value
  return self
end

---Creates a new `Continue` action.
---@return praxis.filter.FilterAction The new action.
function FilterAction.continue()
  return FilterAction.new(FilterAction.CONTINUE)
end

---Creates a new `Reject` action carrying a rejection.
---@param rejection any The host-defined rejection to respond with.
---@return praxis.filter.FilterAction The new action.
function FilterAction.reject(rejection)
  return FilterAction.new(FilterAction.REJECT, rejection)
end

---Creates a new `TerminalResponse` action carrying a complete response.
---@param response any The host-defined complete response to return to the client.
---@return praxis.filter.FilterAction The new action.
function FilterAction.terminal_response(response)
  return FilterAction.new(FilterAction.TERMINAL_RESPONSE, response)
end

---Creates a new `StreamingTerminalResponse` action carrying a streaming
---response.
---@param response any The host-defined streaming response to return to the client.
---@return praxis.filter.FilterAction The new action.
function FilterAction.streaming_terminal_response(response)
  return FilterAction.new(FilterAction.STREAMING_TERMINAL_RESPONSE, response)
end

---Creates a new `Release` action.
---@return praxis.filter.FilterAction The new action.
function FilterAction.release()
  return FilterAction.new(FilterAction.RELEASE)
end

---Creates a new `BodyDone` action.
---@return praxis.filter.FilterAction The new action.
function FilterAction.body_done()
  return FilterAction.new(FilterAction.BODY_DONE)
end

---Returns the action's variant kind.
---@return praxis.filter.FilterActionKind The variant kind.
function FilterAction:kind()
  return self._kind
end

---Returns the payload carried by the action's variant, or nil for a variant
---without a payload.
---@return any The variant payload; nil when the variant has none.
function FilterAction:value()
  return self._value
end

return FilterAction
