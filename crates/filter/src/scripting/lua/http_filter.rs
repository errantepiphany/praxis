// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! [`LuaHttpFilter`] implementation and [`HttpFilter`] trait impl.
//!
//! Gated behind the experimental `scripting` cargo feature.

use std::collections::HashMap;

use async_trait::async_trait;
use bytes::Bytes;
use mlua::{Lua, ObjectLike as _, Table, Value};
use serde::Deserialize;
use tracing::warn;

use super::{
    LOG_TARGET,
    api::{self, convert_action, convert_body_access, convert_body_mode, lua_integer, lua_integer_usize},
};
use crate::{
    actions::FilterAction,
    body::{BodyAccess, BodyMode},
    factory::parse_filter_config,
    filter::{FilterError, HttpFilter, HttpFilterContext},
};

/// The filter's name, used in both config lookup and `HttpFilter::name`.
const LUA_HTTP: &str = "lua_http";

/// Deserialized YAML config for the Lua HTTP filter.
///
/// ```yaml
/// filter: lua_http
/// script: <lua source code>
/// ```
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LuaHttpConfig {
    /// The script to evaluate.
    script: String,
}

/// Experimental Lua HTTP filter (requires the `scripting` cargo feature;
/// intended for development and testing only).
///
/// Runs the configured script once, when the filter is built, then calls
/// whichever of the four hooks the registered filter defines — `on_request`,
/// `on_request_body`, `on_response`, and `on_response_body` — as each
/// request and response streams through. Hooks receive a fresh
/// `praxis.filter.http.HttpFilterContext` built per call, so edits the
/// script makes to it are discarded. Body hooks additionally receive the
/// body chunk as a `praxis.util.Bytes` and an `end_of_stream` flag. Scripts
/// log via `praxis.log` and pretty-print values via `praxis.util.inspect`.
///
/// Body hook participation and delivery are declared by the script's
/// `request_body_access`/`request_body_mode` and
/// `response_body_access`/`response_body_mode` methods, which default to
/// `BodyAccess.none()` and `BodyMode.stream()`.
pub struct LuaHttpFilter {
    /// The Lua state the script ran in; owns the `filter` table.
    lua: Lua,
    /// The filter table `praxis.filter.http.get()` returned after the script
    /// ran; hook lookups go through it and its metatable.
    filter: Table,
}

impl LuaHttpFilter {
    /// Builds the filter from parsed config: creates the Lua state, loads the
    /// bundled `inspect` and `praxis` libraries, wires `praxis.log` into the
    /// Rust tracing macros, exposes `praxis` as a global, runs the script, and
    /// stores the table `praxis.filter.http.get()` returns for later hook
    /// lookups.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if the config is invalid, API setup fails, the
    /// script fails to run, or `praxis.filter.http` is missing or not a table.
    pub fn from_config(config: &serde_yaml::Value) -> Result<Box<dyn HttpFilter>, FilterError> {
        let config: LuaHttpConfig = parse_filter_config(LUA_HTTP, config)?;
        let lua = api::build_lua(&config.script)?;
        let filter = api::get_registered(&lua, "praxis.filter.http")?;
        Ok(Box::new(Self { lua, filter }))
    }

    /// Calls `method` on the registered filter and converts the returned Lua
    /// `BodyAccess`, falling back to `BodyAccess::None` when the method is
    /// missing, errors, or returns an invalid value.
    fn lua_body_access(&self, method: &str) -> BodyAccess {
        let Some(access) = self.call_optional_method(method) else {
            return BodyAccess::None;
        };
        convert_body_access(&access).unwrap_or_else(|error| {
            warn!(target: LOG_TARGET, "{error}");
            BodyAccess::None
        })
    }

    /// Calls `method` on the registered filter and converts the returned Lua
    /// `BodyMode`, falling back to `BodyMode::Stream` when the method is
    /// missing, errors, or returns an invalid value.
    fn lua_body_mode(&self, method: &str) -> BodyMode {
        let Some(mode) = self.call_optional_method(method) else {
            return BodyMode::Stream;
        };
        convert_body_mode(&mode).unwrap_or_else(|error| {
            warn!(target: LOG_TARGET, "{error}");
            BodyMode::Stream
        })
    }

    /// Calls the filter method `method` (which takes no arguments) and returns
    /// its table result, logging and returning `None` on any error.
    fn call_optional_method(&self, method: &str) -> Option<Table> {
        match api::method(&self.filter, method) {
            Ok(Some(_)) => match self.filter.call_method::<Option<Table>>(method, ()) {
                Ok(result) => result,
                Err(error) => {
                    warn!(target: LOG_TARGET, "{error}");
                    None
                },
            },
            Ok(None) => None,
            Err(error) => {
                warn!(target: LOG_TARGET, "{error}");
                None
            },
        }
    }

    /// Builds the request Lua objects (URI, request, context) and calls the
    /// hook `hook` with `args`.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if building an object, calling the hook, or
    /// converting its result fails.
    fn call_hook(
        &self,
        hook: &str,
        ctx: &HttpFilterContext<'_>,
        args: impl mlua::IntoLuaMulti,
    ) -> Result<FilterAction, FilterError> {
        let lua = &self.lua;
        let uri = build_uri(lua, ctx)?;
        let request = build_request(lua, ctx, &uri)?;
        let response = build_response(lua, ctx)?;
        let http_context = build_context(lua, ctx, &request, response.as_ref())?;
        let action = self.filter.call_method::<Option<Table>>(hook, (http_context, args))?;
        convert_action(LUA_HTTP, hook, action)
    }
}

#[async_trait]
impl HttpFilter for LuaHttpFilter {
    /// Returns `"lua_http"`.
    fn name(&self) -> &'static str {
        LUA_HTTP
    }

    // filter request functions ----------------------------------------

    /// Returns the script's request body access declaration (defaults to
    /// `none`).
    fn request_body_access(&self) -> BodyAccess {
        self.lua_body_access("request_body_access")
    }

    /// Returns the script's request body mode declaration (defaults to
    /// `stream`).
    fn request_body_mode(&self) -> BodyMode {
        self.lua_body_mode("request_body_mode")
    }

    /// Calls the script's `on_request` hook with the request context and
    /// returns the [`FilterAction`] the hook produced.
    ///
    /// When the script's filter does not define `on_request`, the Lua objects
    /// are not built and the filter continues.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if building an object, calling the hook, or
    /// converting its result fails.
    async fn on_request(&self, ctx: &mut HttpFilterContext<'_>) -> Result<FilterAction, FilterError> {
        if !api::has_hook(&self.filter, "on_request")? {
            return Ok(FilterAction::Continue);
        }
        self.call_hook("on_request", ctx, ())
    }

    /// Calls the script's `on_request_body` hook with the request context, the
    /// incoming body chunk and the end-of-stream flag, and returns the
    /// [`FilterAction`] the hook produced.
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
        if !api::has_hook(&self.filter, "on_request_body")? {
            return Ok(FilterAction::Continue);
        }
        let chunk = build_body(&self.lua, body.as_ref())?;
        self.call_hook("on_request_body", ctx, (chunk, end_of_stream))
    }

    // filter response functions ---------------------------------------

    /// Returns the script's response body access declaration (defaults to
    /// `none`).
    fn response_body_access(&self) -> BodyAccess {
        self.lua_body_access("response_body_access")
    }

    /// Returns the script's response body mode declaration (defaults to
    /// `stream`).
    fn response_body_mode(&self) -> BodyMode {
        self.lua_body_mode("response_body_mode")
    }

    /// Calls the script's `on_response` hook with the request and response
    /// context and returns the [`FilterAction`] the hook produced.
    ///
    /// When the script's filter does not define `on_response`, the Lua
    /// objects are not built and the filter continues.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if building an object, calling the hook, or
    /// converting its result fails.
    async fn on_response(&self, ctx: &mut HttpFilterContext<'_>) -> Result<FilterAction, FilterError> {
        if !api::has_hook(&self.filter, "on_response")? {
            return Ok(FilterAction::Continue);
        }
        self.call_hook("on_response", ctx, ())
    }

    /// Calls the script's `on_response_body` hook with the request and
    /// response context, the incoming body chunk and the end-of-stream flag,
    /// and returns the [`FilterAction`] the hook produced.
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
        if !api::has_hook(&self.filter, "on_response_body")? {
            return Ok(FilterAction::Continue);
        }
        let chunk = build_body(&self.lua, body.as_ref())?;
        self.call_hook("on_response_body", ctx, (chunk, end_of_stream))
    }
}

/// Builds a `praxis.net.Uri` from the request context.
///
/// Proxy requests normally carry an origin-form target (`/path?query`), so
/// the URI itself has no scheme or authority. The scheme is therefore
/// derived from the connection (`https` when TLS terminated downstream), and
/// the host and port from the URI when present (HTTP/2 `:authority` or an
/// absolute-form target), falling back to the `Host` header. The host keeps
/// an IPv6 literal's brackets, and the port is `nil` when the authority
/// names none, matching `http::uri::Authority`. This mirrors how the Policy
/// request facts and forwarded-header filters derive them.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_uri(lua: &Lua, ctx: &HttpFilterContext<'_>) -> mlua::Result<Table> {
    let new = api::get_function(lua, "praxis.net.Uri", "new")?;
    let uri = &ctx.request.uri;
    let scheme = if ctx.downstream_tls { "https" } else { "http" };
    let authority = uri.authority().cloned().or_else(|| {
        ctx.request
            .headers
            .get(http::header::HOST)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<http::uri::Authority>().ok())
    });
    let host = authority.as_ref().map(http::uri::Authority::host);
    let port = authority
        .as_ref()
        .and_then(http::uri::Authority::port_u16)
        .map(i64::from);
    new.call((scheme, host, port, uri.path(), uri.query()))
}

/// Builds a `praxis.util.Bytes` wrapping `body`'s bytes, or an
/// empty body when `body` is `None`, so proxy-built bodies are never
/// `nil`.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_body(lua: &Lua, body: Option<&Bytes>) -> mlua::Result<Table> {
    let new = api::get_function(lua, "praxis.util.Bytes", "new")?;
    let raw = body.map_or_else(|| lua.create_string(""), |chunk| lua.create_string(chunk))?;
    new.call(raw)
}

/// Builds a `praxis.filter.http.HeaderMap` wrapping the Rust header map.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_header_map(lua: &Lua, headers: &http::HeaderMap) -> mlua::Result<Table> {
    let table = build_header_table(lua, headers)?;
    let new = api::get_function(lua, "praxis.filter.http.HeaderMap", "new")?;
    new.call(table)
}

/// Builds a header table: single-valued headers map to strings, repeated
/// headers to one-based lists.
///
/// # Errors
///
/// Returns [`mlua::Error`] if string or table construction fails.
fn build_header_table(lua: &Lua, headers: &http::HeaderMap) -> mlua::Result<Table> {
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
/// [`build_header_table`] never produces) maps to nil.
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

/// Builds a `praxis.filter.http.Request` from the context's request head.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_request(lua: &Lua, ctx: &HttpFilterContext<'_>, uri: &Table) -> mlua::Result<Table> {
    let new = api::get_function(lua, "praxis.filter.http.Request", "new")?;
    let headers = build_header_map(lua, &ctx.request.headers)?;
    new.call((headers, ctx.request.method.as_str(), uri.clone()))
}

/// Builds a `praxis.filter.http.Response` from the context's response head;
/// `None` while no response head is set.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_response(lua: &Lua, ctx: &HttpFilterContext<'_>) -> mlua::Result<Option<Table>> {
    let Some(response) = ctx.response_header.as_deref() else {
        return Ok(None);
    };
    let new = api::get_function(lua, "praxis.filter.http.Response", "new")?;
    let headers = build_header_map(lua, &response.headers)?;
    let status = i64::from(response.status.as_u16());
    new.call((headers, status)).map(Some)
}

/// Builds a `praxis.filter.http.HttpFilterContext` from the full filter
/// context.
///
/// # Errors
///
/// Returns [`mlua::Error`] if constructing any field or the context fails.
fn build_context(
    lua: &Lua,
    ctx: &HttpFilterContext<'_>,
    request: &Table,
    response: Option<&Table>,
) -> mlua::Result<Table> {
    let fields = lua.create_table()?;
    fields.set("request", request.clone())?;
    fields.set("response", response.cloned())?;
    fields.set(
        "buffered_request_body",
        build_body(lua, ctx.buffered_request_body.as_ref())?,
    )?;
    fields.set("client_addr", build_client_addr(lua, ctx.client_addr)?)?;
    fields.set("cluster", ctx.cluster.as_deref())?;
    set_header_collections(&fields, lua, ctx)?;
    set_body_state(&fields, lua, ctx)?;
    let new = api::get_function(lua, "praxis.filter.http.HttpFilterContext", "new")?;
    new.call(fields)
}

/// Adds the ordered header collections to the context fields table.
///
/// # Errors
///
/// Returns [`mlua::Error`] if table construction fails.
fn set_header_collections(fields: &Table, lua: &Lua, ctx: &HttpFilterContext<'_>) -> mlua::Result<()> {
    let extra = ctx
        .extra_request_headers
        .iter()
        .map(|(name, value)| (name.as_ref(), value.as_str()));
    fields.set("extra_request_headers", build_pairs(lua, extra)?)?;
    fields.set(
        "request_headers_to_remove",
        build_name_list(lua, &ctx.request_headers_to_remove)?,
    )?;
    fields.set(
        "request_headers_to_set",
        build_header_pairs(lua, &ctx.request_headers_to_set)?,
    )?;
    Ok(())
}

/// Adds the body modes and byte counters to the context fields table.
///
/// # Errors
///
/// Returns [`mlua::Error`] if table construction fails.
fn set_body_state(fields: &Table, lua: &Lua, ctx: &HttpFilterContext<'_>) -> mlua::Result<()> {
    fields.set("request_body_mode", build_body_mode(lua, ctx.request_body_mode)?)?;
    fields.set("request_body_bytes", lua_integer(ctx.request_body_bytes))?;
    fields.set("response_body_mode", build_body_mode(lua, ctx.response_body_mode)?)?;
    fields.set("response_body_bytes", lua_integer(ctx.response_body_bytes))?;
    Ok(())
}

/// Builds a Lua `BodyMode` from the Rust mode.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_body_mode(lua: &Lua, mode: BodyMode) -> mlua::Result<Table> {
    match mode {
        BodyMode::Stream => api::get_function(lua, "praxis.filter.http.BodyMode", "stream")?.call(()),
        BodyMode::StreamBuffer { max_bytes } => {
            let limit = max_bytes.map(lua_integer_usize);
            api::get_function(lua, "praxis.filter.http.BodyMode", "stream_buffer")?.call(limit)
        },
        BodyMode::SizeLimit { max_bytes } => {
            api::get_function(lua, "praxis.filter.http.BodyMode", "size_limit")?.call(lua_integer_usize(max_bytes))
        },
    }
}

/// Builds a `praxis.net.IpAddr` from the optional client address, or nil.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_client_addr(lua: &Lua, addr: Option<std::net::IpAddr>) -> mlua::Result<Value> {
    let Some(addr) = addr else {
        return Ok(Value::Nil);
    };
    let new = api::get_function(lua, "praxis.net.IpAddr", "new")?;
    new.call(addr.to_string()).map(Value::Table)
}

/// Builds a one-based Lua array of `{ name, value }` pairs.
///
/// # Errors
///
/// Returns [`mlua::Error`] if table construction fails.
fn build_pairs<'a>(lua: &Lua, pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for ((name, value), index) in pairs.into_iter().zip(1_i64..) {
        let pair = lua.create_table()?;
        pair.set(1_i64, name)?;
        pair.set(2_i64, value)?;
        table.set(index, pair)?;
    }
    Ok(table)
}

/// Builds a one-based Lua array of `{ name, value }` pairs from Rust header
/// pairs.
///
/// # Errors
///
/// Returns [`mlua::Error`] if table construction fails.
fn build_header_pairs(lua: &Lua, headers: &[(http::HeaderName, http::HeaderValue)]) -> mlua::Result<Table> {
    let pairs: Vec<(String, String)> = headers
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();
    build_pairs(lua, pairs.iter().map(|(name, value)| (name.as_str(), value.as_str())))
}

/// Builds a one-based Lua array of header names.
///
/// # Errors
///
/// Returns [`mlua::Error`] if table construction fails.
fn build_name_list(lua: &Lua, names: &[http::HeaderName]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (name, index) in names.iter().zip(1_i64..) {
        table.set(index, name.as_str())?;
    }
    Ok(table)
}
