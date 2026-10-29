-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---The `praxis.net` namespace: network-related classes.
---@class praxis.net
---@field Uri praxis.net.Uri
local net = {}

---A URI split into its scheme, authority, path and query parts.
net.Uri = require("praxis.net.Uri")

return net
