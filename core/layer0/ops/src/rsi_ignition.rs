// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::binary_blob_runtime;
use crate::directive_kernel;
use crate::network_protocol;
use crate::v8_kernel::{
    append_jsonl, parse_bool, parse_f64, print_json, read_json, scoped_state_root, sha256_hex_str,
    write_json, write_receipt,
};
use crate::{clean, now_iso, parse_args};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "RSI_IGNITION_STATE_ROOT";
const STATE_SCOPE: &str = "rsi_ignition";

include!("rsi_ignition_parts/010-state-and-io.rs");
include!("rsi_ignition_parts/020-commands.rs");

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops rsi-ignition status");
        println!("  protheus-ops rsi-ignition ignite [--proposal=<text>] [--module=<id>] [--apply=1|0] [--canary-pass=1|0] [--sim-regression=<0..1>]");
        println!("  protheus-ops rsi-ignition reflect [--drift=<0..1>] [--exploration=<0..1>]");
        println!(
            "  protheus-ops rsi-ignition swarm [--nodes=<n>] [--share-rate=<0..1>] [--apply=1|0]"
        );
        println!("  protheus-ops rsi-ignition evolve [--insight=<text>] [--module=<id>] [--apply=1|0] [--ignite-apply=1|0] [--night-cycle=1|0]");
        return 0;
    }

    match command.as_str() {
        "status" => command_status(root),
        "ignite" => command_ignite(root, &parsed),
        "reflect" => command_reflect(root, &parsed),
        "swarm" => command_swarm(root, &parsed),
        "evolve" => command_evolve(root, &parsed),
        _ => emit(
            root,
            json!({
                "ok": false,
                "type": "rsi_ignition_error",
                "lane": "core/layer0/ops",
                "error": "unknown_command",
                "command": command,
                "exit_code": 2
            }),
        ),
    }
}

#[cfg(test)]
#[path = "rsi_ignition_tests.rs"]
mod tests;
