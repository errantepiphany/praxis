// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Deserialized YAML configuration types for the lua filter.

use serde::Deserialize;

/// Deserialized YAML config for the lua filter.
///
/// ```yaml
/// filter: lua
/// script: <lua source code>
/// ```
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LuaConfig {
    /// The script to evaluate.
    pub script: String,
}
