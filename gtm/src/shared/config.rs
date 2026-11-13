// Copyright (c) 2026
// Helpers for reading shared keys from the TOML config file.
//
// The client and daemon both consume `config.toml` but parse it differently
// (serde structs vs ad-hoc lookups). These helpers centralize the behavior for
// keys that must stay compatible across both sides.

use std::net::SocketAddr;

/// Default loopback port for the daemon status endpoint.
pub const WEB_PORT: u16 = 8991;

/// Read a Discord application id from `config.toml`.
///
/// Accepts `discord_app_id` (daemon-facing key) and `discord_id` (client prefs
/// key) for compatibility. Returns `None` for missing/blank/invalid values.
pub fn discord_id(toml: Option<&toml::Value>) -> Option<String> {
    let raw = match toml.and_then(|v| v.get("discord_app_id").or_else(|| v.get("discord_id")))? {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        _ => return None,
    };
    let id = raw.trim();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        eprintln!("gtmd: ignoring discord_app_id {id:?}: expected a numeric application id");
        return None;
    }
    Some(id.to_string())
}

/// Resolve the daemon status endpoint address from `config.toml`.
///
/// Absent means on (loopback). An empty string means off. A bare port means
/// loopback on that port. Invalid values fall back to loopback default.
pub fn web_addr(toml: Option<&toml::Value>) -> Option<SocketAddr> {
    let raw = toml.and_then(|v| v.get("web_addr")).map(|v| {
        if let Some(s) = v.as_str() {
            s.to_string()
        } else if let Some(i) = v.as_integer() {
            i.to_string()
        } else {
            String::new()
        }
    });
    let addr = match raw {
        Some(s) if s.trim().is_empty() => return None,
        Some(s) => s,
        None => format!("127.0.0.1:{WEB_PORT}"),
    };
    let addr = if addr.chars().all(|c| c.is_ascii_digit()) {
        format!("127.0.0.1:{addr}")
    } else {
        addr
    };
    match addr.parse() {
        Ok(a) => Some(a),
        Err(e) => {
            eprintln!("gtmd: ignoring web_addr {addr:?}: {e}");
            Some(
                format!("127.0.0.1:{WEB_PORT}")
                    .parse()
                    .expect("loopback addr"),
            )
        }
    }
}

/// Read a boolean config key, defaulting when absent/invalid.
pub fn bool_or(toml: Option<&toml::Value>, key: &str, default: bool) -> bool {
    toml.and_then(|v| v.get(key))
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}
