// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Lua API: embedded library loader and Lua-to-Rust conversions.

use mlua::{Function, Lua, ObjectLike as _, Result, Table, Value, chunk};
use tracing::{debug, error, info, trace, warn};

use super::LOG_TARGET;
use crate::{
    actions::{FilterAction, Rejection},
    body::{BodyAccess, BodyMode, BodyRequirement},
    filter::FilterError,
};

/// Embeds the Lua library sources in the binary, installs a
/// `package.searchers` loader that resolves the embedded module names, and
/// exposes the `praxis` namespace as a global for every script.
#[expect(clippy::too_many_lines, reason = "needed")]
pub(super) fn load_library(lua: &Lua) -> Result<()> {
    /// Lua source of the MIT-licensed `inspect` library, which renders any Lua value as a readable string.
    /// <https://github.com/kikito/inspect.lua>
    const INSPECT: &str = include_str!("lib/inspect.lua");
    const PRAXIS: &str = include_str!("lib/praxis/init.lua");
    const PRAXIS_FILTER: &str = include_str!("lib/praxis/filter/init.lua");
    const PRAXIS_FILTER_ACTION: &str = include_str!("lib/praxis/filter/FilterAction.lua");
    const PRAXIS_FILTER_HTTP: &str = include_str!("lib/praxis/filter/http/init.lua");
    const PRAXIS_FILTER_HTTP_BODY_ACCESS: &str = include_str!("lib/praxis/filter/http/BodyAccess.lua");
    const PRAXIS_FILTER_HTTP_BODY_MODE: &str = include_str!("lib/praxis/filter/http/BodyMode.lua");
    const PRAXIS_FILTER_HTTP_CONTEXT: &str = include_str!("lib/praxis/filter/http/HttpFilterContext.lua");
    const PRAXIS_FILTER_HTTP_FILTER: &str = include_str!("lib/praxis/filter/http/HttpFilter.lua");
    const PRAXIS_FILTER_HTTP_HEADER_MAP: &str = include_str!("lib/praxis/filter/http/HeaderMap.lua");
    const PRAXIS_FILTER_HTTP_REQUEST: &str = include_str!("lib/praxis/filter/http/Request.lua");
    const PRAXIS_FILTER_HTTP_RESPONSE: &str = include_str!("lib/praxis/filter/http/Response.lua");
    const PRAXIS_FILTER_TCP: &str = include_str!("lib/praxis/filter/tcp/init.lua");
    const PRAXIS_FILTER_TCP_CONTEXT: &str = include_str!("lib/praxis/filter/tcp/TcpFilterContext.lua");
    const PRAXIS_FILTER_TCP_FILTER: &str = include_str!("lib/praxis/filter/tcp/TcpFilter.lua");
    const PRAXIS_FILTER_TCP_LOAD_SELECTION: &str = include_str!("lib/praxis/filter/tcp/TcpLoadSelection.lua");
    const PRAXIS_LOG: &str = include_str!("lib/praxis/log/init.lua");
    const PRAXIS_LOG_LOG: &str = include_str!("lib/praxis/log/Log.lua");
    const PRAXIS_NET: &str = include_str!("lib/praxis/net/init.lua");
    const PRAXIS_NET_IP_ADDR: &str = include_str!("lib/praxis/net/IpAddr.lua");
    const PRAXIS_NET_URI: &str = include_str!("lib/praxis/net/Uri.lua");
    const PRAXIS_UTIL: &str = include_str!("lib/praxis/util/init.lua");
    const PRAXIS_UTIL_BYTES: &str = include_str!("lib/praxis/util/Bytes.lua");
    const PRAXIS_UTIL_OBJECT: &str = include_str!("lib/praxis/util/Object.lua");

    // Access the package table and searchers array
    let package: Table = lua.globals().get("package")?;

    // In Lua 5.2+, this is "searchers". In Lua 5.1 / LuaJIT, it was "loaders".
    let searchers: Table = package.get("searchers")?;

    // Create a custom loader function
    let custom_loader = lua.create_function(|lua, modname: String| {
        let code = match modname.as_str() {
            "inspect" => Some(INSPECT),
            "praxis" => Some(PRAXIS),
            "praxis.filter" => Some(PRAXIS_FILTER),
            "praxis.filter.FilterAction" => Some(PRAXIS_FILTER_ACTION),
            "praxis.filter.http" => Some(PRAXIS_FILTER_HTTP),
            "praxis.filter.http.BodyAccess" => Some(PRAXIS_FILTER_HTTP_BODY_ACCESS),
            "praxis.filter.http.BodyMode" => Some(PRAXIS_FILTER_HTTP_BODY_MODE),
            "praxis.filter.http.HeaderMap" => Some(PRAXIS_FILTER_HTTP_HEADER_MAP),
            "praxis.filter.http.HttpFilter" => Some(PRAXIS_FILTER_HTTP_FILTER),
            "praxis.filter.http.HttpFilterContext" => Some(PRAXIS_FILTER_HTTP_CONTEXT),
            "praxis.filter.http.Request" => Some(PRAXIS_FILTER_HTTP_REQUEST),
            "praxis.filter.http.Response" => Some(PRAXIS_FILTER_HTTP_RESPONSE),
            "praxis.filter.tcp" => Some(PRAXIS_FILTER_TCP),
            "praxis.filter.tcp.TcpFilter" => Some(PRAXIS_FILTER_TCP_FILTER),
            "praxis.filter.tcp.TcpFilterContext" => Some(PRAXIS_FILTER_TCP_CONTEXT),
            "praxis.filter.tcp.TcpLoadSelection" => Some(PRAXIS_FILTER_TCP_LOAD_SELECTION),
            "praxis.log" => Some(PRAXIS_LOG),
            "praxis.log.Log" => Some(PRAXIS_LOG_LOG),
            "praxis.net" => Some(PRAXIS_NET),
            "praxis.net.IpAddr" => Some(PRAXIS_NET_IP_ADDR),
            "praxis.net.Uri" => Some(PRAXIS_NET_URI),
            "praxis.util" => Some(PRAXIS_UTIL),
            "praxis.util.Bytes" => Some(PRAXIS_UTIL_BYTES),
            "praxis.util.Object" => Some(PRAXIS_UTIL_OBJECT),
            _ => None,
        };

        if let Some(source) = code {
            // Compile the Lua code into a chunk and return it as a function
            let chunk = lua.load(source).set_name(&modname).into_function()?;
            Ok(Value::Function(chunk))
        } else {
            // Return an error string indicating where we looked (standard Lua convention)
            Ok(Value::String(lua.create_string(
                format!("\n\tno embedded module '{modname}'").as_str(),
            )?))
        }
    })?;

    // Append our custom loader to the searchers list. `raw_insert` takes a
    // Lua `Integer` (i64), while `raw_len` returns `usize`, so convert first.
    let len = i64::try_from(searchers.raw_len()).unwrap_or(i64::MAX);
    searchers.raw_insert(len.saturating_add(1), custom_loader)?;

    // Add `praxis` as a global variable in all scripts.
    let praxis = require_module(lua, "praxis")?;
    lua.globals().set("praxis", praxis)?;

    Ok(())
}

/// Creates the Lua state, loads the embedded library, wires logging, and runs
/// `script`.
///
/// # Errors
///
/// Returns [`mlua::Error`] if library setup fails or the script fails to run.
pub(super) fn build_lua(script: &str) -> Result<Lua> {
    let lua = Lua::new();
    load_library(&lua)?;
    wire_logging(&lua)?;
    lua.load(script).exec()?;
    Ok(lua)
}

/// Returns the filter table the script registered in `module`, substituting an
/// empty table when the script registered nothing (or called `set(nil)`).
///
/// # Errors
///
/// Returns [`mlua::Error`] if `module` cannot be loaded or has no `get`
/// function.
pub(super) fn get_registered(lua: &Lua, module: &str) -> Result<Table> {
    let get = get_function(lua, module, "get")?;
    get.call::<Option<Table>>(())?.map_or_else(|| lua.create_table(), Ok)
}

/// Returns whether `filter` defines the function `name`, following its
/// metatable so inherited methods count as defined.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the lookup fails.
pub(super) fn has_hook(filter: &Table, name: &str) -> Result<bool> {
    Ok(matches!(filter.get::<Value>(name)?, Value::Function(_)))
}

/// Returns `filter`'s function `name`, following its metatable, or `None` when
/// the filter does not define it.
///
/// # Errors
///
/// Returns [`mlua::Error`] if the lookup fails.
pub(super) fn method(filter: &Table, name: &str) -> Result<Option<Function>> {
    match filter.get::<Value>(name)? {
        Value::Function(function) => Ok(Some(function)),
        _ => Ok(None),
    }
}

/// Requires the Lua module `module` and returns its table.
///
/// # Errors
///
/// Returns [`mlua::Error`] if `require` is unavailable or `module` fails
/// to load.
pub(super) fn require_module(lua: &Lua, module: &str) -> Result<Table> {
    let require: Function = lua.globals().get("require")?;
    require.call(module)
}

/// Requires the Lua module `module` and returns the named function `fun_name`.
///
/// # Errors
///
/// Returns [`mlua::Error`] if `require` is unavailable, `module` fails to
/// load, or the module has no `fun_name` function.
pub(super) fn get_function(lua: &Lua, module: &str, fun_name: &str) -> Result<Function> {
    require_module(lua, module)?.get(fun_name)
}

/// Wire the Rust tracing macros into the Lua logging functions.
pub(super) fn wire_logging(lua: &Lua) -> Result<()> {
    let error_impl = lua.create_function(|_lua, (log, msg): (String, String)| {
        error!(target: LOG_TARGET, log = log, "{msg}");
        Ok(())
    })?;
    let warn_impl = lua.create_function(|_lua, (log, msg): (String, String)| {
        warn!(target: LOG_TARGET, log = log, "{msg}");
        Ok(())
    })?;
    let info_impl = lua.create_function(|_lua, (log, msg): (String, String)| {
        info!(target: LOG_TARGET, log = log, "{msg}");
        Ok(())
    })?;
    let debug_impl = lua.create_function(|_lua, (log, msg): (String, String)| {
        debug!(target: LOG_TARGET, log = log, "{msg}");
        Ok(())
    })?;
    let trace_impl = lua.create_function(|_lua, (log, msg): (String, String)| {
        trace!(target: LOG_TARGET, log = log, "{msg}");
        Ok(())
    })?;

    lua.load(chunk! {
        local Log = require("praxis.log.Log")
        Log.error_impl = $error_impl
        Log.warn_impl = $warn_impl
        Log.info_impl = $info_impl
        Log.debug_impl = $debug_impl
        Log.trace_impl = $trace_impl
    })
    .exec()?;

    Ok(())
}

/// Converts the `praxis.filter.FilterAction` that `hook` returned into the
/// matching Rust [`FilterAction`]; a hook returning nothing maps to
/// [`FilterAction::Continue`].
///
/// A `reject` action's payload is an HTTP status code integer. The
/// `terminal_response` and `streaming_terminal_response` payloads have no Lua
/// type yet, so they are reported as errors rather than silently downgraded.
///
/// # Errors
///
/// Returns [`FilterError`] if reading the action's kind or rejection fails,
/// the rejection is not a valid HTTP status code, or the kind is unknown or
/// unsupported.
pub(super) fn convert_action(
    filter_name: &str,
    hook: &str,
    action: Option<Table>,
) -> std::result::Result<FilterAction, FilterError> {
    let Some(action) = action else {
        return Ok(FilterAction::Continue);
    };
    let kind: String = action.call_method("kind", ())?;
    match kind.as_str() {
        "continue" => Ok(FilterAction::Continue),
        "reject" => convert_rejection(filter_name, hook, &action).map(FilterAction::Reject),
        "release" => Ok(FilterAction::Release),
        "body_done" => Ok(FilterAction::BodyDone),
        _ => Err(FilterError::from(format!(
            "{filter_name}: {hook} returned unsupported FilterAction kind {kind:?}"
        ))),
    }
}

/// Converts a `reject` action's payload into a status-only [`Rejection`].
///
/// The payload is the Lua integer HTTP status code, matching
/// `FilterAction.reject(403)`.
///
/// # Errors
///
/// Returns [`FilterError`] if reading the payload fails, or the payload is
/// not an integer in the HTTP status range (100..=599).
fn convert_rejection(filter_name: &str, hook: &str, action: &Table) -> std::result::Result<Rejection, FilterError> {
    let value: Value = action.call_method("value", ())?;
    let Value::Integer(code) = value else {
        return Err(FilterError::from(format!(
            "{filter_name}: {hook} returned reject with a non-integer rejection"
        )));
    };
    let status = u16::try_from(code)
        .ok()
        .filter(|status| (100..=599).contains(status))
        .ok_or_else(|| {
            FilterError::from(format!(
                "{filter_name}: {hook} returned reject with invalid HTTP status code {code}"
            ))
        })?;
    Ok(Rejection::status(status))
}

/// Converts a `praxis.filter.http.BodyAccess` into [`BodyAccess`].
///
/// # Errors
///
/// Returns [`FilterError`] if reading the kind or requirement fails, or either
/// value is unknown.
pub(super) fn convert_body_access(access: &Table) -> std::result::Result<BodyAccess, FilterError> {
    let kind: String = access.call_method("kind", ())?;
    match kind.as_str() {
        "none" => Ok(BodyAccess::None),
        "read_only" | "read_write" => {
            let requirement = match access.call_method::<Option<String>>("requirement", ())?.as_deref() {
                None | Some("optional") => BodyRequirement::Optional,
                Some("required") => BodyRequirement::Required,
                Some(other) => {
                    return Err(FilterError::from(format!(
                        "unexpected BodyAccess requirement {other:?}"
                    )));
                },
            };
            if kind == "read_only" {
                Ok(BodyAccess::ReadOnly(requirement))
            } else {
                Ok(BodyAccess::ReadWrite(requirement))
            }
        },
        _ => Err(FilterError::from(format!("unexpected BodyAccess kind {kind:?}"))),
    }
}

/// Converts a `praxis.filter.http.BodyMode` into [`BodyMode`].
///
/// # Errors
///
/// Returns [`FilterError`] if reading the kind or byte limit fails, or either
/// value is unknown or out of range.
pub(super) fn convert_body_mode(mode: &Table) -> std::result::Result<BodyMode, FilterError> {
    let kind: String = mode.call_method("kind", ())?;
    match kind.as_str() {
        "stream" => Ok(BodyMode::Stream),
        "stream_buffer" => Ok(BodyMode::StreamBuffer {
            max_bytes: optional_usize(mode)?,
        }),
        "size_limit" => {
            let max_bytes = optional_usize(mode)?
                .ok_or_else(|| FilterError::from("BodyMode size_limit requires a non-negative integer max_bytes"))?;
            Ok(BodyMode::SizeLimit { max_bytes })
        },
        _ => Err(FilterError::from(format!("unexpected BodyMode kind {kind:?}"))),
    }
}

/// Converts a `u64` into a Lua integer, saturating at `i64::MAX`.
///
/// Lua 5.5 integers are signed 64-bit; byte counters cannot realistically
/// reach the boundary, but saturating keeps the conversion total.
pub(super) fn lua_integer(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Converts a `usize` into a Lua integer, saturating at `i64::MAX`.
pub(super) fn lua_integer_usize(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Reads the mode's optional `max_bytes` accessor as a `usize`.
///
/// # Errors
///
/// Returns [`FilterError`] if the accessor fails or the value is negative or
/// too large.
fn optional_usize(mode: &Table) -> std::result::Result<Option<usize>, FilterError> {
    match mode.call_method::<Option<i64>>("max_bytes", ())? {
        None => Ok(None),
        Some(value) => usize::try_from(value)
            .map(Some)
            .map_err(|error| FilterError::from(format!("invalid BodyMode max_bytes {value}: {error}"))),
    }
}
