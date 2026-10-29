-- SPDX-License-Identifier: Apache-2.0
-- Copyright (c) 2026 Praxis Contributors

---This file ONLY exists for editors to be able to resolve the library.
---It is NOT embedded or loaded by Rust: `api.rs` in the parent directory
---embeds the other modules in this tree and serves them through a custom
---`package.searchers` loader.
---
---@meta praxis_lib

-- lua_ls resolves the require() and exposes `praxis` with full type inference
-- see <project-root>/.luarc.json > diagnostics.globals: "praxis"
praxis = require("praxis")
