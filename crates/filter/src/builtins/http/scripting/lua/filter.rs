// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! [`LuaFilter`] implementation and `HttpFilter` trait impl.

use std::collections::HashMap;

use async_trait::async_trait;
use bytes::Bytes;
use mlua::{Function, Lua, Table, Value, chunk};
use tracing::{debug, error, info, trace, warn};

use super::config::LuaConfig;
use crate::{
    actions::FilterAction,
    body::{BodyAccess, BodyMode},
    factory::parse_filter_config,
    filter::{FilterError, HttpFilter, HttpFilterContext},
};

/// The filter's name, used in both config lookup and `HttpFilter::name`.
const LUA: &str = "lua";

/// Lua source of the MIT-licensed `inspect` library, which renders any
/// Lua value as a readable string.
/// (<https://github.com/kikito/inspect.lua>).
const INSPECT_LUA: &str = include_str!("inspect.lua");

/// Lua source of the Praxis `praxis` library, providing an API to scripts.
const PRAXIS_LUA: &str = include_str!("praxis.lua");

/// Experimental HTTP Lua filter (requires the `lua-filter` cargo feature;
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
///
/// # YAML configuration
///
/// ```yaml
/// filter: lua
/// script: <lua source code>
/// ```
pub struct LuaFilter {
    /// The Lua state the script ran in; owns the `filter` table.
    lua: Lua,
    /// The script's `praxis.filter` table, holding whichever of the four
    /// hook functions the script registered.
    filter: Table,
}

impl LuaFilter {
    /// Builds the filter from parsed config: creates the Lua state, loads
    /// the bundled `inspect` and `praxis` libraries, wires the Rust tracing
    /// macros into `praxis.log`, exposes `praxis` as a global, runs the
    /// script, and stores its `praxis.filter` table for later hook lookups.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if the config is invalid, API setup fails,
    /// the script fails to run, or `praxis.filter` is missing or not a
    /// table.
    #[expect(clippy::too_many_lines, reason = "code reads cleanly enough")]
    pub fn from_config(config: &serde_yaml::Value) -> Result<Box<dyn HttpFilter>, FilterError> {
        let config: LuaConfig = parse_filter_config(LUA, config)?;

        let lua = Lua::new();

        let inspect_lua = lua.load(INSPECT_LUA).set_name("inspect.lua").eval::<Table>()?;
        let praxis_lua = lua.load(PRAXIS_LUA).set_name("praxis.lua").eval::<Table>()?;

        let error_rs = lua.create_function(|_, msg: String| {
            error!("{msg}");
            Ok(())
        })?;
        let warn_rs = lua.create_function(|_, msg: String| {
            warn!("{msg}");
            Ok(())
        })?;
        let info_rs = lua.create_function(|_, msg: String| {
            info!("{msg}");
            Ok(())
        })?;
        let debug_rs = lua.create_function(|_, msg: String| {
            debug!("{msg}");
            Ok(())
        })?;
        let trace_rs = lua.create_function(|_, msg: String| {
            trace!("{msg}");
            Ok(())
        })?;

        let praxis = lua
            .load(chunk! {
                local log = $praxis_lua.Log.new($error_rs, $warn_rs, $info_rs, $debug_rs, $trace_rs)
                local util = $praxis_lua.Util.new($inspect_lua.inspect)
                return $praxis_lua.Praxis.new(log, util)
            })
            .set_name("praxis")
            .eval::<Table>()?;
        lua.globals().set("praxis", &praxis)?;

        lua.load(&config.script).set_name("config.script").exec()?;

        let filter = praxis.get::<Table>("filter")?;
        Ok(Box::new(Self { lua, filter }))
    }
}

#[async_trait]
impl HttpFilter for LuaFilter {
    /// Returns `"lua"`.
    fn name(&self) -> &'static str {
        LUA
    }

    // filter request functions ----------------------------------------

    /// Runs the script's `praxis.filter:on_request` function, if registered,
    /// with a request snapshot table; always continues.
    async fn on_request(&self, ctx: &mut HttpFilterContext<'_>) -> Result<FilterAction, FilterError> {
        if let Some(on_request) = &self.filter.get::<Option<Function>>("on_request")? {
            on_request.call::<()>(build_request(&self.lua, ctx, None, false)?)?;
        }
        // TODO: Let lua function decide what gets returned here.
        Ok(FilterAction::Continue)
    }

    /// Request body access; currently hardcoded.
    fn request_body_access(&self) -> BodyAccess {
        // FIXME: remove hardcoding
        //
        BodyAccess::ReadOnly
    }

    /// Request body mode; currently hardcoded.
    fn request_body_mode(&self) -> BodyMode {
        // FIXME: remove hardcoding
        //
        BodyMode::StreamBuffer { max_bytes: None }
    }

    /// Runs the script's `praxis.filter:on_request_body` function, if registered,
    /// with a request snapshot table including any buffered body chunk;
    /// always continues.
    async fn on_request_body(
        &self,
        ctx: &mut HttpFilterContext<'_>,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
    ) -> Result<FilterAction, FilterError> {
        if let Some(on_request_body) = &self.filter.get::<Option<Function>>("on_request_body")? {
            on_request_body.call::<()>(build_request(&self.lua, ctx, body.as_ref(), end_of_stream)?)?;
        }
        // TODO: Let lua function decide what gets returned here.
        Ok(FilterAction::Continue)
    }

    // filter response functions ---------------------------------------

    /// Runs the script's `praxis.filter:on_response` function, if registered,
    /// with a response snapshot table; always continues.
    async fn on_response(&self, ctx: &mut HttpFilterContext<'_>) -> Result<FilterAction, FilterError> {
        if let Some(on_response) = &self.filter.get::<Option<Function>>("on_response")? {
            on_response.call::<()>(build_response(&self.lua, ctx, None, false)?)?;
        }
        // TODO: Let lua function decide what gets returned here.
        Ok(FilterAction::Continue)
    }

    /// Response body access; currently hardcoded.
    fn response_body_access(&self) -> BodyAccess {
        // FIXME: remove hardcoding
        //
        BodyAccess::ReadOnly
    }

    /// Response body mode; currently hardcoded.
    fn response_body_mode(&self) -> BodyMode {
        // FIXME: remove hardcoding
        //
        BodyMode::StreamBuffer { max_bytes: None }
    }

    /// Runs the script's `praxis.filter:on_response_body` function, if registered,
    /// with a response snapshot table including any buffered body chunk;
    /// always continues.
    fn on_response_body(
        &self,
        ctx: &mut HttpFilterContext<'_>,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
    ) -> Result<FilterAction, FilterError> {
        if let Some(on_response_body) = &self.filter.get::<Option<Function>>("on_response_body")? {
            on_response_body.call::<()>(build_response(&self.lua, ctx, body.as_ref(), end_of_stream)?)?;
        }
        // TODO: Let lua function decide what gets returned here.
        Ok(FilterAction::Continue)
    }
}

/// Builds the request snapshot table passed to `praxis.filter.on_request`
/// and `on_request_body`: `method`, `path`, `query` (nil when the URI has
/// none), and `headers`, plus `body` and `end_of_stream` when a body chunk
/// is buffered.
///
/// # Errors
///
/// Returns [`mlua::Error`] if table construction fails.
fn build_request(
    lua: &Lua,
    ctx: &HttpFilterContext<'_>,
    body: Option<&Bytes>,
    end_of_stream: bool,
) -> mlua::Result<Table> {
    let handle = lua.create_table()?;
    handle.set("method", ctx.request.method.as_str())?;
    handle.set("path", ctx.request.uri.path())?;
    handle.set("query", ctx.request.uri.query())?;
    handle.set("headers", build_headers(lua, &ctx.request.headers)?)?;
    if let Some(body) = body {
        handle.set("body", lua.create_string(body)?)?;
        handle.set("end_of_stream", end_of_stream)?;
    }
    Ok(handle)
}

/// Builds the response snapshot table passed to `praxis.filter.on_response`
/// and `on_response_body`: `status` and `headers`, plus `body` and
/// `end_of_stream` when a body chunk is buffered; an empty table when no
/// response head is set yet.
///
/// # Errors
///
/// Returns [`mlua::Error`] if table construction fails.
fn build_response(
    lua: &Lua,
    ctx: &HttpFilterContext<'_>,
    body: Option<&Bytes>,
    end_of_stream: bool,
) -> mlua::Result<Table> {
    let handle = lua.create_table()?;
    let Some(response) = ctx.response_header.as_deref() else {
        return Ok(handle);
    };
    handle.set("status", i64::from(response.status.as_u16()))?;
    handle.set("headers", build_headers(lua, &response.headers)?)?;
    if let Some(body) = body {
        handle.set("body", lua.create_string(body)?)?;
        handle.set("end_of_stream", end_of_stream)?;
    }
    Ok(handle)
}

/// Builds a header table: single-valued headers map to strings, repeated
/// headers to one-based lists.
///
/// # Errors
///
/// Returns [`mlua::Error`] if table construction fails.
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
/// [`headers_table`] never produces) maps to nil.
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
