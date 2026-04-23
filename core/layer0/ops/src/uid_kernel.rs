// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const BASE36: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

fn usage() {
    println!("uid-kernel commands:");
    println!("  infring-ops uid-kernel normalize-prefix [--payload-base64=<json>]");
    println!("  infring-ops uid-kernel is-alnum [--payload-base64=<json>]");
    println!("  infring-ops uid-kernel stable-uid [--payload-base64=<json>]");
    println!("  infring-ops uid-kernel random-uid [--payload-base64=<json>]");
    println!("  infring-ops uid-kernel contract-uid [--payload-base64=<json>]");
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
            .map_err(|err| format!("uid_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("uid_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("uid_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("uid_kernel_payload_decode_failed:{err}"));
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
        Some(Value::String(v)) => v.trim().chars().take(max_len).collect(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other
            .to_string()
            .trim_matches('"')
            .trim()
            .chars()
            .take(max_len)
            .collect(),
    }
}

fn parse_length(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Number(num)) => num.as_u64().unwrap_or(24) as usize,
        Some(Value::String(raw)) => raw.trim().parse::<usize>().unwrap_or(24),
        _ => 24,
    }
    .clamp(8, 48)
}

fn normalize_prefix(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| {
            ch.to_ascii_lowercase()
                .to_string()
                .chars()
                .collect::<Vec<_>>()
        })
        .take(4)
        .collect()
}

fn bytes_to_base36(bytes: &[u8]) -> String {
    if bytes.is_empty() || bytes.iter().all(|byte| *byte == 0) {
        return "0".to_string();
    }
    let mut digits = bytes.to_vec();
    let mut out = Vec::<char>::new();
    while !digits.is_empty() && digits.iter().any(|byte| *byte != 0) {
        let mut remainder = 0u32;
        let mut quotient = Vec::<u8>::new();
        let mut started = false;
        for byte in digits {
            let acc = remainder * 256 + byte as u32;
            let q = (acc / 36) as u8;
            remainder = acc % 36;
            if q != 0 || started {
                quotient.push(q);
                started = true;
            }
        }
        out.push(BASE36[remainder as usize] as char);
        digits = quotient;
    }
    out.reverse();
    out.into_iter().collect()
}

fn is_alnum(raw: &str) -> bool {
    !raw.is_empty() && raw.chars().all(|ch| ch.is_ascii_alphanumeric())
}

fn stable_uid(seed: &str, prefix: &str, length: usize) -> String {
    let normalized_prefix = normalize_prefix(prefix);
    let digest = Sha256::digest(seed.as_bytes());
    let body = bytes_to_base36(digest.as_slice());
    format!("{normalized_prefix}{body}")
        .chars()
        .take(length)
        .collect()
}

fn random_uid(prefix: &str, length: usize) -> String {
    let normalized_prefix = normalize_prefix(prefix);
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_millis())
        .unwrap_or(0);
    let ts = bytes_to_base36(&ts_ms.to_be_bytes());
    let mut random = [0u8; 24];
    OsRng.fill_bytes(&mut random);
    let body = bytes_to_base36(&random);
    format!("{normalized_prefix}{ts}{body}")
        .chars()
        .take(length)
        .collect()
}

fn contract_uid(payload: &Map<String, Value>) -> Value {
    let plugin_id = normalize_prefix(&clean_text(payload.get("plugin_id"), 64));
    let provider_id = normalize_prefix(&clean_text(payload.get("provider_id"), 64));
    let contract = clean_text(payload.get("contract"), 120)
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
        .collect::<String>();
    let prefix = clean_text(payload.get("prefix"), 32);
    let seed = format!(
        "contract_uid|plugin={plugin_id}|provider={provider_id}|contract={contract}"
    );
    let uid = stable_uid(
        &seed,
        if prefix.is_empty() { "c" } else { prefix.as_str() },
        parse_length(payload.get("length")),
    );
    json!({
        "ok": true,
        "uid": uid,
        "seed": seed,
        "plugin_id": plugin_id,
        "provider_id": provider_id,
        "contract": contract
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
            print_json_line(&cli_error("uid_kernel_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let result = match command.as_str() {
        "normalize-prefix" => cli_receipt(
            "uid_kernel_normalize_prefix",
            json!({
                "ok": true,
                "prefix": normalize_prefix(&clean_text(input.get("value").or_else(|| input.get("prefix")), 64))
            }),
        ),
        "is-alnum" => cli_receipt(
            "uid_kernel_is_alnum",
            json!({
                "ok": true,
                "result": is_alnum(&clean_text(input.get("value"), 128))
            }),
        ),
        "stable-uid" => cli_receipt(
            "uid_kernel_stable_uid",
            json!({
                "ok": true,
                "uid": stable_uid(
                    &clean_text(input.get("seed"), 4096),
                    &clean_text(input.get("prefix"), 32),
                    parse_length(input.get("length")),
                )
            }),
        ),
        "random-uid" => cli_receipt(
            "uid_kernel_random_uid",
            json!({
                "ok": true,
                "uid": random_uid(
                    &clean_text(input.get("prefix"), 32),
                    parse_length(input.get("length")),
                )
            }),
        ),
        "contract-uid" => cli_receipt("uid_kernel_contract_uid", contract_uid(input)),
        _ => cli_error("uid_kernel_error", &format!("unknown_command:{command}")),
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
    fn stable_uid_is_deterministic() {
        assert_eq!(stable_uid("alpha", "AB", 16), stable_uid("alpha", "AB", 16));
    }

    #[test]
    fn normalize_prefix_clamps() {
        assert_eq!(normalize_prefix("A-b_c:d?xyz"), "abcd");
    }

    #[test]
    fn contract_uid_is_deterministic() {
        let payload = serde_json::from_value::<Map<String, Value>>(json!({
            "plugin_id": "openrouter",
            "provider_id": "openrouter",
            "contract": "webSearchProviders",
            "length": 20,
            "prefix": "pc"
        }))
        .expect("payload");
        let one = contract_uid(&payload);
        let two = contract_uid(&payload);
        assert_eq!(one.get("uid"), two.get("uid"));
    }
}
