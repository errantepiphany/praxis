// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! [`LuaHttpFilter`] implementation and `HttpFilter` trait impl.
//!
//! Gated behind the experimental `scripting` cargo feature.

use std::collections::HashMap;

use async_trait::async_trait;
use bytes::Bytes;
use mlua::{Lua, ObjectLike as _, Table, Value};

use super::http_config::{LuaBodyAccess, LuaBodyMode, LuaHttpConfig};
use crate::{
    actions::FilterAction,
    body::{BodyAccess, BodyMode},
    factory::parse_filter_config,
    filter::{FilterError, HttpFilter, HttpFilterContext},
    scripting::lua::api,
};

/// The filter's name, used in both config lookup and `HttpFilter::name`.
const LUA_HTTP: &str = "lua_http";

/// Rejects a body mode declared without matching body access: a filter's
/// mode only takes effect once its access declaration opts it into body
/// processing, so a mode alone would silently do nothing.
///
/// # Errors
///
/// Returns [`FilterError`] when the `{direction}_body_mode` is
/// `stream_buffer` but the `{direction}_body_access` is `none`.
fn check_body_config(direction: &str, access: LuaBodyAccess, mode: LuaBodyMode) -> Result<(), FilterError> {
    if access == LuaBodyAccess::None && mode == LuaBodyMode::StreamBuffer {
        return Err(FilterError::from(format!(
            "{LUA_HTTP}: {direction}_body_mode requires a {direction}_body_access of read_only or read_write"
        )));
    }
    Ok(())
}

/// Experimental Lua HTTP filter (requires the `scripting` cargo feature;
/// intended for development and testing only).
///
/// Runs the configured script once, when the filter is built, then calls
/// whichever hooks the script registered on `praxis.filter` —
/// `on_request`, `on_request_body`, `on_response`, and `on_response_body` —
/// as each request and response streams through. Hooks receive a snapshot
/// table of the request or response head; edits the script makes to it are
/// discarded. The body hooks' table additionally carries the buffered body
/// chunk and an `end_of_stream` flag, when a chunk is buffered. Scripts
/// log via `praxis.log` and pretty-print values via `praxis.util.inspect`.
/// Body hook participation and delivery are configured per direction via
/// `request_body_access`/`request_body_mode` and
/// `response_body_access`/`response_body_mode`.
///
/// # YAML configuration
///
/// ```yaml
/// filter: lua_http
/// script: <lua source code>
/// request_body_access: read_only
/// request_body_mode: stream_buffer
/// response_body_access: read_only
/// response_body_mode: stream_buffer
/// ```
pub struct LuaHttpFilter {
    /// The Lua state the script ran in; owns the `filter` table.
    lua: Lua,
    /// The script's `praxis.filter.http.HttpFilter` table, holding whichever of the four
    /// hook functions the script registered.
    filter: Table,
    /// Configured request body hook participation.
    request_body_access: BodyAccess,
    /// Configured request body delivery mode.
    request_body_mode: BodyMode,
    /// Configured response body hook participation.
    response_body_access: BodyAccess,
    /// Configured response body delivery mode.
    response_body_mode: BodyMode,
}

impl LuaHttpFilter {
    /// Builds the filter from parsed config: creates the Lua state, loads
    /// the bundled `inspect` and `praxis` libraries, wires the Rust tracing
    /// macros into `praxis.log`, exposes `praxis` as a global, runs the
    /// script, and stores its `praxis.filter` table for later hook lookups.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if the config is invalid (including a body
    /// mode declared without body access), API setup fails, the script
    /// fails to run, or `praxis.filter` is missing or not a table.
    pub fn from_config(config: &serde_yaml::Value) -> Result<Box<dyn HttpFilter>, FilterError> {
        let config: LuaHttpConfig = parse_filter_config(LUA_HTTP, config)?;
        check_body_config("request", config.request_body_access, config.request_body_mode)?;
        check_body_config("response", config.response_body_access, config.response_body_mode)?;

        let lua = Lua::new();
        api::load_library(&lua)?;
        api::wire_logging(&lua)?;

        lua.load(&config.script).exec()?;

        let get = api::get_function(&lua, "praxis.filter.http", "get")?;
        // A script that never called `praxis.filter.http.set` leaves the
        // filter nil; an empty table has no hooks, so every phase continues.
        let filter = get.call::<Option<Table>>(())?.map_or_else(|| lua.create_table(), Ok)?;

        Ok(Box::new(Self {
            lua,
            filter,
            request_body_access: config.request_body_access.into(),
            request_body_mode: config.request_body_mode.into(),
            response_body_access: config.response_body_access.into(),
            response_body_mode: config.response_body_mode.into(),
        }))
    }

    /// Builds the request's `praxis.filter.http.HttpBody`: the buffered
    /// request body when the configured access and mode expose it on the
    /// context, an empty body otherwise.
    ///
    /// Only `stream_buffer` mode pre-reads the body onto the context, and
    /// the access declaration opts this filter into that buffering; in
    /// `stream` mode the bytes reach `on_request_body` as chunks instead.
    ///
    /// # Errors
    ///
    /// Returns [`mlua::Error`] if the constructor call fails.
    fn build_request_body(&self, ctx: &HttpFilterContext<'_>) -> mlua::Result<Table> {
        let buffered = if self.request_body_access.participates()
            && matches!(self.request_body_mode, BodyMode::StreamBuffer { .. })
        {
            ctx.buffered_request_body.as_ref()
        } else {
            None
        };
        build_body(&self.lua, buffered)
    }

    /// Builds the response's `praxis.filter.http.HttpBody`: always empty,
    /// because response bytes are never buffered on the filter context.
    ///
    /// They reach `on_response_body` as chunks instead — the complete
    /// body as the end-of-stream chunk in `stream_buffer` mode. The
    /// configured access decides whether that hook runs at all, and the
    /// mode how the protocol layer buffers and forwards the stream.
    ///
    /// # Errors
    ///
    /// Returns [`mlua::Error`] if the constructor call fails.
    fn build_response_body(&self) -> mlua::Result<Table> {
        build_body(&self.lua, None)
    }
}

#[async_trait]
impl HttpFilter for LuaHttpFilter {
    /// Returns `"lua_http"`.
    fn name(&self) -> &'static str {
        LUA_HTTP
    }

    // filter request functions ----------------------------------------

    /// Builds the request Lua objects — `praxis.net.Uri`,
    /// `praxis.filter.http.HttpBody` (the buffered request body when the
    /// configured access and mode expose it, empty otherwise),
    /// `praxis.filter.http.HttpRequest` and
    /// `praxis.filter.http.HttpContext` (response still nil) — from the
    /// filter context, calls the script's `on_request` hook with the
    /// context, and returns the [`FilterAction`] the hook produced.
    ///
    /// When the script's filter does not define `on_request`, the Lua
    /// objects are not built and the filter continues.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if building an object, calling the hook, or
    /// converting its result fails.
    async fn on_request(&self, ctx: &mut HttpFilterContext<'_>) -> Result<FilterAction, FilterError> {
        if !has_hook(&self.filter, "on_request")? {
            return Ok(FilterAction::Continue);
        }

        let lua = &self.lua;
        let uri = build_uri(lua, ctx)?;
        let request_body = self.build_request_body(ctx)?;
        let request = build_request(lua, ctx, &request_body, &uri)?;
        let http_context = build_context(lua, &request, None)?;

        // `call_method` passes the filter table as the `self` argument, so
        // this is the Lua `filter:on_request(ctx)` method call.
        let action = self.filter.call_method::<Option<Table>>("on_request", http_context)?;
        convert_action("on_request", action)
    }

    /// Returns the configured request body access
    /// (`request_body_access` in YAML; defaults to `none`).
    fn request_body_access(&self) -> BodyAccess {
        self.request_body_access
    }

    /// Returns the configured request body mode
    /// (`request_body_mode` in YAML; defaults to `stream`).
    fn request_body_mode(&self) -> BodyMode {
        self.request_body_mode
    }

    /// Builds the request Lua objects as in `on_request`, wraps the incoming
    /// body chunk in a separate `praxis.filter.http.HttpBody` (the complete
    /// body as the end-of-stream chunk in `stream_buffer` mode), calls the
    /// script's `on_request_body` hook with the context, chunk and
    /// end-of-stream flag, and returns the [`FilterAction`] the hook
    /// produced.
    ///
    /// When the script's filter does not define `on_request_body`, the Lua
    /// objects are not built and the filter continues.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if building an object, calling the hook, or
    /// converting its result fails.
    async fn on_request_body(
        &self,
        ctx: &mut HttpFilterContext<'_>,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
    ) -> Result<FilterAction, FilterError> {
        if !has_hook(&self.filter, "on_request_body")? {
            return Ok(FilterAction::Continue);
        }

        let lua = &self.lua;
        let uri = build_uri(lua, ctx)?;
        let request_body = self.build_request_body(ctx)?;
        let request = build_request(lua, ctx, &request_body, &uri)?;
        let http_context = build_context(lua, &request, None)?;
        let chunk_body = build_body(lua, body.as_ref())?;

        // `call_method` passes the filter table as the `self` argument, so
        // this is the Lua `filter:on_request_body(ctx, body, end_of_stream)` method call.
        let action = self
            .filter
            .call_method::<Option<Table>>("on_request_body", (http_context, chunk_body, end_of_stream))?;
        convert_action("on_request_body", action)
    }

    // filter response functions ---------------------------------------

    /// Builds the request and response Lua objects — `praxis.net.Uri`,
    /// `praxis.filter.http.HttpBody`, `praxis.filter.http.HttpRequest`,
    /// `praxis.filter.http.HttpResponse` and
    /// `praxis.filter.http.HttpContext` — from the filter context; the
    /// request body follows the request configuration and the response
    /// body is always empty because response bytes are never buffered on
    /// the context. Calls the script's `on_response` hook with the context
    /// and returns the [`FilterAction`] the hook produced.
    ///
    /// When the script's filter does not define `on_response`, the Lua
    /// objects are not built and the filter continues.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if building an object, calling the hook, or
    /// converting its result fails.
    async fn on_response(&self, ctx: &mut HttpFilterContext<'_>) -> Result<FilterAction, FilterError> {
        if !has_hook(&self.filter, "on_response")? {
            return Ok(FilterAction::Continue);
        }

        let lua = &self.lua;
        let uri = build_uri(lua, ctx)?;
        let request_body = self.build_request_body(ctx)?;
        let request = build_request(lua, ctx, &request_body, &uri)?;
        let response_body = self.build_response_body()?;
        let response = build_response(lua, ctx, &response_body)?;
        let http_context = build_context(lua, &request, response.as_ref())?;

        // `call_method` passes the filter table as the `self` argument, so
        // this is the Lua `filter:on_response(ctx)` method call.
        let action = self.filter.call_method::<Option<Table>>("on_response", http_context)?;
        convert_action("on_response", action)
    }

    /// Returns the configured response body access
    /// (`response_body_access` in YAML; defaults to `none`).
    fn response_body_access(&self) -> BodyAccess {
        self.response_body_access
    }

    /// Returns the configured response body mode
    /// (`response_body_mode` in YAML; defaults to `stream`).
    fn response_body_mode(&self) -> BodyMode {
        self.response_body_mode
    }

    /// Builds the request and response Lua objects as in `on_response`,
    /// wraps the incoming body chunk in a separate
    /// `praxis.filter.http.HttpBody` (the complete body as the
    /// end-of-stream chunk in `stream_buffer` mode), calls the script's
    /// `on_response_body` hook with the context, chunk and end-of-stream
    /// flag, and returns the [`FilterAction`] the hook produced.
    ///
    /// When the script's filter does not define `on_response_body`, the Lua
    /// objects are not built and the filter continues.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if building an object, calling the hook, or
    /// converting its result fails.
    fn on_response_body(
        &self,
        ctx: &mut HttpFilterContext<'_>,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
    ) -> Result<FilterAction, FilterError> {
        if !has_hook(&self.filter, "on_response_body")? {
            return Ok(FilterAction::Continue);
        }

        let lua = &self.lua;
        let uri = build_uri(lua, ctx)?;
        let request_body = self.build_request_body(ctx)?;
        let request = build_request(lua, ctx, &request_body, &uri)?;
        let response_body = self.build_response_body()?;
        let response = build_response(lua, ctx, &response_body)?;
        let http_context = build_context(lua, &request, response.as_ref())?;
        let chunk_body = build_body(lua, body.as_ref())?;

        // `call_method` passes the filter table as the `self` argument, so
        // this is the Lua `filter:on_response_body(ctx, body, end_of_stream)` method call.
        let action = self
            .filter
            .call_method::<Option<Table>>("on_response_body", (http_context, chunk_body, end_of_stream))?;
        convert_action("on_response_body", action)
    }
}

/// Returns whether the script's filter defines the hook function `name`.
///
/// The lookup follows the filter's metatable, so a hook inherited from the
/// `praxis.filter.http.HttpFilter` base class counts as defined.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the lookup fails.
fn has_hook(filter: &Table, name: &str) -> mlua::Result<bool> {
    Ok(matches!(filter.get::<Value>(name)?, Value::Function(_)))
}

/// Converts the `praxis.filter.FilterAction` the hook named `hook` returned
/// into the matching Rust [`FilterAction`]; a hook returning nothing maps to
/// [`FilterAction::Continue`].
///
/// Only the payload-free variants are representable so far: `reject`,
/// `terminal_response` and `streaming_terminal_response` carry payloads the
/// Lua API has no type for yet, so they are reported as errors rather than
/// silently downgraded.
///
/// # Errors
///
/// Returns [`FilterError`] if reading the action's kind fails or the kind is
/// unknown or unsupported.
fn convert_action(hook: &str, action: Option<Table>) -> Result<FilterAction, FilterError> {
    let Some(action) = action else {
        return Ok(FilterAction::Continue);
    };
    let kind: String = action.call_method("kind", ())?;
    match kind.as_str() {
        "continue" => Ok(FilterAction::Continue),
        "release" => Ok(FilterAction::Release),
        "body_done" => Ok(FilterAction::BodyDone),
        _ => Err(FilterError::from(format!(
            "{LUA_HTTP}: {hook} returned unsupported FilterAction kind {kind:?}"
        ))),
    }
}

/// Builds a `praxis.net.Uri` from the request context.
///
/// Proxy requests normally carry an origin-form target (`/path?query`), so
/// the URI itself has no scheme or authority. The scheme is therefore
/// derived from the connection (`https` when TLS terminated downstream), and
/// the authority from the URI when present (HTTP/2 `:authority` or an
/// absolute-form target), falling back to the `Host` header. This mirrors
/// how the Policy request facts and forwarded-header filters derive them.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_uri(lua: &Lua, ctx: &HttpFilterContext<'_>) -> mlua::Result<Table> {
    let new = api::get_function(lua, "praxis.net.Uri", "new")?;
    let uri = &ctx.request.uri;
    let scheme = if ctx.downstream_tls { "https" } else { "http" };
    let authority = uri.authority().map(http::uri::Authority::as_str).or_else(|| {
        ctx.request
            .headers
            .get(http::header::HOST)
            .and_then(|value| value.to_str().ok())
            .filter(|value| value.parse::<http::uri::Authority>().is_ok())
    });
    new.call((scheme, authority, uri.path(), uri.query()))
}

/// Builds a `praxis.filter.http.HttpBody` wrapping `body`'s bytes, or an
/// empty body when `body` is `None`.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_body(lua: &Lua, body: Option<&Bytes>) -> mlua::Result<Table> {
    let new = api::get_function(lua, "praxis.filter.http.HttpBody", "new")?;
    let raw = body.map_or_else(|| lua.create_string(""), |chunk| lua.create_string(chunk))?;
    new.call(raw)
}

/// Builds a `praxis.filter.http.HttpRequest` from the context's request
/// head, the given body and the given URI.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_request(lua: &Lua, ctx: &HttpFilterContext<'_>, body: &Table, uri: &Table) -> mlua::Result<Table> {
    let new = api::get_function(lua, "praxis.filter.http.HttpRequest", "new")?;
    let headers = build_headers(lua, &ctx.request.headers)?;
    new.call((headers, body.clone(), ctx.request.method.as_str(), uri.clone()))
}

/// Builds a `praxis.filter.http.HttpResponse` from the context's response
/// head and the given body; `None` while no response head is set.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_response(lua: &Lua, ctx: &HttpFilterContext<'_>, body: &Table) -> mlua::Result<Option<Table>> {
    let Some(response) = ctx.response_header.as_deref() else {
        return Ok(None);
    };
    let new = api::get_function(lua, "praxis.filter.http.HttpResponse", "new")?;
    let headers = build_headers(lua, &response.headers)?;
    new.call((headers, body.clone(), i64::from(response.status.as_u16())))
        .map(Some)
}

/// Builds a `praxis.filter.http.HttpContext` pairing `request` with
/// `response`, which maps to nil when `None`.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_context(lua: &Lua, request: &Table, response: Option<&Table>) -> mlua::Result<Table> {
    let new = api::get_function(lua, "praxis.filter.http.HttpContext", "new")?;
    new.call((request.clone(), response.cloned()))
}

/// Builds a header table: single-valued headers map to strings, repeated
/// headers to one-based lists.
///
/// # Errors
///
/// Returns [`mlua::Error`] if string or table construction fails.
fn build_headers(lua: &Lua, headers: &http::HeaderMap) -> mlua::Result<Table> {
    let mut collected: HashMap<&str, Vec<String>> = HashMap::new();
    for (name, value) in headers {
        collected
            .entry(name.as_str())
            .or_default()
            .push(String::from_utf8_lossy(value.as_bytes()).into_owned());
    }
    let table = lua.create_table()?;
    for (name, values) in collected {
        table.set(name, build_header_value(lua, &values)?)?;
    }
    Ok(table)
}

/// Converts one header's values into a single Lua string when the header
/// appears once, or a one-based list when it repeats. Empty input (which
/// [`build_headers`] never produces) maps to nil.
///
/// # Errors
///
/// Returns [`mlua::Error`] if string or table construction fails.
fn build_header_value(lua: &Lua, values: &[String]) -> mlua::Result<Value> {
    if values.is_empty() {
        return Ok(Value::Nil);
    }
    if let [only] = values {
        return Ok(Value::String(lua.create_string(only.as_str())?));
    }
    let list = lua.create_table()?;
    // Lua list indexes are one-based, unlike Rust slices.
    for (value, index) in values.iter().zip(1_i64..) {
        list.set(index, value.as_str())?;
    }
    Ok(Value::Table(list))
}
