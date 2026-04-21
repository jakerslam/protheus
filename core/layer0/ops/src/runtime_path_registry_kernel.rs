// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const CLIENT_LOCAL_ROOT: &str = "client/runtime/local";
const CLIENT_STATE_ROOT: &str = "client/runtime/local/state";
const CLIENT_INTERNAL_ROOT: &str = "client/runtime/local/internal";
const CORE_LOCAL_ROOT: &str = "core/local";
const CORE_STATE_ROOT: &str = "core/local/state";
const LEGACY_SURFACES: [&str; 3] = ["state", "client/runtime/state", "local"];

fn usage() {
    println!("runtime-path-registry-kernel commands:");
    println!("  protheus-ops runtime-path-registry-kernel constants");
    println!(
        "  protheus-ops runtime-path-registry-kernel normalize-for-root [--payload-base64=<json>]"
    );
    println!(
        "  protheus-ops runtime-path-registry-kernel resolve-canonical [--payload-base64=<json>]"
    );
    println!("  protheus-ops runtime-path-registry-kernel resolve-client-state [--payload-base64=<json>]");
    println!(
        "  protheus-ops runtime-path-registry-kernel resolve-core-state [--payload-base64=<json>]"
    );
}

fn receipt_date(ts: &str) -> String {
    ts.get(..10).unwrap_or("").to_string()
}

fn base_receipt(kind: &str, ok: bool, ts: &str) -> Value {
    json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": receipt_date(ts)
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
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("runtime_path_registry_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("runtime_path_registry_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("runtime_path_registry_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("runtime_path_registry_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn clean(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v
            .trim()
            .replace('\\', "/")
            .trim_start_matches("./")
            .trim_start_matches('/')
            .to_string(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other
            .to_string()
            .trim_matches('"')
            .trim()
            .replace('\\', "/")
            .trim_start_matches("./")
            .trim_start_matches('/')
            .to_string(),
    }
}

fn clean_root(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().replace('\\', "/"),
        Some(Value::Null) | None => String::new(),
        Some(other) => other
            .to_string()
            .trim_matches('"')
            .trim()
            .replace('\\', "/"),
    }
}

fn normalize_for_root(root_abs: &str, rel_path: &str) -> String {
    let root_name = Path::new(root_abs)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let root_clean = clean(Some(&Value::String(root_abs.to_string())));
    let rel = clean(Some(&Value::String(rel_path.to_string())));
    if rel.is_empty() {
        return rel;
    }
    if root_clean.ends_with("client/runtime") {
        if rel == "client/runtime" {
            return String::new();
        }
        if let Some(stripped) = rel.strip_prefix("client/runtime/") {
            return stripped.to_string();
        }
    }
    if root_name == "client" {
        if let Some(stripped) = rel.strip_prefix("client/") {
            return stripped.to_string();
        }
    }
    if root_name == "core" {
        if let Some(stripped) = rel.strip_prefix("core/") {
            return stripped.to_string();
        }
    }
    rel
}

fn resolve_canonical(root_abs: &str, rel_path: &str) -> String {
    let normalized = normalize_for_root(root_abs, rel_path);
    PathBuf::from(root_abs)
        .join(normalized)
        .to_string_lossy()
        .to_string()
}

pub fn run(_root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("runtime_path_registry_kernel_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let root_abs = clean_root(
        input
            .get("root_abs")
            .or_else(|| input.get("rootAbs"))
            .or_else(|| input.get("root")),
    );
    let rel_path = clean(
        input
            .get("rel_path")
            .or_else(|| input.get("relPath"))
            .or_else(|| input.get("path")),
    );
    let suffix = clean(input.get("suffix"));
    let result = match command.as_str() {
        "constants" => cli_receipt(
            "runtime_path_registry_kernel_constants",
            json!({
                "ok": true,
                "canonical_paths": {
                    "client_local_root": CLIENT_LOCAL_ROOT,
                    "client_state_root": CLIENT_STATE_ROOT,
                    "client_internal_root": CLIENT_INTERNAL_ROOT,
                    "core_local_root": CORE_LOCAL_ROOT,
                    "core_state_root": CORE_STATE_ROOT,
                },
                "legacy_surfaces": LEGACY_SURFACES,
            }),
        ),
        "normalize-for-root" => cli_receipt(
            "runtime_path_registry_kernel_normalize_for_root",
            json!({ "ok": true, "value": normalize_for_root(&root_abs, &rel_path) }),
        ),
        "resolve-canonical" => cli_receipt(
            "runtime_path_registry_kernel_resolve_canonical",
            json!({ "ok": true, "value": resolve_canonical(&root_abs, &rel_path) }),
        ),
        "resolve-client-state" => cli_receipt(
            "runtime_path_registry_kernel_resolve_client_state",
            json!({
                "ok": true,
                "value": resolve_canonical(&root_abs, &format!("{CLIENT_STATE_ROOT}/{}", suffix)),
            }),
        ),
        "resolve-core-state" => cli_receipt(
            "runtime_path_registry_kernel_resolve_core_state",
            json!({
                "ok": true,
                "value": resolve_canonical(&root_abs, &format!("{CORE_STATE_ROOT}/{}", suffix)),
            }),
        ),
        _ => cli_error(
            "runtime_path_registry_kernel_error",
            &format!("unknown_command:{command}"),
        ),
    };
    let exit = if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    };
    print_json_line(&result);
    exit
}
