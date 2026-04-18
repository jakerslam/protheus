// SPDX-License-Identifier: Apache-2.0
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use protheus_graph_core_v1::{run_workflow_json, viz_dot};
use std::env;
use std::fs;

const MAX_ARG_KEY_LEN: usize = 48;
const MAX_YAML_BYTES: usize = 32 * 1024;

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
    if text.chars().count() > max_len {
        text = text.chars().take(max_len).collect();
    }
    text
}

fn parse_arg(args: &[String], key: &str) -> Option<String> {
    let key = sanitize_text(key, MAX_ARG_KEY_LEN, false);
    for arg in args {
        if let Some((k, v)) = arg.split_once('=') {
            if sanitize_text(k, MAX_ARG_KEY_LEN, false) == key {
                let value = sanitize_text(v, MAX_YAML_BYTES, false);
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn load_yaml(args: &[String]) -> Result<String, String> {
    if let Some(v) = parse_arg(args, "--yaml") {
        if v.len() > MAX_YAML_BYTES {
            return Err("yaml_payload_too_large".to_string());
        }
        return Ok(v);
    }
    if let Some(v) = parse_arg(args, "--yaml-base64") {
        let bytes = BASE64_STANDARD
            .decode(v.as_bytes())
            .map_err(|e| format!("base64_decode_failed:{e}"))?;
        if bytes.len() > MAX_YAML_BYTES {
            return Err("yaml_base64_payload_too_large".to_string());
        }
        let text = String::from_utf8(bytes).map_err(|e| format!("utf8_decode_failed:{e}"))?;
        if text.trim().is_empty() {
            return Err("yaml_payload_empty".to_string());
        }
        return Ok(text);
    }
    if let Some(v) = parse_arg(args, "--yaml-file") {
        let metadata =
            fs::metadata(v.as_str()).map_err(|e| format!("yaml_file_stat_failed:{e}"))?;
        if metadata.len() > MAX_YAML_BYTES as u64 {
            return Err("yaml_file_payload_too_large".to_string());
        }
        let text =
            fs::read_to_string(v.as_str()).map_err(|e| format!("yaml_file_read_failed:{e}"))?;
        if text.len() > MAX_YAML_BYTES {
            return Err("yaml_file_payload_too_large".to_string());
        }
        if text.trim().is_empty() {
            return Err("yaml_payload_empty".to_string());
        }
        return Ok(text);
    }
    Err("missing_yaml_payload".to_string())
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  graph_core run --yaml=<payload>");
    eprintln!("  graph_core viz --yaml=<payload>");
    eprintln!("  graph_core demo");
}

fn demo_yaml() -> String {
    serde_json::json!({
        "workflow_id": "graph_demo",
        "nodes": [
            {"id": "collect", "kind": "task"},
            {"id": "score", "kind": "task"},
            {"id": "ship", "kind": "task"}
        ],
        "edges": [
            {"from": "collect", "to": "score"},
            {"from": "score", "to": "ship"}
        ]
    })
    .to_string()
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args
        .first()
        .map(|value| sanitize_text(value, 24, true))
        .unwrap_or_else(|| "demo".to_string());

    match command.as_str() {
        "run" => match load_yaml(&args[1..]) {
            Ok(yaml) => match run_workflow_json(&yaml) {
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
        "viz" => match load_yaml(&args[1..]) {
            Ok(yaml) => match viz_dot(&yaml) {
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
            let yaml = demo_yaml();
            println!(
                "{}",
                run_workflow_json(&yaml).unwrap_or_else(|_| "{}".to_string())
            );
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}
