// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Unit tests for the Lua HTTP and TCP filters and their Lua classes.

use std::{borrow::Cow, sync::Arc, time::Instant};

use bytes::Bytes;
use http::{HeaderValue, Method, StatusCode};

use super::{LuaHttpFilter, LuaTcpFilter, api};
use crate::{
    BodyAccess, BodyMode, BodyRequirement, FilterAction, FilterError, Response, TcpLbSelection,
    load_balancing::{round_robin::RoundRobin, strategy::Strategy},
    tcp_filter::TcpFilterContext,
    test_utils::{make_filter_context, make_request},
};

// -----------------------------------------------------------------------------
// Lua Class Behavior
// -----------------------------------------------------------------------------

/// Builds a Lua state with the embedded library loaded and runs `script`.
fn run_lua(script: &str) {
    api::build_lua(script).expect("Lua script should run");
}

#[test]
fn body_access_class() {
    run_lua(
        r#"
local BodyAccess = praxis.filter.http.BodyAccess
local none = BodyAccess.none()
assert(none:kind() == "none")
assert(none:requirement() == nil)
local read_only = BodyAccess.read_only()
assert(read_only:kind() == "read_only")
assert(read_only:requirement() == "optional")
local read_write = BodyAccess.read_write(BodyAccess.REQUIRED)
assert(read_write:kind() == "read_write")
assert(read_write:requirement() == "required")
"#,
    );
}

#[test]
fn body_mode_class() {
    run_lua(
        r#"
local BodyMode = praxis.filter.http.BodyMode
local stream = BodyMode.stream()
assert(stream:kind() == "stream")
assert(stream:max_bytes() == nil)
local buffered = BodyMode.stream_buffer(1024)
assert(buffered:kind() == "stream_buffer")
assert(buffered:max_bytes() == 1024)
local limited = BodyMode.size_limit(2048)
assert(limited:kind() == "size_limit")
assert(limited:max_bytes() == 2048)
assert(not pcall(BodyMode.size_limit))
assert(not pcall(BodyMode.stream_buffer, -1))
"#,
    );
}

#[test]
fn header_map_class() {
    run_lua(
        r#"
local HeaderMap = praxis.filter.http.HeaderMap
local headers = HeaderMap.new({
  ["content-type"] = "text/html",
  ["set-cookie"] = { "a=1", "b=2" },
})
assert(headers:get("Content-Type") == "text/html")
assert(headers:contains("SET-COOKIE"))
local cookies = headers:get_all("set-cookie")
assert(#cookies == 2)
assert(cookies[1] == "a=1")
assert(cookies[2] == "b=2")
assert(#headers:get_all("missing") == 0)
assert(headers:get("missing") == nil)
"#,
    );
}

#[test]
fn ip_addr_class() {
    run_lua(
        r#"
local IpAddr = praxis.net.IpAddr
local v4 = IpAddr.new("192.0.2.1")
assert(v4:address() == "192.0.2.1")
assert(v4:version() == "v4")
assert(v4:is_v4())
assert(not v4:is_v6())
local v6 = IpAddr.new("::1")
assert(v6:version() == "v6")
assert(v6:is_v6())
"#,
    );
}

#[test]
fn uri_query_values() {
    run_lua(
        r#"
local Uri = praxis.net.Uri
local repeated = Uri.new("https", "example.com", nil, "/", "a=1&b=2&a=3")
local values = repeated:query_values()
assert(#values.a == 2)
assert(values.a[1] == "1")
assert(values.a[2] == "3")
assert(#values.b == 1)
assert(values.b[1] == "2")

local encoded = Uri.new("http", nil, nil, "/", "q=hello%20world&tag=a%2Bb&plus=a+b&flag&empty=&x=1%2")
local decoded = encoded:query_values()
assert(decoded.q[1] == "hello world")
assert(decoded.tag[1] == "a+b")
assert(decoded.plus[1] == "a b")
assert(decoded.flag[1] == "")
assert(decoded.empty[1] == "")
assert(decoded.x[1] == "1%2")

assert(next(Uri.new("http", nil, nil, "/", nil):query_values()) == nil)
assert(next(Uri.new("http", nil, nil, "/", ""):query_values()) == nil)
"#,
    );
}

#[test]
#[expect(clippy::too_many_lines, reason = "Lua fixture")]
fn tcp_classes() {
    run_lua(
        r#"
local TcpLoadSelection = praxis.filter.tcp.TcpLoadSelection
local selection = TcpLoadSelection.new("backend", "10.0.0.1:5432")
assert(selection:cluster() == "backend")
assert(selection:endpoint() == "10.0.0.1:5432")
assert(TcpLoadSelection.new("backend"):endpoint() == nil)

local TcpFilterContext = praxis.filter.tcp.TcpFilterContext
local empty = TcpFilterContext.new()
assert(empty:remote_addr() == "")
assert(empty:local_addr() == "")
assert(empty:sni() == nil)
assert(empty:upstream_addr() == nil)
assert(empty:lb_selection() == nil)
assert(empty:cluster() == nil)
assert(empty:connect_time() == 0)
assert(empty:bytes_in() == 0)
assert(empty:bytes_out() == 0)

local TcpFilter = praxis.filter.tcp.TcpFilter
local filter = TcpFilter.new()
local ctx = TcpFilterContext.new({
  remote_addr = "127.0.0.1:1",
  local_addr = "127.0.0.1:2",
  connect_time = 0,
  bytes_in = 0,
  bytes_out = 0,
})
assert(filter:on_connect(ctx):kind() == "continue")
assert(filter:on_disconnect(ctx):kind() == "continue")
assert(praxis.filter.tcp.get() ~= nil)
"#,
    );
}

// -----------------------------------------------------------------------------
// HTTP Config
// -----------------------------------------------------------------------------

#[test]
fn from_config_parses_script_only() {
    let filter = LuaHttpFilter::from_config(&yaml(
        "
script: |
  praxis.filter.http.set(praxis.filter.http.HttpFilter.new())
",
    ))
    .expect("valid config should parse");
    assert_eq!(filter.name(), "lua_http");
}

#[test]
fn from_config_accepts_tagged_script() {
    let filter = LuaHttpFilter::from_config(&yaml(
        "
script: !lua |
  praxis.filter.http.set(praxis.filter.http.HttpFilter.new())
",
    ))
    .expect("tagged script should parse");
    assert_eq!(filter.name(), "lua_http");
}

#[test]
fn from_config_requires_script() {
    assert!(LuaHttpFilter::from_config(&yaml("{}")).is_err());
}

#[test]
fn from_config_rejects_script_errors() {
    assert!(LuaHttpFilter::from_config(&yaml("script: |\n  error('boom')\n")).is_err());
}

#[test]
fn from_config_defaults_body_declarations() {
    let filter = LuaHttpFilter::from_config(&yaml(
        "
script: |
  praxis.filter.http.set(praxis.filter.http.HttpFilter.new())
",
    ))
    .expect("valid config should parse");
    assert_eq!(filter.request_body_access(), BodyAccess::None);
    assert_eq!(filter.request_body_mode(), BodyMode::Stream);
    assert_eq!(filter.response_body_access(), BodyAccess::None);
    assert_eq!(filter.response_body_mode(), BodyMode::Stream);
}

#[test]
fn from_config_reads_body_declarations_from_lua() {
    let filter = LuaHttpFilter::from_config(&yaml(body_config_script())).expect("valid config should parse");
    assert_eq!(
        filter.request_body_access(),
        BodyAccess::ReadOnly(BodyRequirement::Required)
    );
    assert_eq!(
        filter.request_body_mode(),
        BodyMode::StreamBuffer { max_bytes: Some(1024) }
    );
    assert_eq!(
        filter.response_body_access(),
        BodyAccess::ReadWrite(BodyRequirement::Optional)
    );
    assert_eq!(filter.response_body_mode(), BodyMode::SizeLimit { max_bytes: 2048 });
}

#[test]
fn invalid_body_declarations_fall_back_to_defaults() {
    let filter = LuaHttpFilter::from_config(&yaml(invalid_body_config_script())).expect("valid config should parse");
    assert_eq!(filter.request_body_access(), BodyAccess::None);
    assert_eq!(filter.request_body_mode(), BodyMode::Stream);
}

// -----------------------------------------------------------------------------
// HTTP Hook Dispatch
// -----------------------------------------------------------------------------

#[tokio::test]
async fn on_request_calls_script_hook() {
    let script = handler_script("on_request", "error('on_request called: ' .. ctx:request():method())");

    let error = run_on_request(&script)
        .await
        .expect_err("the hook's error should surface as a filter error");
    assert!(
        error.to_string().contains("on_request called: GET"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn on_request_without_hook_continues() {
    let action = run_on_request(plain_filter_script()).await.expect("hook should run");
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn on_request_without_registered_filter_continues() {
    let action = run_on_request("script: |\n  -- no filter registered\n")
        .await
        .expect("hook should run");
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn on_request_maps_actions() {
    for (body, expected) in [
        ("return praxis.filter.FilterAction.continue()", "continue"),
        ("return praxis.filter.FilterAction.release()", "release"),
        ("return praxis.filter.FilterAction.body_done()", "body_done"),
    ] {
        let action = run_on_request(&handler_script("on_request", body))
            .await
            .expect("hook should run");
        match expected {
            "continue" => assert!(matches!(action, FilterAction::Continue)),
            "release" => assert!(matches!(action, FilterAction::Release)),
            _ => assert!(matches!(action, FilterAction::BodyDone)),
        }
    }
}

#[tokio::test]
async fn on_request_maps_reject() {
    let action = run_on_request(&handler_script(
        "on_request",
        "return praxis.filter.FilterAction.reject(403)",
    ))
    .await
    .expect("hook should run");
    assert!(
        matches!(action, FilterAction::Reject(rejection) if rejection.status == 403),
        "reject(403) should map to a 403 rejection"
    );
}

#[tokio::test]
async fn on_request_rejects_invalid_rejections() {
    for (body, expected) in [
        (
            "return praxis.filter.FilterAction.reject('forbidden')",
            "non-integer rejection",
        ),
        (
            "return praxis.filter.FilterAction.reject(700)",
            "invalid HTTP status code 700",
        ),
        (
            "return praxis.filter.FilterAction.new('bogus')",
            "unsupported FilterAction kind",
        ),
    ] {
        let error = run_on_request(&handler_script("on_request", body))
            .await
            .expect_err("invalid action should be an error");
        assert!(error.to_string().contains(expected), "unexpected error: {error}");
    }
}

#[tokio::test]
async fn on_request_uri_derives_scheme_host_and_port() {
    let filter = LuaHttpFilter::from_config(&yaml(&uri_echo_script())).expect("valid config should parse");
    let mut request = make_request(Method::GET, "/path?q=1");
    request
        .headers
        .insert(http::header::HOST, "example.com:8080".parse().expect("host"));
    let mut ctx = make_filter_context(&request);

    let error = filter
        .on_request(&mut ctx)
        .await
        .expect_err("the hook reports the URI parts as an error");
    assert!(
        error.to_string().contains("http example.com 8080 /path?q=1"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn on_request_uri_derives_https_from_downstream_tls() {
    let filter = LuaHttpFilter::from_config(&yaml(&uri_echo_script())).expect("valid config should parse");
    let mut request = make_request(Method::GET, "/");
    request
        .headers
        .insert(http::header::HOST, "example.com".parse().expect("host"));
    let mut ctx = make_filter_context(&request);
    ctx.downstream_tls = true;

    let error = filter
        .on_request(&mut ctx)
        .await
        .expect_err("the hook reports the URI parts as an error");
    assert!(
        error.to_string().contains("https example.com nil /"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn on_request_uri_prefers_absolute_form_authority() {
    let filter = LuaHttpFilter::from_config(&yaml(&uri_echo_script())).expect("valid config should parse");
    let mut request = make_request(Method::GET, "http://absolute.example/path");
    request
        .headers
        .insert(http::header::HOST, "header.example".parse().expect("host"));
    let mut ctx = make_filter_context(&request);

    let error = filter
        .on_request(&mut ctx)
        .await
        .expect_err("the hook reports the URI parts as an error");
    assert!(
        error.to_string().contains("http absolute.example nil /path"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn on_request_uri_splits_ipv6_host_and_port() {
    let filter = LuaHttpFilter::from_config(&yaml(&uri_echo_script())).expect("valid config should parse");
    let mut request = make_request(Method::GET, "/");
    request
        .headers
        .insert(http::header::HOST, "[::1]:8080".parse().expect("host"));
    let mut ctx = make_filter_context(&request);

    let error = filter
        .on_request(&mut ctx)
        .await
        .expect_err("the hook reports the URI parts as an error");
    assert!(
        error.to_string().contains("http [::1] 8080 /?"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn on_request_body_dispatches_and_maps_actions() {
    for (body, expected) in [
        ("return praxis.filter.FilterAction.continue()", "continue"),
        ("return praxis.filter.FilterAction.release()", "release"),
        ("return praxis.filter.FilterAction.body_done()", "body_done"),
    ] {
        let action = run_on_request_body(&handler_script("on_request_body", body))
            .await
            .expect("hook should run");
        match expected {
            "continue" => assert!(matches!(action, FilterAction::Continue)),
            "release" => assert!(matches!(action, FilterAction::Release)),
            _ => assert!(matches!(action, FilterAction::BodyDone)),
        }
    }
}

#[tokio::test]
async fn on_response_dispatches_and_maps_actions() {
    let action = run_on_response(&handler_script(
        "on_response",
        "return praxis.filter.FilterAction.reject(502)",
    ))
    .await
    .expect("hook should run");
    assert!(
        matches!(action, FilterAction::Reject(rejection) if rejection.status == 502),
        "reject(502) should map to a 502 rejection"
    );
    let action = run_on_response(plain_filter_script()).await.expect("hook should run");
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn on_response_body_dispatches_and_maps_actions() {
    let action = run_on_response_body(&handler_script(
        "on_response_body",
        "return praxis.filter.FilterAction.reject(500)",
    ))
    .expect("hook should run");
    assert!(
        matches!(action, FilterAction::Reject(rejection) if rejection.status == 500),
        "reject(500) should map to a 500 rejection"
    );
    let action = run_on_response_body(plain_filter_script()).expect("hook should run");
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
#[expect(clippy::too_many_lines, reason = "Lua fixture")]
async fn bytes_inspection_methods() {
    let script = r"
script: |
  local Filter = praxis.filter.http.HttpFilter
  local Handler = setmetatable({}, { __index = Filter })
  Handler.__index = Handler

  function Handler.new()
    return setmetatable(Filter.new(), Handler)
  end

  function Handler:on_request_body(ctx, body, end_of_stream)
    local Bytes = praxis.util.Bytes
    local empty = Bytes.new('')
    local none = Bytes.new(nil)
    local invalid = Bytes.new('\255')
    local multibyte = Bytes.new('h\195\169llo')
    assert(body:data() == 'chunk', 'body data')
    assert(body:len() == 5, 'body length')
    assert(body:utf8_len() == 5, 'body char length')
    assert(body:is_empty() == false, 'body is not empty')
    assert(body:is_nil() == false, 'body is not nil')
    assert(body:is_utf8() == true, 'body is utf8')
    assert(empty:data() == '', 'empty data')
    assert(empty:len() == 0, 'empty length')
    assert(empty:utf8_len() == 0, 'empty char length')
    assert(empty:is_empty() == true, 'empty is empty')
    assert(empty:is_nil() == false, 'empty is not nil')
    assert(empty:is_utf8() == true, 'empty is utf8')
    assert(none:data() == nil, 'nil data')
    assert(none:len() == -1, 'nil length')
    assert(none:utf8_len() == -1, 'nil char length')
    assert(none:is_empty() == true, 'nil is empty')
    assert(none:is_nil() == true, 'nil is nil')
    assert(none:is_utf8() == false, 'nil is not utf8')
    assert(invalid:is_utf8() == false, 'invalid is not utf8')
    assert(invalid:utf8_len() == -1, 'invalid char length')
    assert(multibyte:len() == 6, 'multibyte byte length')
    assert(multibyte:utf8_len() == 5, 'multibyte char length')
    return praxis.filter.FilterAction.continue()
  end

  praxis.filter.http.set(Handler.new())
";

    let action = run_on_request_body(script).await.expect("hook should run");
    assert!(matches!(action, FilterAction::Continue));
}

// -----------------------------------------------------------------------------
// HTTP Context Fields
// -----------------------------------------------------------------------------

#[tokio::test]
async fn http_context_exposes_all_fields() {
    let filter = LuaHttpFilter::from_config(&yaml(context_script())).expect("valid config should parse");
    let mut request = make_request(Method::POST, "/path?q=1");
    request
        .headers
        .insert(http::header::HOST, "example.com".parse().expect("host"));
    request.headers.append("x-multi", "a".parse().expect("a"));
    request.headers.append("x-multi", "b".parse().expect("b"));
    request
        .headers
        .append("x-binary", HeaderValue::from_bytes(b"\xff").expect("binary"));
    let mut response = Response {
        status: StatusCode::OK,
        headers: http::HeaderMap::new(),
    };
    response.headers.insert("x-resp", "yes".parse().expect("resp"));
    let mut ctx = make_filter_context(&request);
    ctx.response_header = Some(&mut response);
    ctx.buffered_request_body = Some(Bytes::from_static(b"buffered"));
    ctx.client_addr = Some("192.0.2.1".parse().expect("addr"));
    ctx.cluster = Some(Arc::from("backend"));
    ctx.extra_request_headers = vec![(Cow::Borrowed("x-extra"), "1".to_owned())];
    ctx.request_headers_to_remove = vec!["x-remove".parse().expect("remove")];
    ctx.request_headers_to_set = vec![("x-set".parse().expect("set"), "2".parse().expect("value"))];
    ctx.request_body_mode = BodyMode::StreamBuffer { max_bytes: Some(10) };
    ctx.request_body_bytes = 7;
    ctx.response_body_mode = BodyMode::SizeLimit { max_bytes: 20 };
    ctx.response_body_bytes = 9;

    let action = filter.on_response(&mut ctx).await.expect("hook should run");
    assert!(matches!(action, FilterAction::Continue));
}

// -----------------------------------------------------------------------------
// TCP Filter
// -----------------------------------------------------------------------------

#[test]
fn tcp_from_config_requires_script() {
    assert!(LuaTcpFilter::from_config(&yaml("{}")).is_err());
}

#[test]
fn tcp_from_config_parses_script() {
    let filter =
        LuaTcpFilter::from_config(&yaml("script: |\n  -- no filter registered\n")).expect("valid config should parse");
    assert_eq!(filter.name(), "lua_tcp");
}

#[tokio::test]
async fn tcp_on_connect_exposes_context() {
    let filter = LuaTcpFilter::from_config(&yaml(tcp_script())).expect("valid config should parse");
    let mut ctx = make_tcp_context();
    ctx.lb_selection = Some(lb_selection());

    let action = filter.on_connect(&mut ctx).await.expect("hook should run");
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn tcp_on_disconnect_runs_hook() {
    let filter = LuaTcpFilter::from_config(&yaml(tcp_script())).expect("valid config should parse");
    let mut ctx = make_tcp_context();

    filter.on_disconnect(&mut ctx).await.expect("hook should run");
}

#[tokio::test]
async fn tcp_on_connect_without_hook_continues() {
    let filter =
        LuaTcpFilter::from_config(&yaml("script: |\n  -- no filter registered\n")).expect("valid config should parse");
    let mut ctx = make_tcp_context();

    let action = filter.on_connect(&mut ctx).await.expect("hook should run");
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn tcp_on_connect_maps_reject() {
    let script = "
script: |
  local Handler = setmetatable({}, { __index = praxis.filter.tcp.TcpFilter })
  Handler.__index = Handler
  function Handler.new()
    return setmetatable(praxis.filter.tcp.TcpFilter.new(), Handler)
  end
  function Handler:on_connect(ctx)
    return praxis.filter.FilterAction.reject(403)
  end
  praxis.filter.tcp.set(Handler.new())
";
    let filter = LuaTcpFilter::from_config(&yaml(script)).expect("valid config should parse");
    let mut ctx = make_tcp_context();

    let action = filter.on_connect(&mut ctx).await.expect("hook should run");
    assert!(
        matches!(action, FilterAction::Reject(rejection) if rejection.status == 403),
        "reject(403) should map to a 403 rejection"
    );
}

// -----------------------------------------------------------------------------
// Test Scripts
// -----------------------------------------------------------------------------

/// A config script whose filter declares non-default body access and modes.
fn body_config_script() -> &'static str {
    "
script: |
  local Filter = praxis.filter.http.HttpFilter
  local BodyAccess = praxis.filter.http.BodyAccess
  local BodyMode = praxis.filter.http.BodyMode
  local Handler = setmetatable({}, { __index = Filter })
  Handler.__index = Handler

  function Handler.new()
    return setmetatable(Filter.new(), Handler)
  end

  function Handler:request_body_access()
    return BodyAccess.read_only(BodyAccess.REQUIRED)
  end

  function Handler:request_body_mode()
    return BodyMode.stream_buffer(1024)
  end

  function Handler:response_body_access()
    return BodyAccess.read_write()
  end

  function Handler:response_body_mode()
    return BodyMode.size_limit(2048)
  end

  praxis.filter.http.set(Handler.new())
"
}

/// A config script whose filter declares invalid body access and modes.
fn invalid_body_config_script() -> &'static str {
    r#"
script: |
  local Filter = praxis.filter.http.HttpFilter
  local Handler = setmetatable({}, { __index = Filter })
  Handler.__index = Handler

  function Handler.new()
    return setmetatable(Filter.new(), Handler)
  end

  function Handler:request_body_access()
    return praxis.filter.http.BodyAccess.new("bogus")
  end

  function Handler:request_body_mode()
    return praxis.filter.http.BodyMode.new("bogus")
  end

  praxis.filter.http.set(Handler.new())
"#
}

/// A config script whose filter asserts the full HTTP context surface.
#[expect(clippy::too_many_lines, reason = "Lua fixture")]
fn context_script() -> &'static str {
    r#"
script: |
  local Filter = praxis.filter.http.HttpFilter
  local Handler = setmetatable({}, { __index = Filter })
  Handler.__index = Handler

  function Handler.new()
    return setmetatable(Filter.new(), Handler)
  end

  function Handler:on_response(ctx)
    assert(ctx:request():method() == "POST")
    assert(ctx:request():uri():path() == "/path")
    assert(ctx:request():uri():query() == "q=1")
    assert(ctx:request():headers():get("host") == "example.com")
    local multi = ctx:request():headers():get("x-multi")
    assert(type(multi) == "table" and multi[1] == "a" and multi[2] == "b")
    local binary = ctx:request():headers():get("x-binary")
    assert(type(binary) == "string" and #binary == 3)
    assert(ctx:buffered_request_body():data() == "buffered")
    assert(ctx:client_addr():address() == "192.0.2.1")
    assert(ctx:client_addr():version() == "v4")
    assert(ctx:cluster() == "backend")
    assert(ctx:extra_request_headers()[1][1] == "x-extra")
    assert(ctx:extra_request_headers()[1][2] == "1")
    assert(ctx:request_headers_to_remove()[1] == "x-remove")
    assert(ctx:request_headers_to_set()[1][1] == "x-set")
    assert(ctx:request_headers_to_set()[1][2] == "2")
    assert(ctx:request_body_mode():kind() == "stream_buffer")
    assert(ctx:request_body_mode():max_bytes() == 10)
    assert(ctx:request_body_bytes() == 7)
    assert(ctx:response_body_mode():kind() == "size_limit")
    assert(ctx:response_body_mode():max_bytes() == 20)
    assert(ctx:response_body_bytes() == 9)
    assert(ctx:response():status() == 200)
    assert(ctx:response():headers():get("x-resp") == "yes")
    return praxis.filter.FilterAction.continue()
  end

  praxis.filter.http.set(Handler.new())
"#
}

/// A config script whose TCP filter asserts the connection context surface.
fn tcp_script() -> &'static str {
    r#"
script: |
  local TcpFilter = praxis.filter.tcp.TcpFilter
  local Handler = setmetatable({}, { __index = TcpFilter })
  Handler.__index = Handler

  function Handler.new()
    return setmetatable(TcpFilter.new(), Handler)
  end

  function Handler:on_connect(ctx)
    assert(ctx:remote_addr() == "127.0.0.1:12345")
    assert(ctx:local_addr() == "127.0.0.1:2323")
    assert(ctx:sni() == "example.com")
    assert(ctx:upstream_addr() == "10.0.0.1:5432")
    assert(ctx:cluster() == "backend")
    assert(ctx:connect_time() > 0)
    assert(ctx:bytes_in() == 9223372036854775807)
    assert(ctx:bytes_out() == 3)
    if ctx:lb_selection() ~= nil then
      assert(ctx:lb_selection():cluster() == "backend")
      assert(ctx:lb_selection():endpoint() == "10.0.0.1:5432")
    end
    return praxis.filter.FilterAction.continue()
  end

  function Handler:on_disconnect(ctx)
    assert(ctx:remote_addr() == "127.0.0.1:12345")
    return praxis.filter.FilterAction.continue()
  end

  praxis.filter.tcp.set(Handler.new())
"#
}

/// A `lua_http` script that registers an `HttpFilter` subclass whose `hook`
/// runs `body`.
///
/// The body hooks receive `(ctx, body, end_of_stream)`; the head hooks
/// receive only `(ctx)`.
fn handler_script(hook: &str, body: &str) -> String {
    let params = match hook {
        "on_request" | "on_response" => "ctx",
        _ => "ctx, body, end_of_stream",
    };
    format!(
        "
script: |
  local Filter = praxis.filter.http.HttpFilter
  local Handler = setmetatable({{}}, {{ __index = Filter }})
  Handler.__index = Handler

  function Handler.new()
    return setmetatable(Filter.new(), Handler)
  end

  function Handler:{hook}({params})
    {body}
  end

  praxis.filter.http.set(Handler.new())
"
    )
}

/// A `lua_http` script that registers a filter table with no hooks.
fn plain_filter_script() -> &'static str {
    "
script: |
  praxis.filter.http.set({})
"
}

/// An `on_request` script that raises the request URI's scheme, host, port,
/// path and query, so a failing call reports them.
fn uri_echo_script() -> String {
    handler_script(
        "on_request",
        "local uri = ctx:request():uri(); \
         error(uri:scheme() .. ' ' .. tostring(uri:host()) .. ' ' .. tostring(uri:port()) .. ' ' .. uri:path() .. '?' .. tostring(uri:query()))",
    )
}

// -----------------------------------------------------------------------------
// Test Utilities
// -----------------------------------------------------------------------------

/// Builds a filter from `script` and runs its `on_request` hook against a
/// GET `/` request.
async fn run_on_request(script: &str) -> Result<FilterAction, FilterError> {
    let filter = LuaHttpFilter::from_config(&yaml(script)).expect("config should parse");
    let request = make_request(Method::GET, "/");
    let mut ctx = make_filter_context(&request);
    filter.on_request(&mut ctx).await
}

/// Builds a filter from `script` and runs its `on_request_body` hook against
/// a GET `/` request with a one-chunk body.
async fn run_on_request_body(script: &str) -> Result<FilterAction, FilterError> {
    let filter = LuaHttpFilter::from_config(&yaml(script)).expect("config should parse");
    let request = make_request(Method::GET, "/");
    let mut ctx = make_filter_context(&request);
    let mut body = Some(Bytes::from_static(b"chunk"));
    filter.on_request_body(&mut ctx, &mut body, true).await
}

/// Builds a filter from `script` and runs its `on_response` hook against a
/// GET `/` request with no response head set.
async fn run_on_response(script: &str) -> Result<FilterAction, FilterError> {
    let filter = LuaHttpFilter::from_config(&yaml(script)).expect("config should parse");
    let request = make_request(Method::GET, "/");
    let mut ctx = make_filter_context(&request);
    filter.on_response(&mut ctx).await
}

/// Builds a filter from `script` and runs its `on_response_body` hook
/// against a GET `/` request with a one-chunk body.
fn run_on_response_body(script: &str) -> Result<FilterAction, FilterError> {
    let filter = LuaHttpFilter::from_config(&yaml(script)).expect("config should parse");
    let request = make_request(Method::GET, "/");
    let mut ctx = make_filter_context(&request);
    let mut body = Some(Bytes::from_static(b"chunk"));
    filter.on_response_body(&mut ctx, &mut body, true)
}

/// Builds a TCP filter context with every field populated.
fn make_tcp_context() -> TcpFilterContext<'static> {
    TcpFilterContext {
        remote_addr: "127.0.0.1:12345",
        local_addr: "127.0.0.1:2323",
        sni: Some("example.com"),
        upstream_addr: Some(Cow::Borrowed("10.0.0.1:5432")),
        lb_selection: None,
        cluster: Some(Arc::from("backend")),
        health_registry: None,
        kv_stores: None,
        connect_time: Instant::now(),
        bytes_in: u64::MAX,
        bytes_out: 3,
    }
}

/// Builds a load-balancer selection for the TCP context.
fn lb_selection() -> TcpLbSelection {
    let strategy = Arc::new(Strategy::RoundRobin(RoundRobin::new(Vec::new())));
    TcpLbSelection::new(Arc::from("backend"), strategy, Arc::from("10.0.0.1:5432"))
}

/// Parse a YAML string into a `serde_yaml::Value`.
fn yaml(s: &str) -> serde_yaml::Value {
    serde_yaml::from_str(s).expect("test YAML should parse")
}
