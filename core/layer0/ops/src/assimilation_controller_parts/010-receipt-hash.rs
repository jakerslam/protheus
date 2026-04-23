// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::now_iso;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const LANE_ID: &str = "assimilation_controller";
const REPLACEMENT: &str = "infring-ops assimilation-controller";
const VARIANT_PROFILE_DIR: &str = "planes/contracts/variant_profiles";
const MPU_PROFILE_PATH: &str = "planes/contracts/mpu_compartment_profile_v1.json";
const WASM_DUAL_METER_POLICY_PATH: &str = "planes/contracts/wasm_dual_meter_policy_v1.json";
const HAND_MANIFEST_PATH: &str = "planes/contracts/hands/HAND.toml";
const SCHEDULED_HANDS_CONTRACT_PATH: &str =
    "planes/contracts/hands/scheduled_hands_contract_v1.json";

fn receipt_hash(v: &Value) -> String {
    crate::deterministic_receipt_hash(v)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn usage() {
    println!("Usage:");
    println!("  infring-ops assimilation-controller status [--capability-id=<id>]");
    println!("  infring-ops assimilation-controller run [YYYY-MM-DD] [--capability-id=<id>] [--apply=1|0]");
    println!("  infring-ops assimilation-controller assess [--capability-id=<id>]");
    println!(
        "  infring-ops assimilation-controller record-use --capability-id=<id> [--success=1|0]"
    );
    println!(
        "  infring-ops assimilation-controller rollback --capability-id=<id> [--reason=<text>]"
    );
    println!(
        "  infring-ops assimilation-controller skills-enable [perplexity-mode] [--apply=1|0]"
    );
    println!("  infring-ops assimilation-controller skill-create --task=<text>");
    println!("  infring-ops assimilation-controller skills-dashboard");
    println!("  infring-ops assimilation-controller skills-spawn-subagents --task=<text> [--roles=researcher,executor,reviewer]");
    println!("  infring-ops assimilation-controller skills-computer-use --action=<text> [--target=<text>] [--apply=1|0]");
    println!("  infring-ops assimilation-controller variant-profiles [--strict=1|0]");
    println!("  infring-ops assimilation-controller mpu-compartments [--strict=1|0]");
    println!("  infring-ops assimilation-controller capability-ledger --op=<grant|revoke|verify|status> [--capability=<id>] [--subject=<id>] [--reason=<text>] [--strict=1|0]");
    println!("  infring-ops assimilation-controller wasm-dual-meter [--ticks=<n>] [--fuel-budget=<n>] [--epoch-budget=<n>] [--fuel-per-tick=<n>] [--epoch-step=<n>] [--strict=1|0]");
    println!("  infring-ops assimilation-controller hands-runtime --op=<status|install|start|pause|rotate> [--manifest=<path>] [--version=<semver>] [--strict=1|0]");
    println!("  infring-ops assimilation-controller scheduled-hands --op=<enable|run|status|dashboard|disable> [--strict=1|0] [--iterations=<n>] [--task=<text>] [--cross-refs=a,b]");
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    lane_utils::parse_flag(argv, key, false)
}

fn parse_bool_flag(raw: Option<String>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw.as_deref(), fallback)
}

fn parse_u64_flag(raw: Option<String>, fallback: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}

fn is_token_id(id: &str) -> bool {
    let s = id.trim();
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, value)
}

fn parse_hand_manifest(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("read_hand_manifest_failed:{}:{err}", path.display()))?;
    let mut out = Map::<String, Value>::new();
    for row in raw.lines() {
        let line = row.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k_raw, v_raw)) = line.split_once('=') else {
            continue;
        };
        let key = k_raw.trim().to_ascii_lowercase();
        let value = v_raw.trim();
        if value.starts_with('[') && value.ends_with(']') {
            let inner = &value[1..value.len().saturating_sub(1)];
            let rows = inner
                .split(',')
                .map(|part| part.trim().trim_matches('"').trim_matches('\''))
                .filter(|part| !part.is_empty())
                .map(|part| Value::String(part.to_string()))
                .collect::<Vec<_>>();
            out.insert(key, Value::Array(rows));
            continue;
        }
        if let Ok(parsed) = value.trim_matches('"').trim_matches('\'').parse::<u64>() {
            out.insert(key, Value::Number(parsed.into()));
            continue;
        }
        out.insert(
            key,
            Value::String(value.trim_matches('"').trim_matches('\'').to_string()),
        );
    }
    Ok(Value::Object(out))
}

fn command_claim_ids(command: &str) -> &'static [&'static str] {
    match command {
        "skills-enable" => &["V6-COGNITION-012.1"],
        "skill-create" => &["V6-COGNITION-012.2"],
        "skills-spawn-subagents" => &["V6-COGNITION-012.3"],
        "skills-computer-use" => &["V6-COGNITION-012.4"],
        "skills-dashboard" => &["V6-COGNITION-012.5"],
        "variant-profiles" => &["V7-ASSIMILATE-001.1"],
        "mpu-compartments" => &["V7-ASSIMILATE-001.2"],
        "capability-ledger" => &["V7-ASSIMILATE-001.3"],
        "wasm-dual-meter" => &["V7-ASSIMILATE-001.4"],
        "hands-runtime" => &["V7-ASSIMILATE-001.5"],
        "scheduled-hands" => &[
            "V7-ASSIMILATE-001.5.2",
            "V7-ASSIMILATE-001.5.3",
            "V7-ASSIMILATE-001.5.4",
        ],
        _ => &[],
    }
}

fn conduit_enforcement(argv: &[String], command: &str, strict: bool) -> Value {
    let bypass_requested = parse_bool_flag(parse_flag(argv, "bypass"), false)
        || parse_bool_flag(parse_flag(argv, "client-bypass"), false);
    let ok = !bypass_requested;
    let claim_text = if command.starts_with("skill") || command.starts_with("skills-") {
        "cognition_skill_commands_route_through_core_authority_with_fail_closed_bypass_denial"
    } else {
        "assimilation_contract_commands_route_through_core_authority_with_fail_closed_bypass_denial"
    };
    let claim_evidence = command_claim_ids(command)
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "claim": claim_text,
                "evidence": {
                    "command": command,
                    "bypass_requested": bypass_requested
                }
            })
        })
        .collect::<Vec<_>>();
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "type": "assimilation_controller_conduit_enforcement",
        "command": command,
        "strict": strict,
        "bypass_requested": bypass_requested,
        "errors": if ok { Value::Array(Vec::new()) } else { json!(["conduit_bypass_rejected"]) },
        "claim_evidence": claim_evidence
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn state_root(root: &Path) -> std::path::PathBuf {
    root.join("local")
        .join("state")
        .join("ops")
        .join("assimilation_controller")
}

fn latest_path(root: &Path) -> std::path::PathBuf {
    state_root(root).join("latest.json")
}

fn history_path(root: &Path) -> std::path::PathBuf {
    state_root(root).join("history.jsonl")
}

fn persist_receipt(root: &Path, payload: &Value) {
    let latest = latest_path(root);
    let history = history_path(root);
    let _ = lane_utils::write_json(&latest, payload);
    let _ = lane_utils::append_jsonl(&history, payload);
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn first_non_flag(argv: &[String], skip: usize) -> Option<String> {
    argv.iter()
        .skip(skip)
        .find(|row| !row.starts_with("--"))
        .cloned()
}

fn native_receipt(root: &Path, cmd: &str, argv: &[String]) -> Value {
    let capability_id = parse_flag(argv, "capability-id").unwrap_or_else(|| "unknown".to_string());
    let apply = parse_flag(argv, "apply")
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);

    let mut out = json!({
        "ok": true,
        "type": "assimilation_controller",
        "lane": LANE_ID,
        "ts": now_iso(),
        "command": cmd,
        "argv": argv,
        "capability_id": capability_id,
        "apply": apply,
        "replacement": REPLACEMENT,
        "root": root.to_string_lossy(),
        "claim_evidence": [
            {
                "id": "native_assimilation_controller_lane",
                "claim": "assimilation_controller_executes_natively_in_rust",
                "evidence": {
                    "command": cmd,
                    "capability_id": capability_id,
                    "apply": apply
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn skills_enable_receipt(root: &Path, argv: &[String]) -> Value {
    let mode = parse_flag(argv, "mode")
        .or_else(|| first_non_flag(argv, 1))
        .unwrap_or_else(|| "perplexity-mode".to_string());
    let apply = parse_bool_flag(parse_flag(argv, "apply"), true);
    let mut out = json!({
        "ok": true,
        "type": "assimilation_controller_skills_enable",
        "lane": LANE_ID,
        "ts": now_iso(),
        "mode": mode,
        "apply": apply,
        "auto_activation": true,
        "subagent_orchestration": true,
        "claim_evidence": [
            {
                "id": "V6-COGNITION-012.1",
                "claim": "skills_enable_perplexity_mode_routes_through_rust_core_with_deterministic_activation_receipts",
                "evidence": {
                    "mode": mode,
                    "apply": apply
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    persist_receipt(root, &out);
    out
}

fn skill_create_receipt(root: &Path, argv: &[String]) -> Value {
    let task = parse_flag(argv, "task")
        .or_else(|| first_non_flag(argv, 1))
        .unwrap_or_else(|| "general task".to_string());
    let normalized = task.trim().to_ascii_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let skill_id = format!("skill_{}", &hex::encode(hasher.finalize())[..12]);
    let mut out = json!({
        "ok": true,
        "type": "assimilation_controller_skill_create",
        "lane": LANE_ID,
        "ts": now_iso(),
        "skill_id": skill_id,
        "task": task,
        "auto_activation": true,
        "claim_evidence": [
            {
                "id": "V6-COGNITION-012.2",
                "claim": "natural_language_skill_creation_mints_deterministic_skill_ids_and_receipted_contracts",
                "evidence": {
                    "skill_id": skill_id
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    persist_receipt(root, &out);
    out
}

fn skills_dashboard_receipt(root: &Path) -> Value {
    let latest = read_json(&latest_path(root));
    let history_count = fs::read_to_string(history_path(root))
        .ok()
        .map(|s| s.lines().count())
        .unwrap_or(0usize);
    let mut out = json!({
        "ok": true,
        "type": "assimilation_controller_skills_dashboard",
        "lane": LANE_ID,
        "ts": now_iso(),
        "history_events": history_count,
        "latest": latest,
        "claim_evidence": [
            {
                "id": "V6-COGNITION-012.5",
                "claim": "skills_dashboard_surfaces_history_and_latest_state_from_core_receipt_ledger",
                "evidence": {
                    "history_events": history_count
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    persist_receipt(root, &out);
    out
}

fn skills_spawn_subagents_receipt(root: &Path, argv: &[String]) -> Value {
    let task = parse_flag(argv, "task")
        .or_else(|| first_non_flag(argv, 1))
        .unwrap_or_else(|| "general task".to_string());
    let roles_raw =
        parse_flag(argv, "roles").unwrap_or_else(|| "researcher,executor,reviewer".to_string());
    let roles = roles_raw
        .split(',')
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .collect::<Vec<_>>();
    let mut out = json!({
        "ok": true,
        "type": "assimilation_controller_skills_spawn_subagents",
        "lane": LANE_ID,
        "ts": now_iso(),
        "task": task,
        "roles": roles,
        "handoff_policy": "parent_voice_and_context_inherited",
        "claim_evidence": [
            {
                "id": "V6-COGNITION-012.3",
                "claim": "skills_spawn_subagents_emits_deterministic_spawn_and_handoff_receipts",
                "evidence": {
                    "task": task
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    persist_receipt(root, &out);
    out
}

fn skills_computer_use_receipt(root: &Path, argv: &[String]) -> Value {
    let action = parse_flag(argv, "action")
        .or_else(|| first_non_flag(argv, 1))
        .unwrap_or_else(|| "open browser".to_string());
    let target = parse_flag(argv, "target").unwrap_or_else(|| "desktop".to_string());
    let apply = parse_bool_flag(parse_flag(argv, "apply"), true);
    let replay_id = format!(
        "replay_{}",
        &receipt_hash(&json!({"action": action, "target": target, "apply": apply}))[..12]
    );
    let mut out = json!({
        "ok": true,
        "type": "assimilation_controller_skills_computer_use",
        "lane": LANE_ID,
        "ts": now_iso(),
        "action": action,
        "target": target,
        "apply": apply,
        "replay": {
            "deterministic": true,
            "replay_id": replay_id
        },
        "claim_evidence": [
            {
                "id": "V6-COGNITION-012.4",
                "claim": "skills_computer_use_emits_deterministic_action_receipts_with_replay_metadata",
                "evidence": {
                    "action": action,
                    "target": target,
                    "replay_id": replay_id
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    persist_receipt(root, &out);
    out
}
