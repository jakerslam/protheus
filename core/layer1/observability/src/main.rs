// SPDX-License-Identifier: Apache-2.0
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use protheus_observability_core_v1::{
    load_embedded_observability_profile_json, run_chaos_resilience_json, ChaosScenarioRequest,
    TraceEvent,
};
use std::env;
use std::fs;
use std::path::{Component, Path};

const MAX_ARG_KEY_LEN: usize = 48;
const MAX_REQUEST_BYTES: usize = 32 * 1024;
const MAX_SCENARIO_ID_LEN: usize = 96;
const MAX_TRACE_ID_LEN: usize = 96;
const MAX_TEXT_TOKEN_LEN: usize = 160;
const MAX_EVENT_COUNT: usize = 512;
const MAX_EVENT_TAGS: usize = 32;

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

fn sanitize_text_token(raw: &str, max_len: usize) -> String {
    let mut token: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    token = token.trim().to_string();
    if token.chars().count() > max_len {
        token = token.chars().take(max_len).collect();
    }
    token
}

fn parse_arg(args: &[String], key: &str) -> Option<String> {
    let key = sanitize_text_token(key, MAX_ARG_KEY_LEN);
    for arg in args {
        if let Some((k, v)) = arg.split_once('=') {
            if sanitize_text_token(k, MAX_ARG_KEY_LEN) == key {
                let value = sanitize_text_token(v, MAX_REQUEST_BYTES);
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn is_safe_request_file_path(raw: &str) -> bool {
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

fn load_request_json(args: &[String]) -> Result<String, String> {
    if let Some(v) = parse_arg(args, "--request-json") {
        if v.len() > MAX_REQUEST_BYTES {
            return Err("request_json_too_large".to_string());
        }
        return Ok(v);
    }
    if let Some(v) = parse_arg(args, "--request-base64") {
        let bytes = BASE64_STANDARD
            .decode(v.as_bytes())
            .map_err(|err| format!("base64_decode_failed:{err}"))?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err("request_base64_too_large".to_string());
        }
        let text = String::from_utf8(bytes).map_err(|err| format!("utf8_decode_failed:{err}"))?;
        return Ok(text);
    }
    if let Some(v) = parse_arg(args, "--request-file") {
        if !is_safe_request_file_path(&v) {
            return Err("request_file_path_invalid".to_string());
        }
        let metadata =
            fs::metadata(v.as_str()).map_err(|err| format!("request_file_stat_failed:{err}"))?;
        if !metadata.is_file() {
            return Err("request_file_not_a_file".to_string());
        }
        if metadata.len() > MAX_REQUEST_BYTES as u64 {
            return Err("request_file_too_large".to_string());
        }
        let text = fs::read_to_string(v.as_str())
            .map_err(|err| format!("request_file_read_failed:{err}"))?;
        if text.len() > MAX_REQUEST_BYTES {
            return Err("request_file_too_large".to_string());
        }
        return Ok(text);
    }
    Err("missing_request_payload".to_string())
}

fn normalize_request_json(raw_request: &str) -> Result<String, String> {
    let mut request: ChaosScenarioRequest =
        serde_json::from_str(raw_request).map_err(|err| format!("request_parse_failed:{err}"))?;
    request.scenario_id = sanitize_text_token(&request.scenario_id, MAX_SCENARIO_ID_LEN);
    if request.scenario_id.is_empty() {
        return Err("request_invalid_scenario_id".to_string());
    }
    if request.cycles == 0 {
        return Err("request_invalid_cycles".to_string());
    }
    if request.cycles > 2_000_000 {
        request.cycles = 2_000_000;
    }
    if request.inject_fault_every > request.cycles {
        request.inject_fault_every = request.cycles;
    }
    if request.events.is_empty() {
        return Err("request_missing_events".to_string());
    }
    if request.events.len() > MAX_EVENT_COUNT {
        return Err("request_too_many_events".to_string());
    }
    for event in &mut request.events {
        event.trace_id = sanitize_text_token(&event.trace_id, MAX_TRACE_ID_LEN);
        if event.trace_id.is_empty() {
            return Err("request_invalid_trace_id".to_string());
        }
        event.source = sanitize_text_token(&event.source, MAX_TEXT_TOKEN_LEN);
        event.operation = sanitize_text_token(&event.operation, MAX_TEXT_TOKEN_LEN);
        event.severity = sanitize_text_token(&event.severity, 16).to_ascii_lowercase();
        event.payload_digest = sanitize_text_token(&event.payload_digest, MAX_TEXT_TOKEN_LEN);
        if event.payload_digest.is_empty() {
            return Err("request_invalid_payload_digest".to_string());
        }
        event.tags = event
            .tags
            .iter()
            .map(|tag| sanitize_text_token(tag, 48).to_ascii_lowercase())
            .filter(|tag| !tag.is_empty())
            .take(MAX_EVENT_TAGS)
            .collect();
        if event.tags.is_empty() {
            return Err("request_invalid_event_tags".to_string());
        }
    }
    serde_json::to_string(&request).map_err(|err| format!("request_encode_failed:{err}"))
}

fn demo_request() -> ChaosScenarioRequest {
    ChaosScenarioRequest {
        scenario_id: "observability_demo".to_string(),
        events: vec![
            TraceEvent {
                trace_id: "e1".to_string(),
                ts_millis: 1_000,
                source: "client/runtime/systems/observability".to_string(),
                operation: "trace.capture".to_string(),
                severity: "low".to_string(),
                tags: vec!["runtime.guardrails".to_string()],
                payload_digest: "sha256:e1".to_string(),
                signed: true,
            },
            TraceEvent {
                trace_id: "e2".to_string(),
                ts_millis: 1_120,
                source: "client/runtime/systems/red_legion".to_string(),
                operation: "chaos.replay".to_string(),
                severity: "medium".to_string(),
                tags: vec!["chaos.replay".to_string(), "drift".to_string()],
                payload_digest: "sha256:e2".to_string(),
                signed: true,
            },
        ],
        cycles: 200000,
        inject_fault_every: 500,
        enforce_fail_closed: true,
    }
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  observability_core load-profile");
    eprintln!("  observability_core run-chaos --request-json=<payload>");
    eprintln!("  observability_core run-chaos --request-base64=<base64_payload>");
    eprintln!("  observability_core run-chaos --request-file=<path>");
    eprintln!("  observability_core demo");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args
        .first()
        .map(|value| sanitize_text_token(value, 24).to_ascii_lowercase())
        .unwrap_or_else(|| "demo".to_string());

    match command.as_str() {
        "load-profile" => match load_embedded_observability_profile_json() {
            Ok(payload) => println!("{}", payload),
            Err(err) => {
                eprintln!(
                    "{}",
                    serde_json::json!({ "ok": false, "error": err.to_string() })
                );
                std::process::exit(1);
            }
        },
        "run-chaos" => match load_request_json(&args[1..]) {
            Ok(request_json) => match normalize_request_json(&request_json) {
                Ok(normalized_request_json) => {
                    match run_chaos_resilience_json(&normalized_request_json) {
                        Ok(payload) => println!("{}", payload),
                        Err(err) => {
                            eprintln!(
                                "{}",
                                serde_json::json!({ "ok": false, "error": err.to_string() })
                            );
                            std::process::exit(1);
                        }
                    }
                }
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
            let request_json =
                serde_json::to_string(&demo_request()).unwrap_or_else(|_| "{}".to_string());
            match run_chaos_resilience_json(&request_json) {
                Ok(payload) => println!("{}", payload),
                Err(err) => {
                    eprintln!(
                        "{}",
                        serde_json::json!({ "ok": false, "error": err.to_string() })
                    );
                    std::process::exit(1);
                }
            }
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}
