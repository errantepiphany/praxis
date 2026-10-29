-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

---A logging impl: a function that performs the actual logging for a single log
---level. It receives the logger's name and the already-formatted message
---(`print`, for example, is a valid impl).
---@alias praxis.log.LogImpl fun(log_name: string, log_msg: string)

---A logger that formats messages and hands them to a per-level impl.
---
---Each level's method (`Log:error`, `Log:warn`, `Log:info`, `Log:debug`,
---`Log:trace`) formats its message and calls the matching impl. The impls are
---class-level fields on `Log` (`error_impl`, `warn_impl`, `info_impl`,
---`debug_impl`, `trace_impl`), so setting one affects every `Log` instance.
---When an impl is not set, the level falls back to `print_impl`, which prints a
---timestamped line to standard output.
---
---The proxy sets the impls to its Rust tracing macros when it loads the
---library, so messages reach the proxy's log instead of standard output.
---@class praxis.log.Log: praxis.util.Object
---@field error_impl praxis.log.LogImpl? Class-level error impl overriding the default.
---@field warn_impl praxis.log.LogImpl? Class-level warn impl overriding the default.
---@field info_impl praxis.log.LogImpl? Class-level info impl overriding the default.
---@field debug_impl praxis.log.LogImpl? Class-level debug impl overriding the default.
---@field trace_impl praxis.log.LogImpl? Class-level trace impl overriding the default.
---@field private _name string The logger name included in every message.
local Log = setmetatable({}, { __index = Object })
Log.__index = Log

---The default impl: prints a timestamped, single-line message to standard
---output.
---@param log_level string The level label (`"ERROR"`, `"WARN"`, and so on).
---@param log_name string The logger name.
---@param log_msg string The formatted message.
local function print_impl(log_level, log_name, log_msg)
  print(os.date("%Y-%m-%d %H:%M:%S"), log_level, "praxis.log.Log", log_msg, string.format('log="%s"', log_name))
end

---Creates a new `praxis.log.Log`.
---@param name? string The logger name; defaults to `"default"`.
---@return praxis.log.Log The new logger.
function Log.new(name)
  local self = setmetatable(Object.new(), Log)
  ---@cast self praxis.log.Log
  self._name = name or "default"
  return self
end

---Returns a flat, single-line string representation of `o` (no indents or
---newlines), rendered by `praxis.util.inspect`.
---@param o any The value to render.
---@return string The flat string.
function Log:inspect(o)
  return require("praxis.util").inspect(o, { indent = "", newline = "" })
end

---Logs a message at error level.
---
---When `o` is a string it is treated as a `string.format` format string applied
---to the extra arguments; any other value is rendered with `Log:inspect`. Calls
---the class-level `Log.error_impl` when set, otherwise `print_impl`.
---@param o any A `string.format` format string, or any value to inspect.
---@param ... any Format arguments; used only when `o` is a string.
---@return nil
function Log:error(o, ...)
  local msg = type(o) == "string" and string.format(o, ...) or self:inspect(o)
  if Log.error_impl then
    Log.error_impl(self._name, msg)
  else
    print_impl("ERROR", self._name, msg)
  end
end

---Logs a message at warn level.
---
---When `o` is a string it is treated as a `string.format` format string applied
---to the extra arguments; any other value is rendered with `Log:inspect`. Calls
---the class-level `Log.warn_impl` when set, otherwise `print_impl`.
---@param o any A `string.format` format string, or any value to inspect.
---@param ... any Format arguments; used only when `o` is a string.
---@return nil
function Log:warn(o, ...)
  local msg = type(o) == "string" and string.format(o, ...) or self:inspect(o)
  if Log.warn_impl then
    Log.warn_impl(self._name, msg)
  else
    print_impl("WARN", self._name, msg)
  end
end

---Logs a message at info level.
---
---When `o` is a string it is treated as a `string.format` format string applied
---to the extra arguments; any other value is rendered with `Log:inspect`. Calls
---the class-level `Log.info_impl` when set, otherwise `print_impl`.
---@param o any A `string.format` format string, or any value to inspect.
---@param ... any Format arguments; used only when `o` is a string.
---@return nil
function Log:info(o, ...)
  local msg = type(o) == "string" and string.format(o, ...) or self:inspect(o)
  if Log.info_impl then
    Log.info_impl(self._name, msg)
  else
    print_impl("INFO", self._name, msg)
  end
end

---Logs a message at debug level.
---
---When `o` is a string it is treated as a `string.format` format string applied
---to the extra arguments; any other value is rendered with `Log:inspect`. Calls
---the class-level `Log.debug_impl` when set, otherwise `print_impl`.
---@param o any A `string.format` format string, or any value to inspect.
---@param ... any Format arguments; used only when `o` is a string.
---@return nil
function Log:debug(o, ...)
  local msg = type(o) == "string" and string.format(o, ...) or self:inspect(o)
  if Log.debug_impl then
    Log.debug_impl(self._name, msg)
  else
    print_impl("DEBUG", self._name, msg)
  end
end

---Logs a message at trace level.
---
---When `o` is a string it is treated as a `string.format` format string applied
---to the extra arguments; any other value is rendered with `Log:inspect`. Calls
---the class-level `Log.trace_impl` when set, otherwise `print_impl`.
---@param o any A `string.format` format string, or any value to inspect.
---@param ... any Format arguments; used only when `o` is a string.
---@return nil
function Log:trace(o, ...)
  local msg = type(o) == "string" and string.format(o, ...) or self:inspect(o)
  if Log.trace_impl then
    Log.trace_impl(self._name, msg)
  else
    print_impl("TRACE", self._name, msg)
  end
end

return Log
