// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Functional integration tests for Lua scripting example configurations.

#[cfg(feature = "scripting")]
use std::collections::HashMap;

#[cfg(feature = "scripting")]
use praxis_test_utils::{
    free_port, http_send, parse_body, parse_status, start_full_proxy, start_proxy, wait_for_http, wait_for_tcp,
};

#[cfg(feature = "scripting")]
use super::load_example_config;

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(feature = "scripting")]
#[test]
fn lua_http_example() {
    let proxy_port = free_port();
    let config = load_example_config("scripting/lua_http.yaml", proxy_port, HashMap::new());
    let proxy = start_proxy(&config);
    wait_for_http(&format!("127.0.0.1:{proxy_port}"));

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

#[cfg(feature = "scripting")]
#[test]
fn lua_tcp_example() {
    let proxy_port = free_port();
    let config = load_example_config("scripting/lua_tcp.yaml", proxy_port, HashMap::new());
    let proxy = start_full_proxy(&config);
    wait_for_tcp(&format!("127.0.0.1:{proxy_port}"));
    // FIXME: make this a real test
    assert_eq!(proxy.addr(), "127.0.0.1:2323");
}
