// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("conversation-eye-synthesizer-kernel commands:");
    println!("  protheus-ops conversation-eye-synthesizer-kernel synthesize-envelope --payload-base64=<json>");
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
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
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
            .map_err(|err| format!("conversation_eye_synthesizer_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("conversation_eye_synthesizer_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("conversation_eye_synthesizer_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("conversation_eye_synthesizer_payload_decode_failed:{err}"));
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
        Some(Value::String(v)) => v.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn clean_text(raw: Option<&Value>, max_len: usize) -> String {
    let normalized = as_text(raw)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    normalized.chars().take(max_len).collect::<String>()
}

fn sha16(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::new();
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out.chars().take(16).collect::<String>()
}

fn normalize_tag(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in raw.trim().to_ascii_lowercase().chars() {
        let mapped = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.') {
            ch
        } else {
            '_'
        };
        if mapped == '_' {
            if prev_sep || out.is_empty() {
                continue;
            }
            prev_sep = true;
            out.push(mapped);
        } else {
            prev_sep = false;
            out.push(mapped);
        }
        if out.len() >= 32 {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn normalize_tags(raw_tags: Option<&Value>) -> Vec<String> {
    let defaults = ["conversation", "decision", "insight", "directive", "t1"];
    let mut out = Vec::<String>::new();
    for raw in defaults {
        let tag = normalize_tag(raw);
        if !tag.is_empty() && !out.contains(&tag) {
            out.push(tag);
        }
    }
    if let Some(Value::Array(rows)) = raw_tags {
        for row in rows {
            let tag = normalize_tag(&clean_text(Some(row), 32));
            if !tag.is_empty() && !out.contains(&tag) {
                out.push(tag);
            }
            if out.len() >= 12 {
                break;
            }
        }
    }
    out.truncate(12);
    out
}

fn infer_level(payload: &Map<String, Value>) -> i64 {
    let level = payload
        .get("level")
        .and_then(|value| match value {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => s.trim().parse::<i64>().ok(),
            _ => None,
        })
        .unwrap_or(0);
    if level > 0 {
        return level.clamp(1, 3);
    }
    let priority = clean_text(
        payload.get("priority").or_else(|| payload.get("severity")),
        16,
    )
    .to_ascii_lowercase();
    if matches!(priority.as_str(), "high" | "critical") {
        1
    } else if priority == "medium" {
        2
    } else {
        3
    }
}

fn level_token(level: i64) -> &'static str {
    match level {
        i64::MIN..=1 => "jot1",
        2 => "jot2",
        _ => "jot3",
    }
}

fn synthesize_envelope(row: &Map<String, Value>) -> Value {
    let now = now_iso();
    let date = {
        let raw = clean_text(
            row.get("date")
                .or_else(|| row.get("ts"))
                .or_else(|| row.get("timestamp")),
            32,
        );
        if raw.is_empty() {
            now.clone()
        } else {
            raw
        }
    };
    let title = {
        let raw = clean_text(
            row.get("title")
                .or_else(|| row.get("subject"))
                .or_else(|| row.get("topic")),
            180,
        );
        if raw.is_empty() {
            "Conversation Eye insight".to_string()
        } else {
            raw
        }
    };
    let preview = {
        let raw = clean_text(
            row.get("preview")
                .or_else(|| row.get("summary"))
                .or_else(|| row.get("message"))
                .or_else(|| row.get("content")),
            320,
        );
        if raw.is_empty() {
            title.clone()
        } else {
            raw
        }
    };
    let node_kind = {
        let raw =
            clean_text(row.get("node_kind").or_else(|| row.get("kind")), 32).to_ascii_lowercase();
        if raw.is_empty() {
            "insight".to_string()
        } else {
            raw
        }
    };
    let level = infer_level(row);
    let node_tags = normalize_tags(row.get("node_tags").or_else(|| row.get("tags")));

    let stable_seed = serde_json::to_string(&json!({
        "date": date,
        "title": title,
        "preview": preview,
        "node_kind": node_kind,
        "node_tags": node_tags,
    }))
    .unwrap_or_else(|_| "{}".to_string());

    let node_id = {
        let raw = clean_text(row.get("node_id"), 120);
        if raw.is_empty() {
            format!("conversation-eye-{}", sha16(&stable_seed))
        } else {
            raw
        }
    };
    let hex_id = {
        let raw = clean_text(row.get("hex_id"), 24);
        if raw.is_empty() {
            sha16(&format!("{node_id}|{date}"))
        } else {
            raw
        }
    };
    let xml = {
        let raw = clean_text(row.get("xml"), 1600);
        if raw.is_empty() {
            format!(
                "<conversation-node id=\"{node_id}\" kind=\"{node_kind}\" level=\"{level}\"><title>{title}</title><preview>{preview}</preview></conversation-node>"
            )
        } else {
            raw
        }
    };

    let ts_value = {
        let raw = clean_text(row.get("ts"), 32);
        if raw.is_empty() {
            now
        } else {
            raw
        }
    };
    let level_token_value = {
        let raw = clean_text(row.get("level_token"), 16);
        if raw.is_empty() {
            level_token(level).to_string()
        } else {
            raw
        }
    };

    json!({
        "ts": ts_value,
        "date": date.chars().take(20).collect::<String>(),
        "node_id": node_id,
        "hex_id": hex_id,
        "node_kind": node_kind,
        "level": level,
        "level_token": level_token_value,
        "node_tags": node_tags,
        "edges_to": row.get("edges_to").and_then(Value::as_array).cloned().unwrap_or_default().into_iter().take(12).collect::<Vec<_>>(),
        "title": title,
        "preview": preview,
        "xml": xml,
    })
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
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
            print_json_line(&cli_error(
                "conversation_eye_synthesizer_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let result = match command.as_str() {
        "synthesize-envelope" => cli_receipt(
            "conversation_eye_synthesizer_kernel_synthesize_envelope",
            json!({
                "ok": true,
                "envelope": synthesize_envelope(input),
            }),
        ),
        _ => cli_error(
            "conversation_eye_synthesizer_kernel_error",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesizes_stable_defaults() {
        let row = serde_json::from_value::<Map<String, Value>>(json!({
            "message": "hello world",
            "severity": "high",
        }))
        .unwrap();
        let envelope = synthesize_envelope(&row);
        assert_eq!(envelope["level"], json!(1));
        assert_eq!(envelope["level_token"], json!("jot1"));
        assert_eq!(envelope["node_kind"], json!("insight"));
        assert!(envelope["node_id"]
            .as_str()
            .unwrap()
            .starts_with("conversation-eye-"));
    }
}
