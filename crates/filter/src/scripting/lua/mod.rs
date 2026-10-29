// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Lua HTTP and TCP filters.

/// Log target for events emitted by Lua scripts.
///
/// Expands to this module's path (`praxis_filter::scripting::lua`).
/// See [`api::wire_logging`] for the handlers that emit to it.
const LOG_TARGET: &str = module_path!();

mod api;
mod http_config;
mod http_filter;
mod tcp_filter;

#[cfg(test)]
#[expect(clippy::allow_attributes, reason = "blanket test suppressions")]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests"
)]
mod tests;

pub use self::{http_filter::LuaHttpFilter, tcp_filter::LuaTcpFilter};
