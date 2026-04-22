// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use hex::decode as hex_decode;
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde_json::{json, Map, Value};
use sha2::Sha256;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

type HmacSha256 = Hmac<Sha256>;
fn usage() {
    println!("request-envelope-kernel commands:");
    println!("  protheus-ops request-envelope-kernel envelope-payload --payload-base64=<json>");
    println!("  protheus-ops request-envelope-kernel canonical-string --payload-base64=<json>");
    println!("  protheus-ops request-envelope-kernel sign --payload-base64=<json>");
    println!("  protheus-ops request-envelope-kernel verify --payload-base64=<json>");
    println!("  protheus-ops request-envelope-kernel stamp-env --payload-base64=<json>");
    println!("  protheus-ops request-envelope-kernel verify-from-env --payload-base64=<json>");
    println!("  protheus-ops request-envelope-kernel normalize-files --payload-base64=<json>");
    println!("  protheus-ops request-envelope-kernel normalize-key-id --payload-base64=<json>");
    println!("  protheus-ops request-envelope-kernel normalize-web-query --payload-base64=<json>");
    println!("  protheus-ops request-envelope-kernel normalize-web-date --payload-base64=<json>");
    println!(
        "  protheus-ops request-envelope-kernel normalize-web-freshness --payload-base64=<json>"
    );
    println!(
        "  protheus-ops request-envelope-kernel secret-key-env-var-name --payload-base64=<json>"
    );
}
fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
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
            .map_err(|err| format!("request_envelope_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("request_envelope_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("request_envelope_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("request_envelope_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}
fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}
fn as_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string().trim_matches('"').trim().to_string(),
    }
}
fn normalize_lower(value: Option<&Value>) -> String {
    as_text(value).to_ascii_lowercase()
}
fn normalize_key_id_text(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
        .take(40)
        .collect::<String>()
}
fn normalize_key_id(value: Option<&Value>) -> String {
    normalize_key_id_text(&as_text(value))
}
fn secret_key_env_var_name_text(kid: &str) -> String {
    let key_id = normalize_key_id_text(kid);
    if key_id.is_empty() {
        return String::new();
    }
    format!(
        "REQUEST_GATE_SECRET_{}",
        key_id
            .to_ascii_uppercase()
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect::<String>()
    )
}
fn normalize_web_provider(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "google" => "gemini".to_string(),
        "xai" => "grok".to_string(),
        "moonshot" => "kimi".to_string(),
        other => other.to_string(),
    }
}
fn normalize_web_date_to_iso(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .or_else(|_| chrono::NaiveDate::parse_from_str(trimmed, "%m/%d/%Y"))
        .ok()
        .map(|date| date.format("%Y-%m-%d").to_string())
}
fn iso_to_perplexity_date(raw: &str) -> Option<String> {
    use chrono::Datelike;
    chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
        .ok()
        .map(|date| format!("{}/{}/{}", date.month(), date.day(), date.year()))
}
fn normalize_web_freshness(raw: &str, provider: &str) -> Option<String> {
    let value = raw.trim().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    let provider = normalize_web_provider(provider);
    if provider == "brave" && value.contains("to") {
        let (lhs, rhs) = value.split_once("to")?;
        let start = normalize_web_date_to_iso(lhs)?;
        let end = normalize_web_date_to_iso(rhs)?;
        if start > end {
            return None;
        }
        return Some(format!("{start}to{end}"));
    }
    match provider.as_str() {
        "brave" => match value.as_str() {
            "pd" | "day" => Some("pd".to_string()),
            "pw" | "week" => Some("pw".to_string()),
            _ => None,
        },
        "perplexity" => match value.as_str() {
            "pd" | "day" => Some("day".to_string()),
            "pw" | "week" => Some("week".to_string()),
            _ => None,
        },
        _ => None,
    }
}
fn normalize_web_query_shape(input: &Map<String, Value>) -> Value {
    let query = as_text(input.get("query").or_else(|| input.get("q")));
    if query.is_empty() {
        return json!({ "ok": false, "error": "query_required" });
    }
    let domain = as_text(
        input
            .get("domain")
            .or_else(|| input.get("domain_hint"))
            .or_else(|| input.get("site")),
    );
    let provider = normalize_web_provider(&as_text(
        input
            .get("provider")
            .or_else(|| input.get("provider_hint"))
            .or_else(|| input.get("engine")),
    ));
    json!({
        "ok": true,
        "query": {
            "input": query,
            "sanitized": lane_utils::sanitize_web_tooling_query(&query),
            "canonical": lane_utils::canonicalize_web_tooling_query(&query, if domain.is_empty() { None } else { Some(domain.as_str()) })
        },
        "provider_hint": if provider.is_empty() { "auto" } else { provider.as_str() }
    })
}
fn normalize_files(value: Option<&Value>) -> Vec<String> {
    let mut out = value
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let normalized = as_text(Some(&row)).replace('\\', "/");
            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        })
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}
fn random_nonce() -> String {
    let mut bytes = [0_u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}
fn envelope_payload_map(input: &Map<String, Value>) -> Value {
    let source = normalize_lower(input.get("source"));
    let action = normalize_lower(input.get("action"));
    let ts_num = input
        .get("ts")
        .and_then(Value::as_i64)
        .or_else(|| {
            input
                .get("ts")
                .and_then(Value::as_f64)
                .map(|v| v.floor() as i64)
        })
        .or_else(|| as_text(input.get("ts")).parse::<i64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    let nonce = as_text(input.get("nonce"));
    let kid = normalize_key_id(input.get("kid"));
    json!({
        "source": if source.is_empty() { "local" } else { source.as_str() },
        "action": if action.is_empty() { "apply" } else { action.as_str() },
        "kid": kid,
        "ts": ts_num,
        "nonce": if nonce.is_empty() { random_nonce() } else { nonce },
        "files": normalize_files(input.get("files")),
    })
}
fn canonical_envelope_string(input: &Map<String, Value>) -> String {
    let payload = envelope_payload_map(input);
    let obj = payload.as_object().cloned().unwrap_or_default();
    let files = obj
        .get("files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| as_text(Some(&row)))
        .collect::<Vec<_>>()
        .join(",");
    [
        "v1".to_string(),
        format!("source={}", as_text(obj.get("source"))),
        format!("action={}", as_text(obj.get("action"))),
        format!("kid={}", as_text(obj.get("kid"))),
        format!("ts={}", obj.get("ts").and_then(Value::as_i64).unwrap_or(0)),
        format!("nonce={}", as_text(obj.get("nonce"))),
        format!("files={files}"),
    ]
    .join("|")
}
fn sign_envelope(input: &Map<String, Value>, secret: &str) -> String {
    if secret.trim().is_empty() {
        return String::new();
    }
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("valid hmac key");
    mac.update(canonical_envelope_string(input).as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
fn safe_equal_hex(a: &str, b: &str) -> bool {
    let ax = a.trim().to_ascii_lowercase();
    let bx = b.trim().to_ascii_lowercase();
    if ax.len() != 64 || bx.len() != 64 {
        return false;
    }
    let left = match hex_decode(ax) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let right = match hex_decode(bx) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0_u8;
    for (lhs, rhs) in left.iter().zip(right.iter()) {
        diff |= lhs ^ rhs;
    }
    diff == 0
}
fn verify_envelope(input: &Map<String, Value>) -> Value {
    let secret = as_text(input.get("secret"));
    if secret.is_empty() {
        return json!({ "ok": false, "reason": "secret_missing" });
    }
    let payload = envelope_payload_map(input);
    let payload_obj = payload.as_object().cloned().unwrap_or_default();
    let ts_num = payload_obj.get("ts").and_then(Value::as_i64).unwrap_or(0);
    if ts_num <= 0 {
        return json!({ "ok": false, "reason": "timestamp_invalid" });
    }
    let now = input
        .get("nowSec")
        .or_else(|| input.get("now_sec"))
        .and_then(Value::as_i64)
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    let max_skew = input
        .get("maxSkewSec")
        .or_else(|| input.get("max_skew_sec"))
        .and_then(Value::as_i64)
        .unwrap_or(900)
        .max(30);
    let skew = (now - ts_num).abs();
    if skew > max_skew {
        return json!({ "ok": false, "reason": "timestamp_skew", "skew_sec": skew, "max_skew_sec": max_skew });
    }
    let provided = as_text(input.get("signature"));
    let expected = sign_envelope(&payload_obj, &secret);
    if !safe_equal_hex(&expected, &provided) {
        return json!({ "ok": false, "reason": "signature_mismatch" });
    }
    json!({ "ok": true, "reason": "ok", "skew_sec": skew })
}
fn stamp_guard_env(input: &Map<String, Value>) -> Value {
    let mut env = input
        .get("baseEnv")
        .or_else(|| input.get("base_env"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let source = normalize_lower(input.get("source"));
    let action = normalize_lower(input.get("action"));
    let key_id = normalize_key_id(input.get("kid").or_else(|| env.get("REQUEST_KEY_ID")));
    env.insert(
        "REQUEST_SOURCE".to_string(),
        Value::String(if source.is_empty() {
            "local".to_string()
        } else {
            source
        }),
    );
    env.insert(
        "REQUEST_ACTION".to_string(),
        Value::String(if action.is_empty() {
            "apply".to_string()
        } else {
            action
        }),
    );
    if !key_id.is_empty() {
        env.insert("REQUEST_KEY_ID".to_string(), Value::String(key_id.clone()));
    }
    let key_from_kid = if key_id.is_empty() {
        String::new()
    } else {
        as_text(env.get(&secret_key_env_var_name_text(&key_id)))
    };
    let secret = {
        let explicit = as_text(input.get("secret"));
        if !explicit.is_empty() {
            explicit
        } else if !key_from_kid.is_empty() {
            key_from_kid
        } else {
            as_text(env.get("REQUEST_GATE_SECRET"))
        }
    };
    if secret.is_empty() {
        return Value::Object(env);
    }
    let payload = envelope_payload_map(&Map::from_iter([
        (
            "source".to_string(),
            Value::String(as_text(env.get("REQUEST_SOURCE"))),
        ),
        (
            "action".to_string(),
            Value::String(as_text(env.get("REQUEST_ACTION"))),
        ),
        (
            "files".to_string(),
            input
                .get("files")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new())),
        ),
        (
            "ts".to_string(),
            input.get("ts").cloned().unwrap_or(Value::Null),
        ),
        (
            "nonce".to_string(),
            input.get("nonce").cloned().unwrap_or(Value::Null),
        ),
        ("kid".to_string(), Value::String(key_id)),
    ]));
    let payload_obj = payload.as_object().cloned().unwrap_or_default();
    env.insert(
        "REQUEST_TS".to_string(),
        Value::String(
            payload_obj
                .get("ts")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .to_string(),
        ),
    );
    env.insert(
        "REQUEST_NONCE".to_string(),
        Value::String(as_text(payload_obj.get("nonce"))),
    );
    env.insert(
        "REQUEST_SIG".to_string(),
        Value::String(sign_envelope(&payload_obj, &secret)),
    );
    Value::Object(env)
}
fn verify_from_env(input: &Map<String, Value>) -> Value {
    let env = input
        .get("env")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let key_id = normalize_key_id(env.get("REQUEST_KEY_ID"));
    let key_from_kid = if key_id.is_empty() {
        String::new()
    } else {
        as_text(env.get(&secret_key_env_var_name_text(&key_id)))
    };
    let secret = {
        let explicit = as_text(input.get("secret"));
        if !explicit.is_empty() {
            explicit
        } else if !key_from_kid.is_empty() {
            key_from_kid
        } else {
            as_text(env.get("REQUEST_GATE_SECRET"))
        }
    };
    let mut payload = Map::new();
    payload.insert(
        "source".to_string(),
        Value::String(as_text(env.get("REQUEST_SOURCE"))),
    );
    payload.insert(
        "action".to_string(),
        Value::String(as_text(env.get("REQUEST_ACTION"))),
    );
    payload.insert("kid".to_string(), Value::String(key_id));
    let ts_value = as_text(env.get("REQUEST_TS"))
        .parse::<i64>()
        .ok()
        .map(Value::from)
        .unwrap_or(Value::Null);
    payload.insert("ts".to_string(), ts_value);
    payload.insert(
        "nonce".to_string(),
        Value::String(as_text(env.get("REQUEST_NONCE"))),
    );
    payload.insert(
        "files".to_string(),
        input
            .get("files")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
    );
    payload.insert(
        "signature".to_string(),
        Value::String(as_text(env.get("REQUEST_SIG"))),
    );
    payload.insert("secret".to_string(), Value::String(secret));
    if let Some(v) = input
        .get("maxSkewSec")
        .or_else(|| input.get("max_skew_sec"))
    {
        payload.insert("maxSkewSec".to_string(), v.clone());
    }
    if let Some(v) = input.get("nowSec").or_else(|| input.get("now_sec")) {
        payload.insert("nowSec".to_string(), v.clone());
    }
    verify_envelope(&payload)
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
                    print_json_line(&cli_error("request_envelope_kernel", &err));
                    return 1;
                }
            };
            let obj = payload_obj(&payload);
            let out = match other {
                "envelope-payload" => json!({ "payload": envelope_payload_map(obj) }),
                "canonical-string" => json!({ "canonical": canonical_envelope_string(obj) }),
                "sign" => {
                    let secret = as_text(obj.get("secret"));
                    json!({ "signature": sign_envelope(obj, &secret) })
                }
                "verify" => verify_envelope(obj),
                "stamp-env" => json!({ "env": stamp_guard_env(obj) }),
                "verify-from-env" => verify_from_env(obj),
                "normalize-files" => json!({ "files": normalize_files(obj.get("files")) }),
                "normalize-key-id" => {
                    json!({ "kid": normalize_key_id(obj.get("value").or_else(|| obj.get("kid"))) })
                }
                "normalize-web-query" => normalize_web_query_shape(obj),
                "normalize-web-date" => {
                    let raw = as_text(obj.get("value").or_else(|| obj.get("date")));
                    let iso = normalize_web_date_to_iso(&raw);
                    json!({
                        "ok": iso.is_some(),
                        "input": raw,
                        "iso": iso,
                        "perplexity": iso.as_deref().and_then(iso_to_perplexity_date)
                    })
                }
                "normalize-web-freshness" => {
                    let raw = as_text(obj.get("value").or_else(|| obj.get("freshness")));
                    let provider = normalize_web_provider(&as_text(
                        obj.get("provider").or_else(|| obj.get("provider_hint")),
                    ));
                    let normalized = normalize_web_freshness(&raw, &provider);
                    json!({
                        "ok": normalized.is_some(),
                        "provider": provider,
                        "input": raw,
                        "normalized": normalized
                    })
                }
                "secret-key-env-var-name" => {
                    json!({ "env_var": secret_key_env_var_name_text(&normalize_key_id(obj.get("value").or_else(|| obj.get("kid")))) })
                }
                _ => {
                    usage();
                    print_json_line(&cli_error("request_envelope_kernel", "unknown_command"));
                    return 1;
                }
            };
            print_json_line(&cli_receipt(
                &format!("request_envelope_kernel_{other}"),
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
    fn normalize_files_sorts_and_dedupes() {
        let rows = normalize_files(Some(&json!(["b.txt", "a.txt", "a.txt", "c\\tmp.txt"])));
        assert_eq!(rows, vec!["a.txt", "b.txt", "c/tmp.txt"]);
    }

    #[test]
    fn stamp_and_verify_roundtrip() {
        let env = stamp_guard_env(payload_obj(&json!({
            "baseEnv": { "REQUEST_GATE_SECRET": "secret-123" },
            "files": ["a.txt"],
            "source": "LOCAL",
            "action": "APPLY",
            "ts": 1000,
            "nonce": "abc123"
        })));
        let verify = verify_from_env(payload_obj(&json!({
            "env": env,
            "files": ["a.txt"],
            "secret": "secret-123",
            "nowSec": 1000
        })));
        assert_eq!(verify.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn normalize_web_freshness_maps_shortcuts() {
        assert_eq!(normalize_web_freshness("day", "brave"), Some("pd".to_string()));
        assert_eq!(
            normalize_web_freshness("pw", "perplexity"),
            Some("week".to_string())
        );
        assert_eq!(normalize_web_freshness("yesterday", "brave"), None);
    }

    #[test]
    fn normalize_web_freshness_validates_brave_range() {
        assert_eq!(
            normalize_web_freshness("2024-01-01to2024-01-31", "brave"),
            Some("2024-01-01to2024-01-31".to_string())
        );
        assert_eq!(
            normalize_web_freshness("2024-03-10to2024-03-01", "brave"),
            None
        );
    }
}
