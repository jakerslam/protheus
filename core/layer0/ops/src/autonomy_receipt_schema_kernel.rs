// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("autonomy-receipt-schema-kernel commands:");
    println!("  protheus-ops autonomy-receipt-schema-kernel to-success-criteria-record --payload-base64=<json>");
    println!("  protheus-ops autonomy-receipt-schema-kernel with-success-criteria-verification --payload-base64=<json>");
    println!(
        "  protheus-ops autonomy-receipt-schema-kernel normalize-receipt --payload-base64=<json>"
    );
    println!("  protheus-ops autonomy-receipt-schema-kernel success-criteria-from-receipt --payload-base64=<json>");
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

fn with_success_criteria_verification(
    base_verification: Option<&Value>,
    success_criteria: Option<&Value>,
    options: &Map<String, Value>,
) -> Value {
    let mut base = base_verification
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let fallback = options
        .get("fallback")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let criteria = to_success_criteria_record(success_criteria, &fallback);
    let criteria_obj = criteria.as_object().cloned().unwrap_or_default();
    let criteria_pass = if criteria_obj
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        criteria_obj
            .get("passed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || criteria_obj
                .get("deferred_pending")
                .and_then(Value::as_bool)
                .unwrap_or(false)
    } else {
        true
    };
    let mut checks = base
        .get("checks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut replaced = false;
    for row in &mut checks {
        if row.get("name").and_then(Value::as_str) == Some("success_criteria_met") {
            *row = json!({ "name": "success_criteria_met", "pass": criteria_pass });
            replaced = true;
            break;
        }
    }
    if !replaced {
        checks.push(json!({ "name": "success_criteria_met", "pass": criteria_pass }));
    }
    let mut failed = base
        .get("failed")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let token = short_text(Some(&row), 80);
            if token.is_empty() {
                None
            } else {
                Some(Value::String(token))
            }
        })
        .collect::<Vec<_>>();
    let already = failed
        .iter()
        .any(|row| row.as_str() == Some("success_criteria_met"));
    if criteria_pass {
        failed.retain(|row| row.as_str() != Some("success_criteria_met"));
    } else if !already {
        failed.push(Value::String("success_criteria_met".to_string()));
    }
    let passed = failed.is_empty();
    let mut outcome = short_text(base.get("outcome"), 80);
    if outcome.is_empty() {
        outcome = if passed {
            "shipped".to_string()
        } else {
            "no_change".to_string()
        };
    }
    if !criteria_pass
        && options
            .get("enforceNoChangeOnFailure")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        && outcome == "shipped"
    {
        outcome = "no_change".to_string();
    }
    let primary_failure = if !criteria_pass {
        criteria_obj
            .get("primary_failure")
            .and_then(Value::as_str)
            .map(|v| v.to_string())
            .filter(|v| !v.is_empty())
            .or_else(|| {
                let existing = short_text(base.get("primary_failure"), 180);
                if existing.is_empty() {
                    None
                } else {
                    Some(existing)
                }
            })
            .unwrap_or_else(|| "success_criteria_failed".to_string())
    } else {
        let existing = short_text(base.get("primary_failure"), 180);
        existing
    };
    base.insert("checks".to_string(), Value::Array(checks));
    base.insert("failed".to_string(), Value::Array(failed));
    base.insert("passed".to_string(), Value::Bool(passed));
    base.insert("outcome".to_string(), Value::String(outcome));
    base.insert(
        "primary_failure".to_string(),
        if primary_failure.is_empty() {
            Value::Null
        } else {
            Value::String(primary_failure)
        },
    );
    base.insert("success_criteria".to_string(), criteria);
    Value::Object(base)
}

fn normalize_autonomy_receipt_for_write(receipt: Option<&Value>) -> Value {
    let mut src = receipt
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let intent = src
        .get("intent")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let verification_src = src
        .get("verification")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut checks = verification_src
        .get("checks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let name = short_text(row.get("name"), 80);
            if name.is_empty() {
                None
            } else {
                Some(json!({
                    "name": name,
                    "pass": row.get("pass").and_then(Value::as_bool).unwrap_or(false)
                }))
            }
        })
        .collect::<Vec<_>>();
    let mut failed_set = std::collections::BTreeSet::new();
    for row in verification_src
        .get("failed")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let token = short_text(Some(&row), 80);
        if !token.is_empty() {
            failed_set.insert(token);
        }
    }

    let policy = intent
        .get("success_criteria_policy")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let required = policy
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let min_count = {
        let raw = clamp_count(policy.get("min_count"));
        if raw > 0 {
            raw
        } else if required {
            1
        } else {
            0
        }
    };
    let success_idx = checks
        .iter()
        .position(|row| row.get("name").and_then(Value::as_str) == Some("success_criteria_met"));
    let success_check_pass = success_idx
        .and_then(|idx| checks.get(idx))
        .and_then(|row| row.get("pass"))
        .and_then(Value::as_bool);
    let criteria_in = verification_src.get("success_criteria");
    let criteria = if criteria_in.and_then(Value::as_object).is_some() {
        to_success_criteria_record(
            criteria_in,
            &Map::from_iter([
                ("required".to_string(), Value::Bool(required)),
                ("min_count".to_string(), Value::from(min_count)),
            ]),
        )
    } else {
        synthesize_success_criteria(required, min_count, success_check_pass)
    };
    let criteria_obj = criteria.as_object().cloned().unwrap_or_default();
    let criteria_pass = if criteria_obj
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        criteria_obj
            .get("passed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || criteria_obj
                .get("deferred_pending")
                .and_then(Value::as_bool)
                .unwrap_or(false)
    } else {
        true
    };
    if let Some(idx) = success_idx {
        checks[idx] = json!({ "name": "success_criteria_met", "pass": criteria_pass });
    } else {
        checks.push(json!({ "name": "success_criteria_met", "pass": criteria_pass }));
    }
    if criteria_pass {
        failed_set.remove("success_criteria_met");
    } else {
        failed_set.insert("success_criteria_met".to_string());
    }
    let primary_failure_raw = if !criteria_pass {
        let from_criteria = short_text(criteria_obj.get("primary_failure"), 180);
        if !from_criteria.is_empty() {
            from_criteria
        } else {
            let from_verification = short_text(verification_src.get("primary_failure"), 180);
            if !from_verification.is_empty() {
                from_verification
            } else {
                "success_criteria_failed".to_string()
            }
        }
    } else {
        short_text(verification_src.get("primary_failure"), 180)
    };
    let mut reasons = failed_set
        .iter()
        .cloned()
        .map(Value::String)
        .collect::<Vec<_>>();
    if !primary_failure_raw.is_empty() {
        reasons.push(Value::String(primary_failure_raw.clone()));
    }
    let taxonomy = normalize_reason_list(&reasons);
    let passed = failed_set.is_empty();
    let normalized_verification = json!({
        "checks": checks,
        "failed": failed_set.into_iter().collect::<Vec<_>>(),
        "passed": passed,
        "primary_failure": if primary_failure_raw.is_empty() { Value::Null } else { Value::String(primary_failure_raw.clone()) },
        "primary_failure_taxonomy": taxonomy.first().cloned(),
        "failed_reason_taxonomy": taxonomy,
        "success_criteria": criteria,
    });
    src.insert("verification".to_string(), normalized_verification);
    Value::Object(src)
}

fn success_criteria_from_receipt(receipt: Option<&Value>) -> Value {
    let normalized = normalize_autonomy_receipt_for_write(receipt);
    normalized
        .get("verification")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("success_criteria"))
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    match cmd.as_str() {
        "help" | "--help" | "-h" => {
            usage();
            0
        }
        other => {
            let payload = match payload_json(argv) {
                Ok(value) => value,
                Err(err) => {
                    print_json_line(&cli_error("autonomy_receipt_schema_kernel", &err));
                    return 1;
                }
            };
            let obj = payload_obj(&payload);
            let out = match other {
                "to-success-criteria-record" => {
                    let fallback = obj
                        .get("fallback")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    json!({ "record": to_success_criteria_record(obj.get("criteria"), &fallback) })
                }
                "with-success-criteria-verification" => {
                    let options = obj
                        .get("options")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    json!({ "verification": with_success_criteria_verification(obj.get("baseVerification").or_else(|| obj.get("base_verification")), obj.get("successCriteria").or_else(|| obj.get("success_criteria")), &options) })
                }
                "normalize-receipt" => {
                    json!({ "receipt": normalize_autonomy_receipt_for_write(obj.get("receipt")) })
                }
                "success-criteria-from-receipt" => {
                    json!({ "success_criteria": success_criteria_from_receipt(obj.get("receipt")) })
                }
                _ => {
                    usage();
                    print_json_line(&cli_error(
                        "autonomy_receipt_schema_kernel",
                        "unknown_command",
                    ));
                    return 1;
                }
            };
            print_json_line(&cli_receipt(
                &format!("autonomy_receipt_schema_kernel_{other}"),
                out,
            ));
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_criteria_record_uses_fallbacks() {
        let out = to_success_criteria_record(
            None,
            &Map::from_iter([
                ("required".to_string(), Value::Bool(true)),
                ("min_count".to_string(), Value::from(2)),
            ]),
        );
        assert_eq!(out.get("required").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("min_count").and_then(Value::as_i64), Some(2));
    }

    #[test]
    fn normalize_receipt_synthesizes_missing_success_criteria() {
        let out = normalize_autonomy_receipt_for_write(Some(&json!({
            "intent": { "success_criteria_policy": { "required": true, "min_count": 1 } },
            "verification": { "checks": [], "failed": [] }
        })));
        let criteria = out
            .get("verification")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("success_criteria"))
            .cloned()
            .unwrap_or(Value::Null);
        assert_eq!(
            criteria.get("synthesized").and_then(Value::as_bool),
            Some(true)
        );
    }
}
