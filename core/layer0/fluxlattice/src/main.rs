// SPDX-License-Identifier: Apache-2.0
use fluxlattice::{init_state, morph, settle, status_map};
use std::env;

const MAX_TOKEN_LEN: usize = 64;

fn sanitize_token(input: &str) -> String {
    input
        .chars()
        .filter(|c| !matches!(*c, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'))
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

fn print_json(map: &std::collections::BTreeMap<String, String>) {
    println!(
        "{}",
        serde_json::to_string(map).unwrap_or_else(|_| "{}".to_string())
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = normalize_command(args.get(1).map(|v| v.as_str()).unwrap_or("status").trim());

    let mut state = init_state("fluxlattice_core");
    match cmd.as_str() {
        "init" => {
            state.metadata.insert("command".into(), "init".into());
            let status = status_map(&state);
            print_json(&status);
        }
        "settle" => {
            state = settle(state, "binary");
            state.metadata.insert("command".into(), "settle".into());
            let status = status_map(&state);
            print_json(&status);
        }
        "morph" => {
            let mode = sanitize_token(args.get(2).map(|v| v.as_str()).unwrap_or("dynamic"));
            state = settle(state, "binary");
            state = morph(state, mode.as_str());
            state.metadata.insert("command".into(), "morph".into());
            let status = status_map(&state);
            print_json(&status);
        }
        "status" => {
            state.metadata.insert("command".into(), "status".into());
            let status = status_map(&state);
            print_json(&status);
        }
        _ => {
            eprintln!(
                "{}",
                serde_json::json!({
                    "ok": false,
                    "error": "unsupported_command",
                    "command": cmd
                })
            );
            std::process::exit(2);
        }
    }
}
