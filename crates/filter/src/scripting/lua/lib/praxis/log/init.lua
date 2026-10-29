-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---The `praxis.log` namespace: logging functions backed by a shared default
---logger.
---
---Each function delegates to `default_log`, a module-level `praxis.log.Log`
---named `"default"`, so scripts can log without creating a logger:
---
---    praxis.log.info("handled request for %s", path)
---
---The proxy wires `Log.error_impl`, `Log.warn_impl` and the other level impls
---to its Rust tracing macros when it loads the library, so messages logged here
---reach the proxy's log.
local log = {}

---A logger that formats messages and delegates each level to a class-level impl.
log.Log = require("praxis.log.Log")

---The shared default logger used by the module-level functions.
---@type praxis.log.Log
local default_log = log.Log.new()

---Renders `o` as a flat, single-line string using the default logger.
---@param o any The value to render.
---@return string The flat string.
function log.inspect(o)
  return default_log:inspect(o)
end

---Logs a message at error level through the default logger.
---@param o any A `string.format` format string, or any value to inspect.
---@param ... any Format arguments; used only when `o` is a string.
---@return nil
function log.error(o, ...)
  default_log:error(o, ...)
end

---Logs a message at warn level through the default logger.
---@param o any A `string.format` format string, or any value to inspect.
---@param ... any Format arguments; used only when `o` is a string.
---@return nil
function log.warn(o, ...)
  default_log:warn(o, ...)
end

---Logs a message at info level through the default logger.
---@param o any A `string.format` format string, or any value to inspect.
---@param ... any Format arguments; used only when `o` is a string.
---@return nil
function log.info(o, ...)
  default_log:info(o, ...)
end

---Logs a message at debug level through the default logger.
---@param o any A `string.format` format string, or any value to inspect.
---@param ... any Format arguments; used only when `o` is a string.
---@return nil
function log.debug(o, ...)
  default_log:debug(o, ...)
end

---Logs a message at trace level through the default logger.
---@param o any A `string.format` format string, or any value to inspect.
---@param ... any Format arguments; used only when `o` is a string.
---@return nil
function log.trace(o, ...)
  default_log:trace(o, ...)
end

return log
