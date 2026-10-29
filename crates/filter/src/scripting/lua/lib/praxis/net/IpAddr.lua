-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

local Object = require("praxis.util.Object")

local log = require("praxis.log.Log").new("praxis.net.IpAddr")

---The IP address version.
---@alias praxis.net.IpVersion
---| "v4"
---| "v6"

---An IP address, either IPv4 or IPv6.
---
---This is a simplified Lua counterpart of the Rust `IpAddr` enum. It stores
---the canonical textual form the host supplies and exposes the address version
---as the variant discriminator.
---
---    local IpAddr = require("praxis.net.IpAddr")
---
---    local addr = IpAddr.new("192.0.2.1")
---    assert(addr:version() == "v4")
---@class praxis.net.IpAddr: praxis.util.Object
---@field private _address string The canonical textual address.
local IpAddr = setmetatable({}, { __index = Object })
IpAddr.__index = IpAddr

---Creates a new `praxis.net.IpAddr`.
---@param address string The canonical textual address (IPv4 or IPv6).
---@return praxis.net.IpAddr The new address.
function IpAddr.new(address)
  local self = setmetatable(Object.new(), IpAddr)
  ---@cast self praxis.net.IpAddr
  if type(address) ~= "string" then
    local emsg = string.format("address must be a string but received %s", type(address))
    log:error(emsg)
    error(emsg)
  end
  self._address = address
  return self
end

---Returns the canonical textual address.
---@return string The textual address.
function IpAddr:address()
  return self._address
end

---Returns the address version.
---
---IPv6 addresses always contain a colon; IPv4 addresses never do.
---@return praxis.net.IpVersion The address version.
function IpAddr:version()
  return self._address:find(":", 1, true) and "v6" or "v4"
end

---Returns whether this is an IPv4 address.
---@return boolean Whether this is an IPv4 address.
function IpAddr:is_v4()
  return self:version() == "v4"
end

---Returns whether this is an IPv6 address.
---@return boolean Whether this is an IPv6 address.
function IpAddr:is_v6()
  return self:version() == "v6"
end

return IpAddr
