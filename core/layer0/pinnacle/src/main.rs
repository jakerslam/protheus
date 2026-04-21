// SPDX-License-Identifier: Apache-2.0
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use protheus_pinnacle_core_v1::{get_sovereignty_index, merge_delta, merge_delta_json};
use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::path::{Component, Path};

const MAX_ARG_KEY_LEN: usize = 48;
const MAX_PAYLOAD_BYTES: usize = 32 * 1024;
const MAX_NODE_ID_LEN: usize = 96;
const MAX_CHANGE_KEY_LEN: usize = 160;
const MAX_STRING_VALUE_LEN: usize = 2 * 1024;
const MAX_CHANGE_ENTRIES: usize = 1024;
const MAX_ARRAY_VALUES: usize = 256;

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                ch,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .collect()
}

fn sanitize_token(raw: &str, max_len: usize) -> String {
    let mut value: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    value = value.trim().to_string();
    if value.chars().count() > max_len {
        value = value.chars().take(max_len).collect();
    }
    value
}

fn parse_arg(args: &[String], key: &str) -> Option<String> {
    let key = sanitize_token(key, MAX_ARG_KEY_LEN);
    for arg in args {
        if let Some((k, v)) = arg.split_once('=') {
            if sanitize_token(k, MAX_ARG_KEY_LEN) == key {
                let value = sanitize_token(v, MAX_PAYLOAD_BYTES);
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn is_safe_json_file_path(raw: &str) -> bool {
    let path = Path::new(raw);
    if raw.is_empty() || path.is_dir() {
        return false;
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return false;
    }
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}

fn decode_payload(raw: String) -> Result<String, String> {
    let bytes = BASE64_STANDARD
        .decode(raw.as_bytes())
        .map_err(|e| format!("base64_decode_failed:{e}"))?;
    if bytes.len() > MAX_PAYLOAD_BYTES {
        return Err("payload_base64_too_large".to_string());
    }
    String::from_utf8(bytes).map_err(|e| format!("utf8_decode_failed:{e}"))
}

fn load_json_arg(
    args: &[String],
    raw_key: &str,
    b64_key: &str,
    file_key: &str,
) -> Result<String, String> {
    if let Some(v) = parse_arg(args, raw_key) {
        if v.len() > MAX_PAYLOAD_BYTES {
            return Err(format!("payload_too_large:{raw_key}"));
        }
        return Ok(v);
    }
    if let Some(v) = parse_arg(args, b64_key) {
        return decode_payload(v);
    }
    if let Some(v) = parse_arg(args, file_key) {
        if !is_safe_json_file_path(&v) {
            return Err(format!("payload_file_path_invalid:{file_key}"));
        }
        let metadata = fs::metadata(v.as_str()).map_err(|e| format!("file_stat_failed:{e}"))?;
        if !metadata.is_file() {
            return Err(format!("payload_file_not_a_file:{file_key}"));
        }
        if metadata.len() > MAX_PAYLOAD_BYTES as u64 {
            return Err(format!("payload_file_too_large:{file_key}"));
        }
        let text = fs::read_to_string(v.as_str()).map_err(|e| format!("file_read_failed:{e}"))?;
        if text.len() > MAX_PAYLOAD_BYTES {
            return Err(format!("payload_file_too_large:{file_key}"));
        }
        return Ok(text);
    }
    Err(format!("missing_payload:{}", raw_key))
}

fn sanitize_json_value(value: &mut Value) {
    match value {
        Value::String(raw) => {
            *raw = sanitize_token(raw, MAX_STRING_VALUE_LEN);
        }
        Value::Array(values) => {
            if values.len() > MAX_ARRAY_VALUES {
                values.truncate(MAX_ARRAY_VALUES);
            }
            for entry in values {
                sanitize_json_value(entry);
            }
        }
        Value::Object(map) => {
            let drained: Vec<(String, Value)> =
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            map.clear();
            for (raw_key, mut entry) in drained {
                let key = sanitize_token(&raw_key, MAX_CHANGE_KEY_LEN);
                if key.is_empty() {
                    continue;
                }
                sanitize_json_value(&mut entry);
                map.insert(key, entry);
            }
        }
        _ => {}
    }
}

fn normalize_delta_json(raw: &str) -> Result<String, String> {
    let mut root: Value =
        serde_json::from_str(raw).map_err(|err| format!("payload_parse_failed:{err}"))?;
    let obj = root
        .as_object_mut()
        .ok_or_else(|| "payload_root_not_object".to_string())?;

    let node_raw = obj
        .get("node_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let node_id = sanitize_token(&node_raw, MAX_NODE_ID_LEN);
    if node_id.is_empty() {
        return Err("payload_node_id_invalid".to_string());
    }
    obj.insert("node_id".to_string(), Value::String(node_id));

    let mut normalized_changes = Map::<String, Value>::new();
    let changes = obj
        .get_mut("changes")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "payload_changes_missing".to_string())?;
    for (raw_key, value) in changes {
        if normalized_changes.len() >= MAX_CHANGE_ENTRIES {
            return Err("payload_changes_too_many_entries".to_string());
        }
        let key = sanitize_token(raw_key, MAX_CHANGE_KEY_LEN);
        if key.is_empty() {
            continue;
        }
        let mut normalized_value = value.clone();
        sanitize_json_value(&mut normalized_value);
        if normalized_changes.insert(key, normalized_value).is_some() {
            return Err("payload_changes_key_collision_after_normalization".to_string());
        }
    }
    if normalized_changes.is_empty() {
        return Err("payload_changes_empty_after_normalization".to_string());
    }
    obj.insert("changes".to_string(), Value::Object(normalized_changes));

    serde_json::to_string(&root).map_err(|err| format!("payload_encode_failed:{err}"))
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  pinnacle_core merge --left-json=<payload> --right-json=<payload>");
    eprintln!("  pinnacle_core merge --left-b64=<base64> --right-b64=<base64>");
    eprintln!("  pinnacle_core index --left-json=<payload> --right-json=<payload>");
    eprintln!("  pinnacle_core demo");
}

fn demo_delta(node: &str, value: i64, clock: u64) -> String {
    serde_json::json!({
        "node_id": node,
        "changes": {
            "topic/revenue": {
                "payload": { "score": value },
                "vector_clock": { node: clock },
                "signed": true
            }
        }
    })
    .to_string()
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args
        .first()
        .map(|value| sanitize_token(value, 24).to_ascii_lowercase())
        .unwrap_or_else(|| "demo".to_string());

    match command.as_str() {
        "merge" => {
            let left = load_json_arg(&args[1..], "--left-json", "--left-b64", "--left-file");
            let right = load_json_arg(&args[1..], "--right-json", "--right-b64", "--right-file");
            match (left, right) {
                (Ok(l), Ok(r)) => match (normalize_delta_json(&l), normalize_delta_json(&r)) {
                    (Ok(normalized_left), Ok(normalized_right)) => {
                        match merge_delta_json(&normalized_left, &normalized_right) {
                            Ok(v) => println!("{}", v),
                            Err(err) => {
                                eprintln!("{}", serde_json::json!({ "ok": false, "error": err }));
                                std::process::exit(1);
                            }
                        }
                    }
                    (Err(err), _) | (_, Err(err)) => {
                        eprintln!("{}", serde_json::json!({ "ok": false, "error": err }));
                        std::process::exit(1);
                    }
                },
                (Err(err), _) | (_, Err(err)) => {
                    eprintln!("{}", serde_json::json!({ "ok": false, "error": err }));
                    std::process::exit(1);
                }
            }
        }
        "index" => {
            let left = load_json_arg(&args[1..], "--left-json", "--left-b64", "--left-file");
            let right = load_json_arg(&args[1..], "--right-json", "--right-b64", "--right-file");
            match (left, right) {
                (Ok(l), Ok(r)) => match (normalize_delta_json(&l), normalize_delta_json(&r)) {
                    (Ok(normalized_left), Ok(normalized_right)) => {
                        match get_sovereignty_index(&normalized_left, &normalized_right) {
                            Ok(v) => {
                                println!("{}", serde_json::json!({ "sovereignty_index_pct": v }))
                            }
                            Err(err) => {
                                eprintln!("{}", serde_json::json!({ "ok": false, "error": err }));
                                std::process::exit(1);
                            }
                        }
                    }
                    (Err(err), _) | (_, Err(err)) => {
                        eprintln!("{}", serde_json::json!({ "ok": false, "error": err }));
                        std::process::exit(1);
                    }
                },
                (Err(err), _) | (_, Err(err)) => {
                    eprintln!("{}", serde_json::json!({ "ok": false, "error": err }));
                    std::process::exit(1);
                }
            }
        }
        "demo" => {
            let left = demo_delta("device_a", 42, 2);
            let right = demo_delta("device_b", 45, 2);
            let merged = merge_delta(&left, &right).expect("demo_merge");
            println!(
                "{}",
                serde_json::to_string(&merged).unwrap_or_else(|_| "{}".to_string())
            );
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}
