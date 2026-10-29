-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---The `praxis.net` namespace: network-related classes.
---@class praxis.net
---@field IpAddr praxis.net.IpAddr
---@field Uri praxis.net.Uri
local net = {}

---An IP address, either IPv4 or IPv6.
net.IpAddr = require("praxis.net.IpAddr")

---A URI split into its scheme, host, port, path and query parts.
net.Uri = require("praxis.net.Uri")

return net
