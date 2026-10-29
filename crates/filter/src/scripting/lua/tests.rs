// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Unit tests for Lua HTTP and TCP filters.

use bytes::Bytes;
use http::Method;

use super::LuaHttpFilter;
use crate::{
    BodyAccess, BodyMode, BodyRequirement, FilterAction, FilterError,
    test_utils::{make_filter_context, make_request},
};

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
  praxis.filter.http.set(praxis.filter.http.HttpFilter.new())
",
    );
    let filter = LuaHttpFilter::from_config(&yaml).unwrap();
    assert_eq!(filter.name(), "lua_http");
}

#[test]
fn from_config_defaults_body_configuration() {
    let yaml = yaml(
        "
script: |
  praxis.filter.http.set(praxis.filter.http.HttpFilter.new())
",
    );
    let filter = LuaHttpFilter::from_config(&yaml).unwrap();
    assert_eq!(filter.request_body_access(), BodyAccess::None);
    assert_eq!(filter.request_body_mode(), BodyMode::Stream);
    assert_eq!(filter.response_body_access(), BodyAccess::None);
    assert_eq!(filter.response_body_mode(), BodyMode::Stream);
}

#[test]
fn from_config_parses_body_configuration() {
    let yaml = yaml(
        "
script: |
  praxis.filter.http.set(praxis.filter.http.HttpFilter.new())
request_body_access: read_only
request_body_mode: stream_buffer
response_body_access: read_write
response_body_mode: stream_buffer
",
    );
    let filter = LuaHttpFilter::from_config(&yaml).unwrap();
    assert_eq!(
        filter.request_body_access(),
        BodyAccess::ReadOnly(BodyRequirement::Optional)
    );
    assert_eq!(filter.request_body_mode(), BodyMode::StreamBuffer { max_bytes: None });
    assert_eq!(
        filter.response_body_access(),
        BodyAccess::ReadWrite(BodyRequirement::Optional)
    );
    assert_eq!(filter.response_body_mode(), BodyMode::StreamBuffer { max_bytes: None });
}

#[test]
fn from_config_rejects_mode_without_access() {
    let yaml = yaml(
        "
script: |
  -- x
response_body_mode: stream_buffer
",
    );
    let Err(error) = LuaHttpFilter::from_config(&yaml) else {
        panic!("stream_buffer mode without access should be rejected");
    };
    let error = error.to_string();
    assert!(
        error.contains("response_body_mode requires"),
        "unexpected error: {error}"
    );
}

#[test]
fn from_config_rejects_unknown_access_value() {
    let yaml = yaml(
        "
script: |
  -- x
request_body_access: read
",
    );
    assert!(LuaHttpFilter::from_config(&yaml).is_err());
}

// -----------------------------------------------------------------------------
// Hook Dispatch
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
    let action = run_on_request(plain_filter_script()).await.unwrap();
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn on_request_without_registered_filter_continues() {
    let script = "
script: |
  -- the script never calls praxis.filter.http.set
";

    let action = run_on_request(script).await.unwrap();
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn on_request_returns_hook_action() {
    let script = handler_script("on_request", "return praxis.filter.FilterAction.release()");

    let action = run_on_request(&script).await.unwrap();
    assert!(matches!(action, FilterAction::Release));
}

#[tokio::test]
async fn on_request_maps_body_done() {
    let script = handler_script("on_request", "return praxis.filter.FilterAction.body_done()");

    let action = run_on_request(&script).await.unwrap();
    assert!(matches!(action, FilterAction::BodyDone));
}

#[tokio::test]
async fn on_request_maps_continue() {
    let script = handler_script("on_request", "return praxis.filter.FilterAction.continue()");

    let action = run_on_request(&script).await.unwrap();
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn on_request_rejects_unsupported_action() {
    let script = handler_script("on_request", "return praxis.filter.FilterAction.reject(403)");

    let error = run_on_request(&script)
        .await
        .expect_err("an unsupported action kind should be an error");
    assert!(
        error
            .to_string()
            .contains("on_request returned unsupported FilterAction kind"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn on_request_uri_derives_scheme_and_host_authority() {
    let script = uri_echo_script();
    let filter = LuaHttpFilter::from_config(&yaml(&script)).unwrap();
    let mut request = make_request(Method::GET, "/path?q=1");
    request
        .headers
        .insert(http::header::HOST, "example.com:8080".parse().unwrap());
    let mut ctx = make_filter_context(&request);

    let error = filter
        .on_request(&mut ctx)
        .await
        .expect_err("the hook reports the URI parts as an error");
    assert!(
        error.to_string().contains("http example.com:8080 /path?q=1"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn on_request_uri_derives_https_from_downstream_tls() {
    let script = uri_echo_script();
    let filter = LuaHttpFilter::from_config(&yaml(&script)).unwrap();
    let mut request = make_request(Method::GET, "/");
    request
        .headers
        .insert(http::header::HOST, "example.com".parse().unwrap());
    let mut ctx = make_filter_context(&request);
    ctx.downstream_tls = true;

    let error = filter
        .on_request(&mut ctx)
        .await
        .expect_err("the hook reports the URI parts as an error");
    assert!(
        error.to_string().contains("https example.com /"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn on_request_uri_prefers_absolute_form_authority() {
    let script = uri_echo_script();
    let filter = LuaHttpFilter::from_config(&yaml(&script)).unwrap();
    let mut request = make_request(Method::GET, "http://absolute.example/path");
    request
        .headers
        .insert(http::header::HOST, "header.example".parse().unwrap());
    let mut ctx = make_filter_context(&request);

    let error = filter
        .on_request(&mut ctx)
        .await
        .expect_err("the hook reports the URI parts as an error");
    assert!(
        error.to_string().contains("http absolute.example /path"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn on_request_body_without_hook_continues() {
    let action = run_on_request_body(plain_filter_script()).await.unwrap();
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn on_request_body_returns_hook_action() {
    let script = handler_script("on_request_body", "return praxis.filter.FilterAction.continue()");

    let action = run_on_request_body(&script).await.unwrap();
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn on_request_body_maps_release() {
    let script = handler_script("on_request_body", "return praxis.filter.FilterAction.release()");

    let action = run_on_request_body(&script).await.unwrap();
    assert!(matches!(action, FilterAction::Release));
}

#[tokio::test]
async fn on_request_body_maps_body_done() {
    let script = handler_script("on_request_body", "return praxis.filter.FilterAction.body_done()");

    let action = run_on_request_body(&script).await.unwrap();
    assert!(matches!(action, FilterAction::BodyDone));
}

#[tokio::test]
async fn on_request_body_rejects_unsupported_action() {
    let script = handler_script("on_request_body", "return praxis.filter.FilterAction.reject(403)");

    let error = run_on_request_body(&script)
        .await
        .expect_err("an unsupported action kind should be an error");
    assert!(
        error
            .to_string()
            .contains("on_request_body returned unsupported FilterAction kind"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn on_response_without_hook_continues() {
    let action = run_on_response(plain_filter_script()).await.unwrap();
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn on_response_returns_hook_action() {
    let script = handler_script("on_response", "return praxis.filter.FilterAction.release()");

    let action = run_on_response(&script).await.unwrap();
    assert!(matches!(action, FilterAction::Release));
}

#[tokio::test]
async fn on_response_maps_continue() {
    let script = handler_script("on_response", "return praxis.filter.FilterAction.continue()");

    let action = run_on_response(&script).await.unwrap();
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn on_response_rejects_unsupported_action() {
    let script = handler_script("on_response", "return praxis.filter.FilterAction.reject(403)");

    let error = run_on_response(&script)
        .await
        .expect_err("an unsupported action kind should be an error");
    assert!(
        error
            .to_string()
            .contains("on_response returned unsupported FilterAction kind"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn on_response_body_without_hook_continues() {
    let action = run_on_response_body(plain_filter_script()).unwrap();
    assert!(matches!(action, FilterAction::Continue));
}

#[tokio::test]
async fn on_response_body_returns_hook_action() {
    let script = handler_script("on_response_body", "return praxis.filter.FilterAction.release()");

    let action = run_on_response_body(&script).unwrap();
    assert!(matches!(action, FilterAction::Release));
}

#[tokio::test]
async fn on_response_body_maps_body_done() {
    let script = handler_script("on_response_body", "return praxis.filter.FilterAction.body_done()");

    let action = run_on_response_body(&script).unwrap();
    assert!(matches!(action, FilterAction::BodyDone));
}

#[tokio::test]
async fn on_response_body_rejects_unsupported_action() {
    let script = handler_script("on_response_body", "return praxis.filter.FilterAction.reject(403)");

    let error = run_on_response_body(&script).expect_err("an unsupported action kind should be an error");
    assert!(
        error
            .to_string()
            .contains("on_response_body returned unsupported FilterAction kind"),
        "unexpected error: {error}"
    );
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

/// An `on_request` script that raises the request URI's scheme, authority,
/// path and query, so a failing call reports them.
fn uri_echo_script() -> String {
    handler_script(
        "on_request",
        "local uri = ctx:request():uri(); \
         error(uri:scheme() .. ' ' .. tostring(uri:authority()) .. ' ' .. uri:path() .. '?' .. tostring(uri:query()))",
    )
}

/// Builds a filter from `script` and runs its `on_request` hook against a
/// GET `/` request.
async fn run_on_request(script: &str) -> Result<FilterAction, FilterError> {
    let filter = LuaHttpFilter::from_config(&yaml(script)).unwrap();
    let request = make_request(Method::GET, "/");
    let mut ctx = make_filter_context(&request);
    filter.on_request(&mut ctx).await
}

/// Builds a filter from `script` and runs its `on_request_body` hook against
/// a GET `/` request with a one-chunk body.
async fn run_on_request_body(script: &str) -> Result<FilterAction, FilterError> {
    let filter = LuaHttpFilter::from_config(&yaml(script)).unwrap();
    let request = make_request(Method::GET, "/");
    let mut ctx = make_filter_context(&request);
    let mut body = Some(Bytes::from_static(b"chunk"));
    filter.on_request_body(&mut ctx, &mut body, true).await
}

/// Builds a filter from `script` and runs its `on_response` hook against a
/// GET `/` request with no response head set.
async fn run_on_response(script: &str) -> Result<FilterAction, FilterError> {
    let filter = LuaHttpFilter::from_config(&yaml(script)).unwrap();
    let request = make_request(Method::GET, "/");
    let mut ctx = make_filter_context(&request);
    filter.on_response(&mut ctx).await
}

/// Builds a filter from `script` and runs its `on_response_body` hook
/// against a GET `/` request with a one-chunk body.
fn run_on_response_body(script: &str) -> Result<FilterAction, FilterError> {
    let filter = LuaHttpFilter::from_config(&yaml(script)).unwrap();
    let request = make_request(Method::GET, "/");
    let mut ctx = make_filter_context(&request);
    let mut body = Some(Bytes::from_static(b"chunk"));
    filter.on_response_body(&mut ctx, &mut body, true)
}

// -----------------------------------------------------------------------------
// Test Utilities
// -----------------------------------------------------------------------------

/// Parse a YAML string into a `serde_yaml::Value`.
fn yaml(s: &str) -> serde_yaml::Value {
    serde_yaml::from_str(s).expect("test YAML should parse")
}
