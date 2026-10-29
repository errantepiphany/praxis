// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Lua API.

use mlua::{Function, Lua, Result, Table, Value, chunk};
use tracing::{debug, error, info, trace, warn};

/// Embed Lua api scripts directly into the binary at compile time
#[expect(clippy::too_many_lines, reason = "needed")]
pub(super) fn load_library(lua: &Lua) -> Result<()> {
    /// Lua source of the MIT-licensed `inspect` library, which renders any Lua value as a readable string.
    /// <https://github.com/kikito/inspect.lua>
    const INSPECT: &str = include_str!("lib/inspect.lua");

    const PRAXIS: &str = include_str!("lib/praxis/init.lua");

    const PRAXIS_FILTER: &str = include_str!("lib/praxis/filter/init.lua");
    const PRAXIS_FILTER_ACTION: &str = include_str!("lib/praxis/filter/FilterAction.lua");
    const PRAXIS_FILTER_HTTP: &str = include_str!("lib/praxis/filter/http/init.lua");
    const PRAXIS_FILTER_HTTP_BODY: &str = include_str!("lib/praxis/filter/http/HttpBody.lua");
    const PRAXIS_FILTER_HTTP_CONTEXT: &str = include_str!("lib/praxis/filter/http/HttpContext.lua");
    const PRAXIS_FILTER_HTTP_FILTER: &str = include_str!("lib/praxis/filter/http/HttpFilter.lua");
    const PRAXIS_FILTER_HTTP_MESSAGE: &str = include_str!("lib/praxis/filter/http/HttpMessage.lua");
    const PRAXIS_FILTER_HTTP_REQUEST: &str = include_str!("lib/praxis/filter/http/HttpRequest.lua");
    const PRAXIS_FILTER_HTTP_RESPONSE: &str = include_str!("lib/praxis/filter/http/HttpResponse.lua");

    const PRAXIS_LOG: &str = include_str!("lib/praxis/log/init.lua");
    const PRAXIS_LOG_LOG: &str = include_str!("lib/praxis/log/Log.lua");

    const PRAXIS_NET: &str = include_str!("lib/praxis/net/init.lua");
    const PRAXIS_NET_URI: &str = include_str!("lib/praxis/net/Uri.lua");

    const PRAXIS_UTIL: &str = include_str!("lib/praxis/util/init.lua");
    const PRAXIS_UTIL_OBJECT: &str = include_str!("lib/praxis/util/Object.lua");

    // Access the package table and searchers array
    let package: Table = lua.globals().get("package")?;

    // In Lua 5.2+, this is "searchers". In Lua 5.1 / LuaJIT, use "loaders".
    let searchers: Table = package.get("searchers")?;

    // Create a custom loader function
    let custom_loader = lua.create_function(|lua, modname: String| {
        let code = match modname.as_str() {
            "inspect" => Some(INSPECT),
            "praxis" => Some(PRAXIS),
            "praxis.filter" => Some(PRAXIS_FILTER),
            "praxis.filter.FilterAction" => Some(PRAXIS_FILTER_ACTION),
            "praxis.filter.http" => Some(PRAXIS_FILTER_HTTP),
            "praxis.filter.http.HttpBody" => Some(PRAXIS_FILTER_HTTP_BODY),
            "praxis.filter.http.HttpContext" => Some(PRAXIS_FILTER_HTTP_CONTEXT),
            "praxis.filter.http.HttpFilter" => Some(PRAXIS_FILTER_HTTP_FILTER),
            "praxis.filter.http.HttpMessage" => Some(PRAXIS_FILTER_HTTP_MESSAGE),
            "praxis.filter.http.HttpRequest" => Some(PRAXIS_FILTER_HTTP_REQUEST),
            "praxis.filter.http.HttpResponse" => Some(PRAXIS_FILTER_HTTP_RESPONSE),
            "praxis.log" => Some(PRAXIS_LOG),
            "praxis.log.Log" => Some(PRAXIS_LOG_LOG),
            "praxis.net" => Some(PRAXIS_NET),
            "praxis.net.Uri" => Some(PRAXIS_NET_URI),
            "praxis.util" => Some(PRAXIS_UTIL),
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

/// Requires the Lua module `module` and returns the named function.
///
/// # Errors
///
/// Returns [`mlua::Error`] if `require` is unavailable, `module` fails to
/// load, or the module has no `new` function.
pub(super) fn get_function(lua: &Lua, module: &str, fun_name: &str) -> Result<Function> {
    require_module(lua, module)?.get(fun_name)
}

/// Wire the Rust tracing macros into the Lua logging functions.
#[expect(clippy::too_many_lines, reason = "needed")]
pub(super) fn wire_logging(lua: &Lua) -> Result<()> {
    // have to pass this, otherwise it would be "praxis_filter::scripting::lua::api"
    const LOG_TARGET: &str = "praxis_filter::scripting::lua";
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
