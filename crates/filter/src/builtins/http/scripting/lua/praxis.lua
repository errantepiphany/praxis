-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

-- A helper API for user Lua scripts, in conjunction with
-- request/response handles provided by `../lua/filter.rs`.

-- ======================================
-- Log
-- ======================================

local Log = {}
Log.__index = Log

function Log.new(error_rs, warn_rs, info_rs, debug_rs, trace_rs)
  local log = setmetatable({}, Log)
  log._error_rs = error_rs
  log._warn_rs = warn_rs
  log._info_rs = info_rs
  log._debug_rs = debug_rs
  log._trace_rs = trace_rs
  return log
end

function Log:error(...)
  self._error_rs(string.format(...))
end

function Log:warn(...)
  self._warn_rs(string.format(...))
end

function Log:info(...)
  self._info_rs(string.format(...))
end

function Log:debug(...)
  self._debug_rs(string.format(...))
end

function Log:trace(...)
  self._trace_rs(string.format(...))
end

-- ======================================
-- Util
-- ======================================

local Util = {}
Util.__index = Util

function Util.new(inspect)
  local util = setmetatable({}, Util)
  util._inspect = inspect
  return util
end

function Util:inspect(root, options)
  options = options or {}
  options.newline = options.newline or ""
  options.indent = options.indent or ""
  return self._inspect(root, options)
end

-- ======================================
-- Filter
-- ======================================

local Filter = {}
Filter.__index = Filter

function Filter.new()
  local filter = setmetatable({}, Filter)
  return filter
end

function Filter:on_request(on_request)
  self.on_request = on_request
end

function Filter:on_request_body(on_request_body)
  self.on_request_body = on_request_body
end

function Filter:on_response(on_response)
  self.on_response = on_response
end

function Filter:on_response_body(on_response_body)
  self.on_response_body = on_response_body
end

-- ======================================
-- Praxis
-- ======================================

local Praxis = {}
Praxis.__index = Praxis

function Praxis.new(log, util)
  local praxis = setmetatable({}, Praxis)
  praxis.log = log
  praxis.util = util
  praxis.filter = Filter.new()
  return praxis
end

return {
  Log = Log,
  Util = Util,
  Praxis = Praxis,
}
