// SPDX-License-Identifier: Apache-2.0
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use protheus_red_legion_core_v1::{run_chaos_game, run_chaos_game_json, ChaosGameRequest};
use std::env;
use std::fs;

const MAX_ARG_KEY_LEN: usize = 48;
const MAX_REQUEST_BYTES: usize = 32 * 1024;
const MAX_MISSION_ID_LEN: usize = 96;

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

fn sanitize_text(raw: &str, max_len: usize, lowercase: bool) -> String {
    let mut text: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    text = text.trim().to_string();
    if lowercase {
        text = text.to_ascii_lowercase();
    }
    if text.len() > max_len {
        text.truncate(max_len);
    }
    text
}

fn parse_arg(args: &[String], key: &str) -> Option<String> {
    let key = sanitize_text(key, MAX_ARG_KEY_LEN, false);
    for arg in args {
        if let Some((k, v)) = arg.split_once('=') {
            if sanitize_text(k, MAX_ARG_KEY_LEN, false) == key {
                let value = sanitize_text(v, MAX_REQUEST_BYTES, false);
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
        if v.len() > MAX_REQUEST_BYTES {
            return Err("request_json_too_large".to_string());
        }
        return Ok(v);
    }
    if let Some(v) = parse_arg(args, "--request-base64") {
        let bytes = BASE64_STANDARD
            .decode(v.as_bytes())
            .map_err(|e| format!("base64_decode_failed:{e}"))?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err("request_base64_too_large".to_string());
        }
        let text = String::from_utf8(bytes).map_err(|e| format!("utf8_decode_failed:{e}"))?;
        if text.trim().is_empty() {
            return Err("request_payload_empty".to_string());
        }
        return Ok(text);
    }
    if let Some(v) = parse_arg(args, "--request-file") {
        let text =
            fs::read_to_string(v.as_str()).map_err(|e| format!("request_file_read_failed:{e}"))?;
        if text.len() > MAX_REQUEST_BYTES {
            return Err("request_file_too_large".to_string());
        }
        if text.trim().is_empty() {
            return Err("request_payload_empty".to_string());
        }
        return Ok(text);
    }
    Err("missing_request_payload".to_string())
}

fn normalize_request_payload(raw: &str) -> Result<String, String> {
    let mut request: ChaosGameRequest =
        serde_json::from_str(raw).map_err(|e| format!("request_parse_failed:{e}"))?;
    request.mission_id = sanitize_text(&request.mission_id, MAX_MISSION_ID_LEN, true);
    if request.mission_id.is_empty() {
        return Err("request_mission_id_invalid".to_string());
    }
    if request.cycles == 0 {
        return Err("request_cycles_invalid".to_string());
    }
    if request.cycles > 2_000_000 {
        request.cycles = 2_000_000;
    }
    if request.inject_fault_every > request.cycles {
        request.inject_fault_every = request.cycles;
    }
    serde_json::to_string(&request).map_err(|e| format!("request_encode_failed:{e}"))
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  red_legion_core run --request-json=<payload>");
    eprintln!("  red_legion_core run --request-base64=<payload>");
    eprintln!("  red_legion_core demo");
}

fn demo_request() -> ChaosGameRequest {
    ChaosGameRequest {
        mission_id: "red_legion_demo".to_string(),
        cycles: 220000,
        inject_fault_every: 500,
        enforce_fail_closed: true,
        event_seed: 1_000,
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args
        .first()
        .map(|value| sanitize_text(value, 24, true))
        .unwrap_or_else(|| "demo".to_string());

    match command.as_str() {
        "run" => match load_request(&args[1..]) {
            Ok(payload) => match normalize_request_payload(&payload) {
                Ok(normalized_payload) => match run_chaos_game_json(&normalized_payload) {
                    Ok(v) => println!("{}", v),
                    Err(err) => {
                        eprintln!("{}", serde_json::json!({ "ok": false, "error": err }));
                        std::process::exit(1);
                    }
                },
                Err(err) => {
                    eprintln!("{}", serde_json::json!({ "ok": false, "error": err }));
                    std::process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("{}", serde_json::json!({ "ok": false, "error": err }));
                std::process::exit(1);
            }
        },
        "demo" => {
            let request = demo_request();
            let receipt = run_chaos_game(&request).expect("demo");
            println!(
                "{}",
                serde_json::to_string(&receipt).unwrap_or_else(|_| "{}".to_string())
            );
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}
