// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Functional integration tests for scripting example configurations.

use std::collections::HashMap;

use praxis_test_utils::{free_port, http_send, parse_body, parse_status, start_proxy};

use super::load_example_config;

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(feature = "lua-filter")]
#[test]
fn simple_lua_example() {
    let proxy_port = free_port();
    let config = load_example_config("scripting/lua.yaml", proxy_port, HashMap::new());
    let proxy = start_proxy(&config);

    let raw = http_send(
        proxy.addr(),
        "GET / HTTP/1.1\r\n\
         Host: localhost\r\n\
         Connection: close\r\n\r\n",
    );
    // The script asserts on the request and response handle contents
    // (method, path, status); a Lua error fails the request, so a 200
    // here proves the handles carried correct data.
    assert_eq!(parse_status(&raw), 200, "success should return 200");
    assert_eq!(
        parse_body(&raw),
        "OK",
        "response should come from the static_response filter"
    );
}
