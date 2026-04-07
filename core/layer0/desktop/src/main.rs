// Layer ownership: core/layer0/desktop (authoritative)
// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::path::PathBuf;

fn print_json(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
    );
}

fn main() {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = args
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let payload = match command.as_str() {
        "status" => infring_desktop::status_payload(&cwd),
        "launch" => infring_desktop::launch_payload(&cwd),
        "help" | "--help" | "-h" => {
            println!("Usage:");
            println!("  infring-desktop status");
            println!("  infring-desktop launch");
            return;
        }
        _ => serde_json::json!({
            "ok": false,
            "error": "unknown_command",
            "command": command
        }),
    };
    print_json(&payload);
}
