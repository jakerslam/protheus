// Layer ownership: core/layer0/desktop (authoritative)
// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::path::PathBuf;

const MAX_COMMAND_LEN: usize = 64;
const SUPPORTED_COMMANDS: [&str; 2] = ["status", "launch"];

fn sanitize_command_token(input: &str) -> String {
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
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(MAX_COMMAND_LEN)
        .collect()
}

fn normalize_command(input: &str) -> String {
    match sanitize_command_token(input).as_str() {
        "" => "status".to_string(),
        "check" => "status".to_string(),
        "ls" => "status".to_string(),
        "run" => "launch".to_string(),
        "start" => "launch".to_string(),
        "restart" => "launch".to_string(),
        other => other.to_string(),
    }
}

fn print_json(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
    );
}

fn unknown_command_payload(command: &str) -> serde_json::Value {
    let body = serde_json::json!({
        "ok": false,
        "type": "infring_desktop_unknown_command",
        "error": "unknown_command",
        "command": command,
        "supported_commands": SUPPORTED_COMMANDS,
    });
    serde_json::json!({
        "ok": false,
        "type": "infring_desktop_unknown_command",
        "error": "unknown_command",
        "command": command,
        "supported_commands": SUPPORTED_COMMANDS,
        "receipt_hash": infring_desktop::deterministic_receipt_hash(&body)
    })
}

fn print_help() {
    println!("Usage:");
    for command in SUPPORTED_COMMANDS {
        println!("  infring-desktop {command}");
    }
}

fn main() {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = args
        .first()
        .map(|value| normalize_command(value))
        .unwrap_or_else(|| "status".to_string());
    let payload = match command.as_str() {
        "status" => infring_desktop::status_payload(&cwd),
        "launch" => infring_desktop::launch_payload(&cwd),
        "help" | "--help" | "-h" => {
            print_help();
            return;
        }
        _ => unknown_command_payload(command.as_str()),
    };
    print_json(&payload);
}
