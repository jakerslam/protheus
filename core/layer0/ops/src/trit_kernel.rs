// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const TRIT_PAIN: i64 = -1;
const TRIT_UNKNOWN: i64 = 0;
const TRIT_OK: i64 = 1;

fn usage() {
    println!("trit-kernel commands:");
    println!("  protheus-ops trit-kernel normalize --payload-base64=<json>");
    println!("  protheus-ops trit-kernel label --payload-base64=<json>");
    println!("  protheus-ops trit-kernel from-label --payload-base64=<json>");
    println!("  protheus-ops trit-kernel invert --payload-base64=<json>");
    println!("  protheus-ops trit-kernel majority --payload-base64=<json>");
    println!("  protheus-ops trit-kernel consensus --payload-base64=<json>");
    println!("  protheus-ops trit-kernel propagate --payload-base64=<json>");
    println!("  protheus-ops trit-kernel serialize --payload-base64=<json>");
    println!("  protheus-ops trit-kernel parse-serialized --payload-base64=<json>");
    println!("  protheus-ops trit-kernel serialize-vector --payload-base64=<json>");
    println!("  protheus-ops trit-kernel parse-vector --payload-base64=<json>");
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
            .map_err(|err| format!("trit_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("trit_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("trit_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("trit_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_text(value: &Value) -> String {
    match value {
        Value::String(v) => v.trim().to_ascii_lowercase(),
        Value::Null => String::new(),
        other => other
            .to_string()
            .trim_matches('"')
            .trim()
            .to_ascii_lowercase(),
    }
}

fn normalize_token_text(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in raw.trim().chars() {
        let next = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '_'
        };
        if next == '_' {
            if prev_sep {
                continue;
            }
            prev_sep = true;
        } else {
            prev_sep = false;
        }
        out.push(next);
    }
    out.trim_matches('_').to_string()
}

fn normalize_token(value: &Value) -> String {
    normalize_token_text(&as_text(value))
}

fn normalize_weight(value: Option<&Value>, fallback: f64) -> f64 {
    let raw = value.and_then(Value::as_f64).unwrap_or(fallback);
    if !raw.is_finite() || raw <= 0.0 {
        fallback
    } else {
        raw
    }
}

fn normalize_trit(value: &Value) -> i64 {
    match value {
        Value::Number(number) => {
            let raw = number.as_f64().unwrap_or(0.0);
            if raw > 0.0 {
                TRIT_OK
            } else if raw < 0.0 {
                TRIT_PAIN
            } else {
                TRIT_UNKNOWN
            }
        }
        Value::Bool(flag) => {
            if *flag {
                TRIT_OK
            } else {
                TRIT_PAIN
            }
        }
        _ => {
            let token = normalize_token(value);
            if matches!(
                token.as_str(),
                "ok" | "pass"
                    | "allow"
                    | "approved"
                    | "healthy"
                    | "up"
                    | "true"
                    | "success"
                    | "green"
                    | "ready"
            ) {
                TRIT_OK
            } else if matches!(
                token.as_str(),
                "pain"
                    | "fail"
                    | "failed"
                    | "error"
                    | "blocked"
                    | "deny"
                    | "denied"
                    | "critical"
                    | "false"
                    | "down"
                    | "red"
            ) {
                TRIT_PAIN
            } else {
                TRIT_UNKNOWN
            }
        }
    }
}

fn trit_label(value: i64) -> &'static str {
    match value {
        TRIT_PAIN => "pain",
        TRIT_OK => "ok",
        _ => "unknown",
    }
}

fn values_array(value: Option<&Value>) -> Vec<Value> {
    value.and_then(Value::as_array).cloned().unwrap_or_default()
}

fn majority_trit(values: &[Value], weights: &[Value], tie_breaker: &str) -> i64 {
    if values.is_empty() {
        return TRIT_UNKNOWN;
    }
    let mut pain = 0.0;
    let mut unknown = 0.0;
    let mut ok = 0.0;
    for (idx, row) in values.iter().enumerate() {
        let weight = normalize_weight(weights.get(idx), 1.0);
        match normalize_trit(row) {
            TRIT_PAIN => pain += weight,
            TRIT_OK => ok += weight,
            _ => unknown += weight,
        }
    }
    if pain > ok && pain > unknown {
        return TRIT_PAIN;
    }
    if ok > pain && ok > unknown {
        return TRIT_OK;
    }
    if unknown > pain && unknown > ok {
        return TRIT_UNKNOWN;
    }
    match normalize_token_text(tie_breaker).as_str() {
        "pain" => TRIT_PAIN,
        "ok" => TRIT_OK,
        "first_non_zero" => values
            .iter()
            .map(normalize_trit)
            .find(|row| *row != TRIT_UNKNOWN)
            .unwrap_or(TRIT_UNKNOWN),
        _ => TRIT_UNKNOWN,
    }
}

fn consensus_trit(values: &[Value]) -> i64 {
    if values.is_empty() {
        return TRIT_UNKNOWN;
    }
    let rows = values.iter().map(normalize_trit).collect::<Vec<_>>();
    let non_zero = rows
        .iter()
        .copied()
        .filter(|row| *row != TRIT_UNKNOWN)
        .collect::<Vec<_>>();
    if non_zero.is_empty() {
        return TRIT_UNKNOWN;
    }
    let has_pain = non_zero.iter().any(|row| *row == TRIT_PAIN);
    let has_ok = non_zero.iter().any(|row| *row == TRIT_OK);
    if has_pain && has_ok {
        TRIT_UNKNOWN
    } else if has_pain {
        TRIT_PAIN
    } else {
        TRIT_OK
    }
}

fn propagate_trit(parent: i64, child: i64, mode: &str) -> i64 {
    match mode {
        "strict" => {
            if parent == TRIT_PAIN || child == TRIT_PAIN {
                TRIT_PAIN
            } else if parent == TRIT_OK && child == TRIT_OK {
                TRIT_OK
            } else {
                TRIT_UNKNOWN
            }
        }
        "permissive" => {
            if parent == TRIT_OK || child == TRIT_OK {
                TRIT_OK
            } else if parent == TRIT_PAIN && child == TRIT_PAIN {
                TRIT_PAIN
            } else {
                TRIT_UNKNOWN
            }
        }
        _ => {
            if child == TRIT_PAIN {
                TRIT_PAIN
            } else if parent == TRIT_PAIN && child == TRIT_UNKNOWN {
                TRIT_PAIN
            } else if parent == TRIT_OK && child == TRIT_OK {
                TRIT_OK
            } else if parent == TRIT_UNKNOWN && child == TRIT_OK {
                TRIT_OK
            } else {
                TRIT_UNKNOWN
            }
        }
    }
}

fn serialize_trit(value: i64) -> &'static str {
    match value {
        TRIT_PAIN => "-1",
        TRIT_OK => "1",
        _ => "0",
    }
}

fn parse_serialized_trit(value: &Value) -> i64 {
    match normalize_token_text(&as_text(value)).as_str() {
        "-1" | "-" => TRIT_PAIN,
        "1" | "+" => TRIT_OK,
        "pain" | "fail" | "red" => TRIT_PAIN,
        "ok" | "pass" | "green" => TRIT_OK,
        "unknown" | "neutral" => TRIT_UNKNOWN,
        _ => TRIT_UNKNOWN,
    }
}

fn serialize_trit_vector(values: &[Value]) -> Value {
    let digits = values
        .iter()
        .map(|row| match normalize_trit(row) {
            TRIT_PAIN => "-",
            TRIT_OK => "+",
            _ => "0",
        })
        .collect::<Vec<_>>()
        .join("");
    json!({
        "schema_id": "balanced_trit_vector",
        "schema_version": "1.0.0",
        "encoding": "balanced_ternary_sign",
        "digits": digits,
        "values": values.iter().map(|row| serialize_trit(normalize_trit(row))).collect::<Vec<_>>()
    })
}

fn parse_trit_vector(payload: &Value) -> Vec<i64> {
    if let Some(rows) = payload.as_array() {
        return rows.iter().map(parse_serialized_trit).collect::<Vec<_>>();
    }
    let obj = payload.as_object().cloned().unwrap_or_default();
    if let Some(rows) = obj.get("values").and_then(Value::as_array) {
        return rows.iter().map(parse_serialized_trit).collect::<Vec<_>>();
    }
    let digits = obj
        .get("digits")
        .map(as_text)
        .unwrap_or_default()
        .chars()
        .filter(|ch| matches!(*ch, '+' | '-' | '0' | '1'))
        .collect::<String>();
    digits
        .chars()
        .map(|ch| parse_serialized_trit(&Value::String(ch.to_string())))
        .collect::<Vec<_>>()
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
                    print_json_line(&cli_error("trit_kernel", &err));
                    return 1;
                }
            };
            let obj = payload_obj(&payload);
            let out = match other {
                "normalize" => {
                    json!({ "trit": normalize_trit(obj.get("value").unwrap_or(&Value::Null)) })
                }
                "label" => {
                    let trit = normalize_trit(obj.get("value").unwrap_or(&Value::Null));
                    json!({ "label": trit_label(trit), "trit": trit })
                }
                "from-label" => {
                    json!({ "trit": normalize_trit(obj.get("value").unwrap_or(&Value::Null)) })
                }
                "invert" => {
                    let trit = normalize_trit(obj.get("value").unwrap_or(&Value::Null));
                    json!({ "trit": if trit == TRIT_PAIN { TRIT_OK } else if trit == TRIT_OK { TRIT_PAIN } else { TRIT_UNKNOWN } })
                }
                "majority" => {
                    let values = values_array(obj.get("values"));
                    let weights = values_array(obj.get("weights"));
                    let tie_breaker = obj
                        .get("tie_breaker")
                        .map(as_text)
                        .unwrap_or_else(|| "unknown".to_string());
                    json!({ "trit": majority_trit(&values, &weights, &tie_breaker) })
                }
                "consensus" => {
                    let values = values_array(obj.get("values"));
                    json!({ "trit": consensus_trit(&values) })
                }
                "propagate" => {
                    let parent = normalize_trit(obj.get("parent").unwrap_or(&Value::Null));
                    let child = normalize_trit(obj.get("child").unwrap_or(&Value::Null));
                    let mode = obj
                        .get("mode")
                        .map(as_text)
                        .unwrap_or_else(|| "cautious".to_string());
                    json!({ "trit": propagate_trit(parent, child, &mode) })
                }
                "serialize" => {
                    let trit = normalize_trit(obj.get("value").unwrap_or(&Value::Null));
                    json!({ "serialized": serialize_trit(trit) })
                }
                "parse-serialized" => {
                    json!({ "trit": parse_serialized_trit(obj.get("value").unwrap_or(&Value::Null)) })
                }
                "serialize-vector" => {
                    let values = values_array(obj.get("values"));
                    json!({ "vector": serialize_trit_vector(&values) })
                }
                "parse-vector" => {
                    let parsed = parse_trit_vector(obj.get("payload").unwrap_or(&Value::Null));
                    json!({ "values": parsed })
                }
                _ => {
                    usage();
                    print_json_line(&cli_error("trit_kernel", "unknown_command"));
                    return 1;
                }
            };
            print_json_line(&cli_receipt(&format!("trit_kernel_{other}"), out));
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn majority_and_consensus_match_expected() {
        let rows = vec![
            Value::String("ok".to_string()),
            Value::String("pain".to_string()),
            Value::String("ok".to_string()),
        ];
        assert_eq!(majority_trit(&rows, &[], "unknown"), TRIT_OK);
        assert_eq!(consensus_trit(&rows), TRIT_UNKNOWN);
    }

    #[test]
    fn serialize_and_parse_vector_roundtrip() {
        let vector = serialize_trit_vector(&[
            Value::String("pain".to_string()),
            Value::String("unknown".to_string()),
            Value::String("ok".to_string()),
        ]);
        assert_eq!(vector.get("digits").and_then(Value::as_str), Some("-0+"));
        assert_eq!(
            parse_trit_vector(&vector),
            vec![TRIT_PAIN, TRIT_UNKNOWN, TRIT_OK]
        );
    }
}
