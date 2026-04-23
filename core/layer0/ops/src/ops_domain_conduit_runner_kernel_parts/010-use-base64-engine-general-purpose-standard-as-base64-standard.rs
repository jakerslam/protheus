// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

fn usage() {
    println!("ops-domain-conduit-runner-kernel commands:");
    println!("  infring-ops ops-domain-conduit-runner-kernel parse-argv --payload-base64=<json>");
    println!(
        "  infring-ops ops-domain-conduit-runner-kernel build-pass-args --payload-base64=<json>"
    );
    println!("  infring-ops ops-domain-conduit-runner-kernel build-run-options [--payload-base64=<json>]");
    println!(
        "  infring-ops ops-domain-conduit-runner-kernel prepare-run [--payload-base64=<json>]"
    );
    println!("  infring-ops ops-domain-conduit-runner-kernel run --payload-base64=<json>");
    println!(
        "  infring-ops ops-domain-conduit-runner-kernel ipc-daemon [--queue-dir=<path>] [--poll-ms=<n>]"
    );
}

fn receipt_envelope(kind: &str, ok: bool) -> Value {
    let ts = now_iso();
    json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
    })
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw).map_err(|err| {
            format!("ops_domain_conduit_runner_kernel_payload_decode_failed:{err}")
        });
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("ops_domain_conduit_runner_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("ops_domain_conduit_runner_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text).map_err(|err| {
            format!("ops_domain_conduit_runner_kernel_payload_decode_failed:{err}")
        });
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = as_str(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn parse_bool_text(raw: &str, fallback: bool) -> bool {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn parse_i64_text(raw: &str, fallback: i64) -> i64 {
    raw.trim().parse::<i64>().ok().unwrap_or(fallback)
}

fn argv_list(payload: &Map<String, Value>) -> Vec<String> {
    payload
        .get("argv")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().map(|row| as_str(Some(row))).collect::<Vec<_>>())
        .unwrap_or_default()
}

fn parse_args_map(argv: &[String]) -> Map<String, Value> {
    let mut out = Map::new();
    out.insert("_".to_string(), Value::Array(Vec::new()));
    let mut index = 0usize;
    while index < argv.len() {
        let token = argv[index].trim().to_string();
        if !token.starts_with("--") {
            out.entry("_".to_string()).and_modify(|rows| {
                if let Value::Array(values) = rows {
                    values.push(Value::String(token.clone()));
                }
            });
            index += 1;
            continue;
        }
        if let Some((key, value)) = token.split_once('=') {
            out.insert(
                key.trim_start_matches("--").to_string(),
                Value::String(value.to_string()),
            );
            index += 1;
            continue;
        }
        let key = token.trim_start_matches("--").to_string();
        let next = argv.get(index + 1).cloned().unwrap_or_default();
        if !next.is_empty() && !next.starts_with("--") {
            out.insert(key, Value::String(next));
            index += 2;
            continue;
        }
        out.insert(key, Value::Bool(true));
        index += 1;
    }
    out
}

fn parsed_map(payload: &Map<String, Value>) -> Map<String, Value> {
    payload
        .get("parsed")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| parse_args_map(&argv_list(payload)))
}

fn parsed_positionals(parsed: &Map<String, Value>) -> Vec<String> {
    parsed
        .get("_")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().map(|row| as_str(Some(row))).collect::<Vec<_>>())
        .unwrap_or_default()
}

fn build_pass_args_vec(parsed: &Map<String, Value>) -> Vec<String> {
    let positional = parsed_positionals(parsed);
    let mut forwarded_flags = Vec::new();
    for (key, value) in parsed {
        if matches!(
            key.as_str(),
            "_" | "domain"
                | "run-context"
                | "skip-runtime-gate"
                | "stdio-timeout-ms"
                | "timeout-ms"
        ) {
            continue;
        }
        match value {
            Value::Bool(true) => forwarded_flags.push(format!("--{key}")),
            Value::Null | Value::Bool(false) => {}
            _ => forwarded_flags.push(format!("--{key}={}", as_str(Some(value)))),
        }
    }
    if !as_str(parsed.get("domain")).is_empty() {
        return positional.into_iter().chain(forwarded_flags).collect();
    }
    let mut args = if positional.is_empty() {
        Vec::new()
    } else {
        positional.into_iter().skip(1).collect::<Vec<_>>()
    };
    args.extend(forwarded_flags);
    args
}

fn build_run_options_value(parsed: &Map<String, Value>) -> Value {
    let skip_runtime_gate = parse_bool_text(
        as_str(parsed.get("skip-runtime-gate")).as_str(),
        parse_bool_text(
            std::env::var("INFRING_OPS_DOMAIN_SKIP_RUNTIME_GATE")
                .unwrap_or_else(|_| "true".to_string())
                .as_str(),
            true,
        ),
    );
    let stdio_timeout_ms = parse_i64_text(
        as_str(parsed.get("stdio-timeout-ms")).as_str(),
        parse_i64_text(
            std::env::var("INFRING_OPS_DOMAIN_STDIO_TIMEOUT_MS")
                .or_else(|_| std::env::var("INFRING_CONDUIT_STDIO_TIMEOUT_MS"))
                .unwrap_or_else(|_| "120000".to_string())
                .as_str(),
            120000,
        ),
    );
    let timeout_ms = parse_i64_text(
        as_str(parsed.get("timeout-ms")).as_str(),
        parse_i64_text(
            std::env::var("INFRING_OPS_DOMAIN_BRIDGE_TIMEOUT_MS")
                .or_else(|_| std::env::var("INFRING_CONDUIT_BRIDGE_TIMEOUT_MS"))
                .unwrap_or_else(|_| (stdio_timeout_ms + 1000).max(125000).to_string())
                .as_str(),
            (stdio_timeout_ms + 1000).max(125000),
        ),
    );
    let run_context = clean_text(parsed.get("run-context"), 120);
    json!({
        "runContext": if run_context.is_empty() { Value::Null } else { Value::String(run_context) },
        "skipRuntimeGate": skip_runtime_gate,
        "stdioTimeoutMs": stdio_timeout_ms,
        "timeoutMs": timeout_ms
    })
}

fn run_parse_argv(payload: &Map<String, Value>) -> Value {
    json!({
        "ok": true,
        "parsed": Value::Object(parse_args_map(&argv_list(payload)))
    })
}

fn run_build_pass_args(payload: &Map<String, Value>) -> Value {
    let parsed = parsed_map(payload);
    json!({
        "ok": true,
        "args": Value::Array(build_pass_args_vec(&parsed).into_iter().map(Value::String).collect())
    })
}
