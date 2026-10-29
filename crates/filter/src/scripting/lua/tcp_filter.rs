// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! [`LuaTcpFilter`] implementation and [`TcpFilter`] trait impl.
//!
//! Gated behind the experimental `scripting` cargo feature.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use mlua::{Lua, ObjectLike as _, Table, Value};
use serde::Deserialize;

use super::api::{self, convert_action, lua_integer};
use crate::{
    actions::FilterAction,
    factory::parse_filter_config,
    filter::FilterError,
    tcp_filter::{TcpFilter, TcpFilterContext},
};

/// The filter's name, used in both config lookup and `TcpFilter::name`.
const LUA_TCP: &str = "lua_tcp";

/// Deserialized YAML config for the Lua TCP filter.
///
/// ```yaml
/// filter: lua_tcp
/// script: <lua source code>
/// ```
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LuaTcpConfig {
    /// The script to evaluate.
    script: String,
}

/// Experimental Lua TCP filter (requires the `scripting` cargo feature;
/// intended for development and testing only).
///
/// Runs the configured script once, when the filter is built, then calls the
/// registered filter's `on_connect` and `on_disconnect` hooks. Each hook
/// receives a fresh `praxis.filter.tcp.TcpFilterContext`, so edits the script
/// makes to it are discarded. Scripts log via `praxis.log` and pretty-print
/// values via `praxis.util.inspect`.
pub struct LuaTcpFilter {
    /// The Lua state the script ran in; owns the `filter` table.
    lua: Lua,
    /// The filter table `praxis.filter.tcp.get()` returned after the script
    /// ran; hook lookups go through it and its metatable.
    filter: Table,
}

impl LuaTcpFilter {
    /// Builds the filter from parsed config: creates the Lua state, loads the
    /// bundled `inspect` and `praxis` libraries, wires `praxis.log` into the
    /// Rust tracing macros, exposes `praxis` as a global, runs the script, and
    /// stores the table `praxis.filter.tcp.get()` returns for later hook
    /// lookups.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if the config is invalid, API setup fails, the
    /// script fails to run, or `praxis.filter.tcp` is missing or not a table.
    pub fn from_config(config: &serde_yaml::Value) -> Result<Box<dyn TcpFilter>, FilterError> {
        let config: LuaTcpConfig = parse_filter_config(LUA_TCP, config)?;
        let lua = api::build_lua(&config.script)?;
        let filter = api::get_registered(&lua, "praxis.filter.tcp")?;
        Ok(Box::new(Self { lua, filter }))
    }
}

#[async_trait]
impl TcpFilter for LuaTcpFilter {
    /// Returns `"lua_tcp"`.
    fn name(&self) -> &'static str {
        LUA_TCP
    }

    /// Calls the script's `on_connect` hook with the connection context and
    /// returns the [`FilterAction`] the hook produced.
    ///
    /// When the script's filter does not define `on_connect`, the Lua objects
    /// are not built and the filter continues.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if building the context, calling the hook, or
    /// converting its result fails.
    async fn on_connect(&self, ctx: &mut TcpFilterContext<'_>) -> Result<FilterAction, FilterError> {
        if !api::has_hook(&self.filter, "on_connect")? {
            return Ok(FilterAction::Continue);
        }
        let context = build_tcp_context(&self.lua, ctx)?;
        let action = self.filter.call_method::<Option<Table>>("on_connect", context)?;
        convert_action(LUA_TCP, "on_connect", action)
    }

    /// Calls the script's `on_disconnect` hook with the connection context.
    ///
    /// The hook's action is ignored: the Rust trait returns `()`.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if building the context, calling the hook, or
    /// converting its result fails.
    async fn on_disconnect(&self, ctx: &mut TcpFilterContext<'_>) -> Result<(), FilterError> {
        if !api::has_hook(&self.filter, "on_disconnect")? {
            return Ok(());
        }
        let context = build_tcp_context(&self.lua, ctx)?;
        let action = self.filter.call_method::<Option<Table>>("on_disconnect", context)?;
        let _action = convert_action(LUA_TCP, "on_disconnect", action)?;
        Ok(())
    }
}

/// Builds a `praxis.filter.tcp.TcpFilterContext` from the connection context.
///
/// # Errors
///
/// Returns [`mlua::Error`] if constructing any field or the context fails.
fn build_tcp_context(lua: &Lua, ctx: &TcpFilterContext<'_>) -> mlua::Result<Table> {
    let fields = lua.create_table()?;
    fields.set("remote_addr", ctx.remote_addr)?;
    fields.set("local_addr", ctx.local_addr)?;
    fields.set("sni", ctx.sni)?;
    fields.set("upstream_addr", ctx.upstream_addr.as_deref())?;
    fields.set("lb_selection", build_lb_selection(lua, ctx)?)?;
    fields.set("cluster", ctx.cluster.as_deref())?;
    fields.set("connect_time", connect_time_seconds(ctx.connect_time))?;
    fields.set("bytes_in", lua_integer(ctx.bytes_in))?;
    fields.set("bytes_out", lua_integer(ctx.bytes_out))?;
    let new = api::get_function(lua, "praxis.filter.tcp.TcpFilterContext", "new")?;
    new.call(fields)
}

/// Builds a `praxis.filter.tcp.TcpLoadSelection` from the connection context,
/// or nil when no selection exists.
///
/// The Rust `TcpLbSelection` fields are private, so the cluster and endpoint
/// are read from the context's public `cluster` and `upstream_addr` fields,
/// which the TCP load balancer sets alongside the selection.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the constructor call fails.
fn build_lb_selection(lua: &Lua, ctx: &TcpFilterContext<'_>) -> mlua::Result<Value> {
    if ctx.lb_selection.is_none() {
        return Ok(Value::Nil);
    }
    let Some(cluster) = ctx.cluster.as_deref() else {
        return Ok(Value::Nil);
    };
    let new = api::get_function(lua, "praxis.filter.tcp.TcpLoadSelection", "new")?;
    new.call((cluster, ctx.upstream_addr.as_deref())).map(Value::Table)
}

/// Converts a monotonic connect timestamp into fractional Unix-epoch seconds.
fn connect_time_seconds(connect_time: Instant) -> f64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
    now.saturating_sub(connect_time.elapsed()).as_secs_f64()
}
