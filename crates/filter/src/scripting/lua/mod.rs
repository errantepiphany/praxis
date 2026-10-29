// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Lua HTTP and TCP filters.

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
