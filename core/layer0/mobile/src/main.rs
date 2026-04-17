// SPDX-License-Identifier: Apache-2.0
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use protheus_mobile_core_v1::{run_mobile_cycle, run_mobile_cycle_json};
use std::env;
use std::fs;

const MAX_ARG_KEY_LEN: usize = 64;
const MAX_ARG_VALUE_LEN: usize = 16_384;
const MAX_REQUEST_BYTES: usize = 128_000;
const MAX_PATH_LEN: usize = 4_096;
const MAX_COMMAND_LEN: usize = 64;

fn strip_invisible_unicode(input: &str) -> String {
    input
        .chars()
        .filter(|c| !matches!(*c, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'))
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

fn sanitize_text(input: &str, max_len: usize) -> String {
    strip_invisible_unicode(input)
        .trim()
        .chars()
        .take(max_len)
        .collect()
}

fn normalize_request_payload(payload: &str) -> Result<String, String> {
    let normalized = sanitize_text(payload, MAX_REQUEST_BYTES);
    if normalized.is_empty() {
        return Err("request_payload_empty".to_string());
    }
    if normalized.as_bytes().len() > MAX_REQUEST_BYTES {
        return Err("request_too_large".to_string());
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&normalized).map_err(|e| format!("request_json_invalid:{e}"))?;
    let canonical =
        serde_json::to_string(&parsed).map_err(|e| format!("request_json_encode_failed:{e}"))?;
    if canonical.as_bytes().len() > MAX_REQUEST_BYTES {
        return Err("request_too_large".to_string());
    }
    Ok(canonical)
}

fn parse_arg(args: &[String], key: &str) -> Option<String> {
    let normalized_key = sanitize_text(key, MAX_ARG_KEY_LEN);
    for arg in args {
        if let Some((k, v)) = arg.split_once('=') {
            if sanitize_text(k, MAX_ARG_KEY_LEN) == normalized_key {
                let value = sanitize_text(v, MAX_ARG_VALUE_LEN);
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn load_request(args: &[String]) -> Result<String, String> {
    if let Some(v) = parse_arg(args, "--request-json") {
        return normalize_request_payload(v.as_str());
    }
    if let Some(v) = parse_arg(args, "--request-base64") {
        if v.is_empty() {
            return Err("request_base64_empty".to_string());
        }
        let bytes = BASE64_STANDARD
            .decode(v.as_bytes())
            .map_err(|e| format!("base64_decode_failed:{e}"))?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err("request_too_large".to_string());
        }
        let text = String::from_utf8(bytes).map_err(|e| format!("utf8_decode_failed:{e}"))?;
        return normalize_request_payload(text.as_str());
    }
    if let Some(v) = parse_arg(args, "--request-file") {
        let path = sanitize_text(v.as_str(), MAX_PATH_LEN);
        if path.is_empty() {
            return Err("request_file_path_empty".to_string());
        }
        let bytes = fs::read(path.as_str()).map_err(|e| format!("request_file_read_failed:{e}"))?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err("request_too_large".to_string());
        }
        let text = String::from_utf8(bytes).map_err(|e| format!("utf8_decode_failed:{e}"))?;
        return normalize_request_payload(text.as_str());
    }
    Err("missing_request_payload".to_string())
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  mobile_core run --request-json=<payload>");
    eprintln!("  mobile_core run --request-base64=<payload>");
    eprintln!("  mobile_core run --request-file=<path>");
    eprintln!("  mobile_core demo");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args
        .first()
        .map(|raw| sanitize_text(raw, MAX_COMMAND_LEN).to_lowercase())
        .unwrap_or_else(|| "demo".to_string());

    match command.as_str() {
        "run" => match load_request(&args[1..]) {
            Ok(payload) => match run_mobile_cycle_json(&payload) {
                Ok(v) => println!("{}", v),
                Err(err) => {
                    eprintln!("{}", serde_json::json!({"ok": false, "error": err}));
                    std::process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("{}", serde_json::json!({"ok": false, "error": err}));
                std::process::exit(1);
            }
        },
        "demo" => {
            let report = run_mobile_cycle(None).expect("demo");
            println!(
                "{}",
                serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string())
            );
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}
