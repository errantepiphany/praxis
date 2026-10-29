// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! YAML config types for the Lua HTTP filter.
//!
//! Gated behind the experimental `scripting` cargo feature.

use serde::Deserialize;

use crate::body::{BodyAccess, BodyMode, BodyRequirement};

/// Deserialized YAML config for the lua filter.
///
/// ```yaml
/// filter: lua_http
/// script: <lua source code>
/// request_body_access: read_only
/// request_body_mode: stream_buffer
/// response_body_access: read_only
/// response_body_mode: stream_buffer
/// ```
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LuaHttpConfig {
    /// The script to evaluate.
    pub script: String,
    /// Which request body hooks the script receives. Defaults to `none`.
    #[serde(default)]
    pub request_body_access: LuaBodyAccess,
    /// How request body bytes reach those hooks. Defaults to `stream`;
    /// only meaningful when `request_body_access` is not `none`.
    #[serde(default)]
    pub request_body_mode: LuaBodyMode,
    /// Which response body hooks the script receives. Defaults to `none`.
    #[serde(default)]
    pub response_body_access: LuaBodyAccess,
    /// How response body bytes reach those hooks. Defaults to `stream`;
    /// only meaningful when `response_body_access` is not `none`.
    #[serde(default)]
    pub response_body_mode: LuaBodyMode,
}

/// Config-side body access declaration, deserialized from `none`,
/// `read_only`, or `read_write`.
///
/// Converts to [`BodyAccess`] with [`BodyRequirement::Optional`]: the
/// pipeline may still skip the script's body processing for an admitted
/// request rather than rejecting it.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) enum LuaBodyAccess {
    /// No body hooks; message bodies map to empty `HttpBody`s.
    #[default]
    None,
    /// Body hooks observe the body without mutation rights.
    ReadOnly,
    /// Body hooks may rewrite the body.
    ReadWrite,
}

impl From<LuaBodyAccess> for BodyAccess {
    fn from(access: LuaBodyAccess) -> Self {
        match access {
            LuaBodyAccess::None => BodyAccess::None,
            LuaBodyAccess::ReadOnly => BodyAccess::ReadOnly(BodyRequirement::Optional),
            LuaBodyAccess::ReadWrite => BodyAccess::ReadWrite(BodyRequirement::Optional),
        }
    }
}

/// Config-side body delivery mode, deserialized from `stream` or
/// `stream_buffer`.
///
/// Converts to [`BodyMode`]; `stream_buffer` buffers up to the pipeline's
/// buffering cap (64 MiB by default) rather than unbounded.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) enum LuaBodyMode {
    /// Deliver body chunks as they arrive.
    #[default]
    Stream,
    /// Accumulate the body and defer forwarding until end of stream or
    /// release.
    StreamBuffer,
}

impl From<LuaBodyMode> for BodyMode {
    fn from(mode: LuaBodyMode) -> Self {
        match mode {
            LuaBodyMode::Stream => BodyMode::Stream,
            LuaBodyMode::StreamBuffer => BodyMode::StreamBuffer { max_bytes: None },
        }
    }
}
