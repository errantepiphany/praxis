// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Functional integration tests for Lua scripting example configurations.

#[cfg(feature = "scripting")]
use std::{
    collections::HashMap,
    io::{Read as _, Write as _},
    net::{TcpListener, TcpStream},
    time::Duration,
};

#[cfg(feature = "scripting")]
use praxis_test_utils::{
    free_port, http_send, parse_body, parse_status, start_backend, start_full_proxy, start_proxy, wait_for_http,
    wait_for_tcp,
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
    let backend_port = start_backend("OK");
    let config = load_example_config(
        "scripting/lua_http.yaml",
        proxy_port,
        HashMap::from([("127.0.0.1:3000", backend_port)]),
    );
    let proxy = start_proxy(&config);
    wait_for_http(&format!("127.0.0.1:{proxy_port}"));

    let raw = http_send(
        proxy.addr(),
        "GET / HTTP/1.1\r\n\
         Host: localhost\r\n\
         Connection: close\r\n\r\n",
    );
    // The script's hooks run for the request and response; a Lua error fails
    // the request, so a 200 here proves the context handles carried data.
    assert_eq!(parse_status(&raw), 200, "success should return 200");
    assert_eq!(
        parse_body(&raw),
        "OK",
        "response should come from the backend through the Lua HTTP filter"
    );
}

#[cfg(feature = "scripting")]
#[test]
fn lua_tcp_example() {
    let proxy_port = free_port();
    let echo_port = start_tcp_echo_backend();
    let config = load_example_config(
        "scripting/lua_tcp.yaml",
        proxy_port,
        HashMap::from([("127.0.0.1:2323", proxy_port), ("127.0.0.1:12323", echo_port)]),
    );
    let proxy = start_full_proxy(&config);
    wait_for_tcp(proxy.addr());

    let mut stream = TcpStream::connect(proxy.addr()).expect("connect to proxy");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set read timeout");
    stream.write_all(b"ping").expect("write request");
    let mut buf = [0_u8; 4];
    stream.read_exact(&mut buf).expect("read echoed bytes");
    assert_eq!(&buf, b"ping", "the Lua TCP filter should pass the connection through");
}

// -----------------------------------------------------------------------------
// Test Utilities
// -----------------------------------------------------------------------------

/// Starts a raw TCP echo backend and returns its port.
///
/// Accepts repeatedly so proxy readiness probes (which connect and close) do
/// not consume the backend's single connection.
#[cfg(feature = "scripting")]
fn start_tcp_echo_backend() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind echo backend");
    let port = listener.local_addr().expect("echo backend address").port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            std::thread::spawn(move || {
                let mut buf = [0_u8; 1024];
                while let Ok(read) = stream.read(&mut buf) {
                    if read == 0 || stream.write_all(&buf[..read]).is_err() {
                        break;
                    }
                }
            });
        }
    });
    port
}
