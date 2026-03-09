// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/security (authoritative)

use crate::clean;
use serde_json::{json, Value};
use std::path::Path;

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let rest = if argv.is_empty() { &[][..] } else { &argv[1..] };

    let (payload, code) = match cmd.as_str() {
        "guard" => infring_layer1_security::run_guard(root, rest),
        "anti-sabotage-shield" | "anti_sabotage_shield" => {
            infring_layer1_security::run_anti_sabotage_shield(root, rest)
        }
        "constitution-guardian" | "constitution_guardian" => {
            infring_layer1_security::run_constitution_guardian(root, rest)
        }
        "remote-emergency-halt" | "remote_emergency_halt" => {
            infring_layer1_security::run_remote_emergency_halt(root, rest)
        }
        "soul-token-guard" | "soul_token_guard" => {
            infring_layer1_security::run_soul_token_guard(root, rest)
        }
        "integrity-reseal" | "integrity_reseal" => {
            infring_layer1_security::run_integrity_reseal(root, rest)
        }
        "integrity-reseal-assistant" | "integrity_reseal_assistant" => {
            infring_layer1_security::run_integrity_reseal_assistant(root, rest)
        }
        "capability-lease" | "capability_lease" => {
            infring_layer1_security::run_capability_lease(root, rest)
        }
        "startup-attestation" | "startup_attestation" => {
            infring_layer1_security::run_startup_attestation(root, rest)
        }
        "status" => (
            json!({
                "ok": true,
                "type": "security_plane_status",
                "lane": "core/layer1/security",
                "commands": [
                    "guard",
                    "anti-sabotage-shield",
                    "constitution-guardian",
                    "remote-emergency-halt",
                    "soul-token-guard",
                    "integrity-reseal",
                    "integrity-reseal-assistant",
                    "capability-lease",
                    "startup-attestation"
                ]
            }),
            0,
        ),
        _ => (
            json!({
                "ok": false,
                "type": "security_plane_error",
                "error": format!("unknown_command:{}", clean(cmd, 120)),
                "usage": [
                    "protheus-ops security-plane guard [--files=<a,b,c>] [--strict=1|0]",
                    "protheus-ops security-plane anti-sabotage-shield <snapshot|verify|watch|status> [flags]",
                    "protheus-ops security-plane constitution-guardian <init-genesis|propose-change|approve-change|veto-change|run-gauntlet|activate-change|enforce-inheritance|emergency-rollback|status> [flags]",
                    "protheus-ops security-plane remote-emergency-halt <status|sign-halt|sign-purge|receive|receive-b64> [flags]",
                    "protheus-ops security-plane soul-token-guard <issue|stamp-build|verify|status> [flags]",
                    "protheus-ops security-plane integrity-reseal <check|apply> [flags]",
                    "protheus-ops security-plane integrity-reseal-assistant <run|status> [flags]",
                    "protheus-ops security-plane capability-lease <issue|verify|consume> [flags]",
                    "protheus-ops security-plane startup-attestation <issue|verify|status> [flags]"
                ]
            }),
            2,
        ),
    };

    print_json(&payload);
    code
}
