// SPDX-License-Identifier: Apache-2.0
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use infring_singularity_seed_core_v1::{
    freeze_seed, run_guarded_cycle, show_seed_state_json, CycleRequest, DriftOverride,
};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Component, Path};

const MAX_ARG_KEY_LEN: usize = 48;
const MAX_ARG_VALUE_LEN: usize = 32 * 1024;
const MAX_DRIFT_LOOP_ID_LEN: usize = 96;
const MAX_DRIFT_OVERRIDES: usize = 128;
const MAX_ABS_DRIFT_PCT: f64 = 100.0;

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

fn sanitize_cli_value(raw: &str, max_len: usize) -> String {
    let mut normalized: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    normalized = normalized.trim().to_string();
    if normalized.chars().count() > max_len {
        normalized = normalized.chars().take(max_len).collect();
    }
    normalized
}

fn parse_arg(args: &[String], key: &str) -> Option<String> {
    let key = sanitize_cli_value(key, MAX_ARG_KEY_LEN);
    for arg in args {
        if let Some((k, v)) = arg.split_once('=') {
            if sanitize_cli_value(k, MAX_ARG_KEY_LEN) == key {
                let value = sanitize_cli_value(v, MAX_ARG_VALUE_LEN);
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

fn parse_request(args: &[String]) -> Result<CycleRequest, String> {
    if let Some(v) = parse_arg(args, "--request-json") {
        if v.len() > MAX_ARG_VALUE_LEN {
            return Err("request_json_too_large".to_string());
        }
        return serde_json::from_str(&v).map_err(|err| format!("request_parse_failed:{err}"));
    }
    if let Some(v) = parse_arg(args, "--request-base64") {
        let bytes = BASE64_STANDARD
            .decode(v.as_bytes())
            .map_err(|err| format!("base64_decode_failed:{err}"))?;
        if bytes.len() > MAX_ARG_VALUE_LEN {
            return Err("request_base64_too_large".to_string());
        }
        let text = String::from_utf8(bytes).map_err(|err| format!("utf8_decode_failed:{err}"))?;
        return serde_json::from_str(&text).map_err(|err| format!("request_parse_failed:{err}"));
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
        if metadata.len() > MAX_ARG_VALUE_LEN as u64 {
            return Err("request_file_too_large".to_string());
        }
        let text = fs::read_to_string(v.as_str())
            .map_err(|err| format!("request_file_read_failed:{err}"))?;
        if text.len() > MAX_ARG_VALUE_LEN {
            return Err("request_file_too_large".to_string());
        }
        return serde_json::from_str(&text).map_err(|err| format!("request_parse_failed:{err}"));
    }

    let mut request = CycleRequest::default();
    if let Some(v) = parse_arg(args, "--inject-drift") {
        let mut overrides_by_loop = BTreeMap::<String, DriftOverride>::new();
        for part in v.split(',') {
            if overrides_by_loop.len() >= MAX_DRIFT_OVERRIDES {
                return Err("too_many_drift_overrides".to_string());
            }
            let trimmed = sanitize_cli_value(part, MAX_ARG_VALUE_LEN);
            if trimmed.is_empty() {
                continue;
            }
            let (loop_id, drift) = trimmed
                .split_once(':')
                .ok_or_else(|| format!("invalid_inject_drift:{trimmed}"))?;
            let loop_id = sanitize_cli_value(loop_id, MAX_DRIFT_LOOP_ID_LEN);
            if loop_id.is_empty() {
                return Err("invalid_drift_loop_id".to_string());
            }
            let drift_pct = drift
                .parse::<f64>()
                .map_err(|_| format!("invalid_drift_value:{drift}"))?;
            if !drift_pct.is_finite() {
                return Err("invalid_drift_value_non_finite".to_string());
            }
            if drift_pct.abs() > MAX_ABS_DRIFT_PCT {
                return Err("invalid_drift_value_out_of_bounds".to_string());
            }
            overrides_by_loop.insert(loop_id.clone(), DriftOverride { loop_id, drift_pct });
        }
        request.drift_overrides = overrides_by_loop.into_values().collect();
    }

    Ok(request)
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  singularity_seed_core freeze");
    eprintln!("  singularity_seed_core cycle [--request-json=<json>] [--request-base64=<base64>] [--request-file=<path>] [--inject-drift=<loop:drift,...>]");
    eprintln!("  singularity_seed_core show");
    eprintln!("  singularity_seed_core demo");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args
        .first()
        .map(|value| sanitize_cli_value(value, MAX_ARG_KEY_LEN).to_ascii_lowercase())
        .unwrap_or_else(|| "demo".to_string());

    match command.as_str() {
        "freeze" => match freeze_seed() {
            Ok(report) => println!(
                "{}",
                serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(err) => {
                eprintln!(
                    "{}",
                    serde_json::json!({ "ok": false, "error": err.to_string() })
                );
                std::process::exit(1);
            }
        },
        "cycle" => match parse_request(&args[1..]) {
            Ok(request) => match run_guarded_cycle(&request) {
                Ok(report) => println!(
                    "{}",
                    serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string())
                ),
                Err(err) => {
                    eprintln!(
                        "{}",
                        serde_json::json!({ "ok": false, "error": err.to_string() })
                    );
                    std::process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("{}", serde_json::json!({ "ok": false, "error": err }));
                std::process::exit(1);
            }
        },
        "show" => match show_seed_state_json() {
            Ok(payload) => println!("{payload}"),
            Err(err) => {
                eprintln!(
                    "{}",
                    serde_json::json!({ "ok": false, "error": err.to_string() })
                );
                std::process::exit(1);
            }
        },
        "demo" => match run_guarded_cycle(&CycleRequest::default()) {
            Ok(report) => println!(
                "{}",
                serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(err) => {
                eprintln!(
                    "{}",
                    serde_json::json!({ "ok": false, "error": err.to_string() })
                );
                std::process::exit(1);
            }
        },
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}
