-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---The `praxis.filter.tcp` namespace: classes for writing TCP filters.
---@class praxis.filter.tcp
---@field TcpFilter praxis.filter.tcp.TcpFilter
---@field TcpFilterContext praxis.filter.tcp.TcpFilterContext
---@field TcpLoadSelection praxis.filter.tcp.TcpLoadSelection
---@field private _filter? praxis.filter.tcp.TcpFilter The registered filter.
local tcp = {}

---Base class for TCP filters.
tcp.TcpFilter = require("praxis.filter.tcp.TcpFilter")

---Per-connection state passed to TCP filter hooks.
tcp.TcpFilterContext = require("praxis.filter.tcp.TcpFilterContext")

---The load-balancer selection for a TCP connection.
tcp.TcpLoadSelection = require("praxis.filter.tcp.TcpLoadSelection")

---Sets the TCP filter to use. Passing nil clears it; the proxy then
---substitutes an empty filter that defines no hooks.
---@param filter? praxis.filter.tcp.TcpFilter The TCP filter to use.
---@return nil
tcp.set = function(filter)
  tcp._filter = filter
end

-- Set a default TCP filter.
tcp.set(tcp.TcpFilter.new())

---Gets the TCP filter to use.
---@return praxis.filter.tcp.TcpFilter? The TCP filter to use, or nil after `set(nil)`.
tcp.get = function()
  return tcp._filter
end

return tcp
