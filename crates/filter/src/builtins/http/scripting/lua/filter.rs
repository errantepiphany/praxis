// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! [`LuaFilter`] implementation and `HttpFilter` trait impl.

use std::collections::HashMap;

use async_trait::async_trait;
use mlua::{Function, Lua, Table, Value};

use super::config::LuaConfig;
use crate::{
    FilterAction,
    FilterError,
    // Rejection,
    factory::parse_filter_config,
    filter::{HttpFilter, HttpFilterContext},
};

/// HTTP Lua filter.
///
/// Experimental: requires the `lua-filter` cargo feature,
/// which is off by default. This filter is intended for development
/// and testing only.
///
/// Uses the `mlua` library:
/// <https://github.com/mlua-rs/mlua>
/// <https://mlua.rs/mlua/index.html>
///
/// Evaluates a Lua script at startup and, on each request, calls
/// the script's global `praxis_on_request` function (if defined).
///
/// The script contract follows the Envoy Lua filter: scripts define
/// global `praxis_on_request(request_handle)` and
/// `praxis_on_response(response_handle)` functions. The handles are
/// read-only snapshot tables:
///
/// - `request_handle`: `method` (string), `path` (string), `query` (string or nil), `headers` (table of name to string,
///   or to a list of strings when a header repeats)
/// - `response_handle`: `status` (number), `headers` (table as above)
///
/// # YAML configuration
///
/// ```yaml
/// filter: lua
/// script: <lua source code>
/// ```
pub struct LuaFilter {
    /// The Lua state that compiled the script; owns the functions below.
    lua: Lua,
    /// The script's global `praxis_on_request` function, if it defines one.
    lua_on_request: Option<Function>,
    /// The script's global `praxis_on_response` function, if it defines one.
    lua_on_response: Option<Function>,
}

impl LuaFilter {
    /// Builds the filter from parsed config. resolving inline
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if the config is invalid.
    pub fn from_config(config: &serde_yaml::Value) -> Result<Box<dyn HttpFilter>, FilterError> {
        let config: LuaConfig = parse_filter_config("lua", config)?;
        let lua = Lua::new();
        lua.load(&config.script).exec()?;
        let globals = lua.globals();
        let request: Option<Function> = globals.get("praxis_on_request")?;
        let response: Option<Function> = globals.get("praxis_on_response")?;
        // FIXME: If neither are defined, it should be a configuration error.

        Ok(Box::new(Self {
            lua,
            lua_on_request: request,
            lua_on_response: response,
        }))
    }
}

#[async_trait]
impl HttpFilter for LuaFilter {
    fn name(&self) -> &'static str {
        "lua"
    }

    async fn on_request(&self, ctx: &mut HttpFilterContext<'_>) -> Result<FilterAction, FilterError> {
        if let Some(on_request) = &self.lua_on_request {
            on_request.call::<()>(request_handle(&self.lua, ctx)?)?;
        }
        Ok(FilterAction::Continue)
    }

    async fn on_response(&self, ctx: &mut HttpFilterContext<'_>) -> Result<FilterAction, FilterError> {
        if let Some(on_response) = &self.lua_on_response {
            on_response.call::<()>(response_handle(&self.lua, ctx)?)?;
        }
        Ok(FilterAction::Continue)
    }
}

/// Builds the `request_handle` snapshot table passed to `praxis_on_request`.
///
/// # Errors
///
/// Returns [`mlua::Error`] if table construction fails.
fn request_handle(lua: &Lua, ctx: &HttpFilterContext<'_>) -> mlua::Result<Table> {
    let handle = lua.create_table()?;
    handle.set("method", ctx.request.method.as_str())?;
    handle.set("path", ctx.request.uri.path())?;
    handle.set("query", ctx.request.uri.query())?;
    handle.set("headers", headers_table(lua, &ctx.request.headers)?)?;
    Ok(handle)
}

/// Builds the `response_handle` snapshot table passed to
/// `praxis_on_response`, or an empty table when no response head is set
/// yet.
///
/// # Errors
///
/// Returns [`mlua::Error`] if table construction fails.
fn response_handle(lua: &Lua, ctx: &HttpFilterContext<'_>) -> mlua::Result<Value> {
    let Some(response) = ctx.response_header.as_deref() else {
        return Ok(Value::Table(lua.create_table()?));
    };
    let handle = lua.create_table()?;
    handle.set("status", i64::from(response.status.as_u16()))?;
    handle.set("headers", headers_table(lua, &response.headers)?)?;
    Ok(Value::Table(handle))
}

/// Builds a Lua table of header name to value. A header that appears
/// once maps to a string; a repeated header maps to a one-based list.
///
/// # Errors
///
/// Returns [`mlua::Error`] if table construction fails.
fn headers_table(lua: &Lua, headers: &http::HeaderMap) -> mlua::Result<Table> {
    let mut collected: HashMap<&str, Vec<String>> = HashMap::new();
    for (name, value) in headers {
        collected
            .entry(name.as_str())
            .or_default()
            .push(String::from_utf8_lossy(value.as_bytes()).into_owned());
    }
    let table = lua.create_table()?;
    for (name, values) in collected {
        table.set(name, header_value(lua, &values)?)?;
    }
    Ok(table)
}

/// Converts one header's values into a single Lua string, or a
/// one-based list when the header repeats.
///
/// # Errors
///
/// Returns [`mlua::Error`] if string or table construction fails.
fn header_value(lua: &Lua, values: &[String]) -> mlua::Result<Value> {
    if values.is_empty() {
        return Ok(Value::Nil);
    }
    if let [only] = values {
        return Ok(Value::String(lua.create_string(only.as_str())?));
    }
    let list = lua.create_table()?;
    for (value, index) in values.iter().zip(1_i64..) {
        list.set(index, value.as_str())?;
    }
    Ok(Value::Table(list))
}
