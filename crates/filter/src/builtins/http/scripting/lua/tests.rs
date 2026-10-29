// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Unit tests for the lua filter.

use super::filter::LuaFilter;

// -----------------------------------------------------------------------------
// Config Validation
// -----------------------------------------------------------------------------

#[test]
fn from_config_parses_valid_inline() {
    let yaml = yaml(
        "
script: |
  -- comments are okay
  print('Hello World')
",
    );
    let filter = LuaFilter::from_config(&yaml).unwrap();
    assert_eq!(filter.name(), "lua");
}

// -----------------------------------------------------------------------------
// Test Utilities
// -----------------------------------------------------------------------------

/// Parse a YAML string into a `serde_yaml::Value`.
fn yaml(s: &str) -> serde_yaml::Value {
    serde_yaml::from_str(s).expect("test YAML should parse")
}
