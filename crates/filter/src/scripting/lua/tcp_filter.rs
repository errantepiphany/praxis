// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! [`LuaTcpFilter`] implementation and [`TcpFilter`] trait impl.
//!
//! Gated behind the experimental `scripting` cargo feature.

use async_trait::async_trait;
use tracing::info;

use crate::{
    EmptyFilterConfig,
    actions::FilterAction,
    filter::FilterError,
    parse_filter_config,
    tcp_filter::{TcpFilter, TcpFilterContext},
};

/// The filter's name, used in both config lookup and `HttpFilter::name`.
const LUA_TCP: &str = "lua_tcp";

/// Experimental Lua TCP filter (requires the `scripting` cargo feature;
/// intended for development and testing only).
///
/// Currently a placeholder: it accepts no configuration fields and runs no
/// Lua. Its connection hooks only log the connection details.
///
/// TODO: Execute a configured script (look into mlua's async feature and/or a
/// Lua instance per thread).
///
/// # YAML configuration
///
/// ```yaml
/// filter: lua_tcp
/// ```
pub struct LuaTcpFilter;

impl LuaTcpFilter {
    /// Builds the filter from parsed config.
    ///
    /// The filter takes no configuration; any field besides `filter` is
    /// rejected.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError`] if the config contains unknown fields.
    pub fn from_config(config: &serde_yaml::Value) -> Result<Box<dyn TcpFilter>, FilterError> {
        let _: EmptyFilterConfig = parse_filter_config(LUA_TCP, config)?;
        Ok(Box::new(Self))
    }
}

#[async_trait]
impl TcpFilter for LuaTcpFilter {
    fn name(&self) -> &'static str {
        LUA_TCP
    }

    async fn on_connect(&self, ctx: &mut TcpFilterContext<'_>) -> Result<FilterAction, FilterError> {
        info!(
            remote = ctx.remote_addr,
            local = ctx.local_addr,
            upstream = ctx.upstream_addr.as_deref().unwrap_or("-"),
            sni = ctx.sni.unwrap_or("-"),
            "TCP connection accepted (LUA)"
        );
        Ok(FilterAction::Continue)
    }

    async fn on_disconnect(&self, ctx: &mut TcpFilterContext<'_>) -> Result<(), FilterError> {
        let duration_ms = u64::try_from(ctx.connect_time.elapsed().as_millis()).unwrap_or(u64::MAX);
        info!(
            remote = ctx.remote_addr,
            upstream = ctx.upstream_addr.as_deref().unwrap_or("-"),
            sni = ctx.sni.unwrap_or("-"),
            duration_ms,
            bytes_in = ctx.bytes_in,
            bytes_out = ctx.bytes_out,
            "TCP connection closed (LUA)"
        );
        Ok(())
    }
}
