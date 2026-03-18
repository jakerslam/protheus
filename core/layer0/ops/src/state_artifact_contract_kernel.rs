// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("state-artifact-contract-kernel commands:");
    println!("  protheus-ops state-artifact-contract-kernel now-iso");
    println!("  protheus-ops state-artifact-contract-kernel decorate-artifact-row [--payload-base64=<json>]");
    println!(
        "  protheus-ops state-artifact-contract-kernel trim-jsonl-rows [--payload-base64=<json>]"
    );
    println!("  protheus-ops state-artifact-contract-kernel write-artifact-set [--payload-base64=<json>]");
    println!("  protheus-ops state-artifact-contract-kernel append-artifact-history [--payload-base64=<json>]");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out =
        json!({"ok": ok, "type": kind, "ts": ts, "date": ts[..10].to_string(), "payload": payload});
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({"ok": false, "type": kind, "ts": ts, "date": ts[..10].to_string(), "error": error, "fail_closed": true});
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("state_artifact_contract_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("state_artifact_contract_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("state_artifact_contract_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("state_artifact_contract_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    match value {
        Some(Value::String(v)) => v
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(max_len)
            .collect(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other
            .to_string()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(max_len)
            .collect(),
    }
}

fn parse_usize(value: Option<&Value>, fallback: usize) -> usize {
    match value {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(fallback as u64) as usize,
        Some(Value::String(v)) => v.trim().parse::<usize>().unwrap_or(fallback),
        _ => fallback,
    }
}

fn resolve_path(root: &Path, value: Option<&Value>) -> Option<PathBuf> {
    let raw = clean_text(value, 2048);
    if raw.is_empty() {
        return None;
    }
    let candidate = PathBuf::from(&raw);
    if candidate.is_absolute() {
        Some(candidate)
    } else {
        Some(root.join(candidate))
    }
}

fn decorate_artifact_row(payload: &Map<String, Value>, options: &Map<String, Value>) -> Value {
    let mut out = payload.clone();
    let schema_id = clean_text(
        options
            .get("schemaId")
            .or_else(|| options.get("schema_id"))
            .or_else(|| payload.get("schema_id")),
        120,
    );
    let schema_version = clean_text(
        options
            .get("schemaVersion")
            .or_else(|| options.get("schema_version"))
            .or_else(|| payload.get("schema_version")),
        40,
    );
    let artifact_type = clean_text(
        options
            .get("artifactType")
            .or_else(|| options.get("artifact_type"))
            .or_else(|| payload.get("artifact_type")),
        80,
    );
    out.insert(
        "schema_id".to_string(),
        Value::String(if schema_id.is_empty() {
            "state_artifact_row".to_string()
        } else {
            schema_id
        }),
    );
    out.insert(
        "schema_version".to_string(),
        Value::String(if schema_version.is_empty() {
            "1.0".to_string()
        } else {
            schema_version
        }),
    );
    out.insert(
        "artifact_type".to_string(),
        Value::String(if artifact_type.is_empty() {
            "receipt".to_string()
        } else {
            artifact_type
        }),
    );
    let ts = clean_text(payload.get("ts"), 48);
    out.insert(
        "ts".to_string(),
        Value::String(if ts.is_empty() { now_iso() } else { ts }),
    );
    Value::Object(out)
}

fn trim_jsonl_rows(file_path: &Path, max_rows: usize) -> Result<(), String> {
    if max_rows == 0 || !file_path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(file_path)
        .map_err(|err| format!("state_artifact_contract_kernel_read_failed:{err}"))?;
    let rows = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if rows.len() <= max_rows {
        return Ok(());
    }
    let kept = rows[rows.len() - max_rows..].join("\n") + "\n";
    fs::write(file_path, kept)
        .map_err(|err| format!("state_artifact_contract_kernel_trim_failed:{err}"))
}

fn write_artifact_set(
    root: &Path,
    payload: &Map<String, Value>,
    options: &Map<String, Value>,
) -> Result<Value, String> {
    let paths = payload
        .get("paths")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let row = decorate_artifact_row(
        payload
            .get("payload")
            .or_else(|| payload.get("row"))
            .and_then(Value::as_object)
            .unwrap_or(payload),
        options,
    );
    if options
        .get("writeLatest")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        if let Some(latest_path) = resolve_path(
            root,
            paths.get("latestPath").or_else(|| paths.get("latest_path")),
        ) {
            lane_utils::write_json(&latest_path, &row)?;
        }
    }
    if options
        .get("appendReceipt")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        if let Some(receipts_path) = resolve_path(
            root,
            paths
                .get("receiptsPath")
                .or_else(|| paths.get("receipts_path")),
        ) {
            lane_utils::append_jsonl(&receipts_path, &row)?;
            let max_receipt_rows = parse_usize(
                options
                    .get("maxReceiptRows")
                    .or_else(|| options.get("max_receipt_rows")),
                0,
            );
            if max_receipt_rows > 0 {
                trim_jsonl_rows(&receipts_path, max_receipt_rows)?;
            }
        }
    }
    if let Some(history_path) = resolve_path(
        root,
        paths
            .get("historyPath")
            .or_else(|| paths.get("history_path")),
    ) {
        lane_utils::append_jsonl(&history_path, &row)?;
    }
    Ok(row)
}

fn append_artifact_history(
    root: &Path,
    payload: &Map<String, Value>,
    options: &Map<String, Value>,
) -> Result<Value, String> {
    let row = decorate_artifact_row(
        payload
            .get("payload")
            .or_else(|| payload.get("row"))
            .and_then(Value::as_object)
            .unwrap_or(payload),
        options,
    );
    let history_path = resolve_path(
        root,
        payload
            .get("historyPath")
            .or_else(|| payload.get("history_path")),
    )
    .ok_or_else(|| "state_artifact_contract_kernel_missing_history_path".to_string())?;
    lane_utils::append_jsonl(&history_path, &row)?;
    Ok(row)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
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
            print_json_line(&cli_error("state_artifact_contract_kernel_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let options = input
        .get("options")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let result = match command.as_str() {
        "now-iso" => cli_receipt(
            "state_artifact_contract_kernel_now_iso",
            json!({ "ok": true, "ts": now_iso() }),
        ),
        "decorate-artifact-row" => cli_receipt(
            "state_artifact_contract_kernel_decorate_artifact_row",
            json!({
                "ok": true,
                "row": decorate_artifact_row(
                    input.get("payload").or_else(|| input.get("row")).and_then(Value::as_object).unwrap_or(input),
                    &options,
                )
            }),
        ),
        "trim-jsonl-rows" => match resolve_path(
            root,
            input.get("filePath").or_else(|| input.get("file_path")),
        ) {
            Some(file_path) => match trim_jsonl_rows(
                &file_path,
                parse_usize(input.get("maxRows").or_else(|| input.get("max_rows")), 0),
            ) {
                Ok(()) => cli_receipt(
                    "state_artifact_contract_kernel_trim_jsonl_rows",
                    json!({ "ok": true, "file_path": file_path }),
                ),
                Err(err) => cli_error("state_artifact_contract_kernel_error", &err),
            },
            None => cli_error(
                "state_artifact_contract_kernel_error",
                "state_artifact_contract_kernel_missing_file_path",
            ),
        },
        "write-artifact-set" => match write_artifact_set(root, input, &options) {
            Ok(row) => cli_receipt(
                "state_artifact_contract_kernel_write_artifact_set",
                json!({ "ok": true, "row": row }),
            ),
            Err(err) => cli_error("state_artifact_contract_kernel_error", &err),
        },
        "append-artifact-history" => match append_artifact_history(root, input, &options) {
            Ok(row) => cli_receipt(
                "state_artifact_contract_kernel_append_artifact_history",
                json!({ "ok": true, "row": row }),
            ),
            Err(err) => cli_error("state_artifact_contract_kernel_error", &err),
        },
        _ => cli_error(
            "state_artifact_contract_kernel_error",
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
