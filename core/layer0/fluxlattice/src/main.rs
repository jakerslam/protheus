// SPDX-License-Identifier: Apache-2.0
use fluxlattice::{init_state, morph, settle, status_map};
use std::env;

const MAX_TOKEN_LEN: usize = 64;

fn sanitize_token(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(MAX_TOKEN_LEN)
        .collect()
}

fn normalize_command(raw: &str) -> String {
    let normalized = sanitize_token(raw).to_lowercase();
    match normalized.as_str() {
        "" => "status".to_string(),
        "check" => "status".to_string(),
        "run" => "morph".to_string(),
        _ => normalized,
    }
}

fn normalize_mode(raw: Option<&str>) -> String {
    let normalized = sanitize_token(raw.unwrap_or("dynamic")).to_lowercase();
    match normalized.as_str() {
        "" => "dynamic".to_string(),
        "dyn" | "dynamic" => "dynamic".to_string(),
        "static" | "stable" => "static".to_string(),
        "coalesce" | "coalesced" => "coalesced".to_string(),
        _ => "dynamic".to_string(),
    }
}

fn print_json(map: &std::collections::BTreeMap<String, String>) {
    println!(
        "{}",
        serde_json::to_string(map).unwrap_or_else(|_| "{}".to_string())
    );
}

fn emit_state(mut state: fluxlattice::FluxState, command: &str) {
    state.metadata.insert("command".into(), command.to_string());
    print_json(&status_map(&state));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = normalize_command(args.get(1).map(|v| v.as_str()).unwrap_or("status").trim());

    let mut state = init_state("fluxlattice_core");
    match cmd.as_str() {
        "init" => {
            emit_state(state, "init");
        }
        "settle" => {
            state = settle(state, "binary");
            emit_state(state, "settle");
        }
        "morph" => {
            let mode = normalize_mode(args.get(2).map(|v| v.as_str()));
            state = settle(state, "binary");
            state = morph(state, mode.as_str());
            emit_state(state, "morph");
        }
        "status" => {
            emit_state(state, "status");
        }
        _ => {
            eprintln!(
                "{}",
                serde_json::json!({
                    "ok": false,
                    "error": "unsupported_command",
                    "command": cmd,
                    "supported_commands": ["init", "settle", "morph", "status"]
                })
            );
            std::process::exit(2);
        }
    }
}
