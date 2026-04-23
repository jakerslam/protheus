// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

fn usage() {
    println!("autonomy-receipt-schema-kernel commands:");
    println!("  infring-ops autonomy-receipt-schema-kernel to-success-criteria-record --payload-base64=<json>");
    println!("  infring-ops autonomy-receipt-schema-kernel with-success-criteria-verification --payload-base64=<json>");
    println!(
        "  infring-ops autonomy-receipt-schema-kernel normalize-receipt --payload-base64=<json>"
    );
    println!("  infring-ops autonomy-receipt-schema-kernel success-criteria-from-receipt --payload-base64=<json>");
}

fn receipt_envelope(kind: &str, ok: bool) -> Value {
    let ts = now_iso();
    json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string()
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
            .map_err(|err| format!("autonomy_receipt_schema_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("autonomy_receipt_schema_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("autonomy_receipt_schema_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("autonomy_receipt_schema_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn short_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = match value {
        Some(Value::String(v)) => v.split_whitespace().collect::<Vec<_>>().join(" "),
        Some(Value::Null) | None => String::new(),
        Some(other) => other
            .to_string()
            .trim_matches('"')
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    };
    out = out.trim().to_string();
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn normalize_reason_token(value: Option<&Value>) -> String {
    let raw = short_text(value, 180).to_ascii_lowercase();
    if raw.is_empty() {
        return String::new();
    }
    let compact = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if compact.is_empty() {
        return String::new();
    }
    if compact.contains("gate_manual") {
        return "route_gate_manual".to_string();
    }
    if compact.contains("gate_deny") {
        return "route_gate_deny".to_string();
    }
    if compact.contains("not_executable") {
        return "route_not_executable".to_string();
    }
    if compact.contains("preflight_executable") {
        return "preflight_not_executable".to_string();
    }
    if compact.contains("pre_exec_criteria_gate") {
        return "pre_exec_criteria_gate_failed".to_string();
    }
    if compact.contains("queue_accept_logged") {
        return "queue_accept_not_logged".to_string();
    }
    if compact.contains("deferred_pending_window") {
        return "deferred_pending_window".to_string();
    }
    if compact.contains("success_criteria") {
        return "success_criteria_failed".to_string();
    }
    if compact.contains("postcheck_fail") {
        return "postcheck_failed".to_string();
    }
    if compact.contains("adapter_") && compact.contains("unverified") {
        return "actuation_unverified".to_string();
    }
    if compact.contains("actuation") && compact.contains("exit_") {
        return "actuation_execution_failed".to_string();
    }
    if compact.contains("route") && compact.contains("exit_") {
        return "route_execution_failed".to_string();
    }
    if compact.contains("exec_failed") || compact.contains("command_failed") {
        return "execution_failed".to_string();
    }
    compact.chars().take(80).collect::<String>()
}

fn normalize_reason_list(values: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        let token = normalize_reason_token(Some(value));
        if token.is_empty() || !seen.insert(token.clone()) {
            continue;
        }
        out.push(token);
    }
    out
}

fn clamp_count(value: Option<&Value>) -> i64 {
    let n = value
        .and_then(Value::as_i64)
        .or_else(|| value.and_then(Value::as_f64).map(|v| v as i64))
        .unwrap_or(0);
    n.max(0)
}

fn to_success_criteria_record(criteria: Option<&Value>, fallback: &Map<String, Value>) -> Value {
    let src = criteria
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let required_fallback = fallback
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let min_count_fallback = clamp_count(fallback.get("min_count"));
    let checks = src
        .get("checks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .take(12)
        .collect::<Vec<_>>();
    let min_count = {
        let raw = clamp_count(src.get("min_count"));
        if raw > 0 {
            raw
        } else {
            min_count_fallback
        }
    };
    let hard_failed_count = {
        let hard = clamp_count(src.get("hard_failed_count"));
        if hard > 0 {
            hard
        } else {
            clamp_count(src.get("failed_count"))
        }
    };
    json!({
        "required": src.get("required").and_then(Value::as_bool).unwrap_or(false) || required_fallback,
        "min_count": min_count,
        "total_count": clamp_count(src.get("total_count")),
        "evaluated_count": clamp_count(src.get("evaluated_count")),
        "passed_count": clamp_count(src.get("passed_count")),
        "failed_count": clamp_count(src.get("failed_count")),
        "hard_failed_count": hard_failed_count,
        "unknown_count": clamp_count(src.get("unknown_count")),
        "deferred_count": clamp_count(src.get("deferred_count")),
        "deferred_pending": src.get("deferred_pending").and_then(Value::as_bool).unwrap_or(false),
        "pass_rate": src.get("pass_rate").and_then(Value::as_f64).map(Value::from).unwrap_or(Value::Null),
        "passed": src.get("passed").and_then(Value::as_bool).unwrap_or(false),
        "primary_failure": src.get("primary_failure").map(|v| Value::String(short_text(Some(v), 180))).unwrap_or(Value::Null),
        "checks": checks,
        "synthesized": src.get("synthesized").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn synthesize_success_criteria(required: bool, min_count: i64, check_pass: Option<bool>) -> Value {
    let resolved_pass = match check_pass {
        Some(true) => true,
        Some(false) => false,
        None => !required,
    };
    let evaluated = if check_pass.is_some() { 1 } else { 0 };
    json!({
        "required": required,
        "min_count": if min_count > 0 { min_count } else if required { 1 } else { 0 },
        "total_count": 0,
        "evaluated_count": evaluated,
        "passed_count": if resolved_pass { 1 } else { 0 },
        "failed_count": if resolved_pass { 0 } else { 1 },
        "hard_failed_count": if resolved_pass { 0 } else { 1 },
        "unknown_count": if evaluated == 0 { 1 } else { 0 },
        "deferred_count": 0,
        "deferred_pending": false,
        "pass_rate": if evaluated > 0 { Value::from(if resolved_pass { 1.0 } else { 0.0 }) } else { Value::Null },
        "passed": resolved_pass,
        "primary_failure": if resolved_pass { Value::Null } else { Value::String("success_criteria_missing_in_receipt_pipeline".to_string()) },
        "checks": [],
        "synthesized": true,
    })
}
