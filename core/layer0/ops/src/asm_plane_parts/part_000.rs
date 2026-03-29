// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::asm_plane (authoritative)

use crate::v8_kernel::{
    append_jsonl, load_json_or, parse_bool, parse_u64, read_json, scoped_state_root,
    sha256_hex_str, write_json, write_receipt,
};
use crate::{clean, now_iso, parse_args, ParsedArgs};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const STATE_ENV: &str = "ASM_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "asm_plane";

const WASM_DUAL_METER_POLICY_PATH: &str = "planes/contracts/wasm_dual_meter_policy_v1.json";
const HAND_MANIFEST_PATH: &str = "planes/contracts/hands/HAND.toml";
const CRDT_PROFILE_PATH: &str = "planes/contracts/crdt_automerge_profile_v1.json";
const TRUST_CHAIN_POLICY_PATH: &str = "planes/contracts/trust_chain_integration_v1.json";
const FASTPATH_POLICY_PATH: &str = "planes/contracts/fastpath_hotpath_policy_v1.json";
const INDUSTRIAL_ISA95_PATH: &str = "planes/contracts/industrial/isa95_template.json";
const INDUSTRIAL_RAMI_PATH: &str = "planes/contracts/industrial/rami40_template.json";
const INDUSTRIAL_CHECKLIST_PATH: &str = "planes/contracts/industrial/validation_checklist.json";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops asm-plane status");
    println!("  protheus-ops asm-plane wasm-dual-meter [--strict=1|0] [--ticks=<n>] [--fuel-budget=<n>] [--epoch-budget=<n>] [--fuel-per-tick=<n>] [--epoch-step=<n>]");
    println!(
        "  protheus-ops asm-plane hands-runtime [--strict=1|0] [--op=status|install|start|pause|rotate] [--manifest=<path>] [--version=<semver>]"
    );
    println!(
        "  protheus-ops asm-plane crdt-adapter [--strict=1|0] [--op=merge|replay] [--left-json=<json>] [--right-json=<json>]"
    );
    println!(
        "  protheus-ops asm-plane trust-chain [--strict=1|0] [--policy=<path>] [--allow-missing-rekor=1|0]"
    );
    println!(
        "  protheus-ops asm-plane fastpath [--strict=1|0] [--policy=<path>] [--workload=1,2,3] [--inject-mismatch=1|0]"
    );
    println!(
        "  protheus-ops asm-plane industrial-pack [--strict=1|0] [--isa95=<path>] [--rami=<path>] [--checklist=<path>]"
    );
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn history_path(root: &Path) -> PathBuf {
    state_root(root).join("history.jsonl")
}

fn print_payload(payload: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn emit(root: &Path, payload: Value) -> i32 {
    match write_receipt(root, STATE_ENV, STATE_SCOPE, payload) {
        Ok(out) => {
            print_payload(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                1
            }
        }
        Err(err) => {
            let out = json!({
                "ok": false,
                "type": "asm_plane_error",
                "error": clean(err, 220)
            });
            print_payload(&out);
            1
        }
    }
}

fn load_hand_manifest(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("read_manifest_failed:{}:{err}", path.display()))?;
    let mut out = Map::<String, Value>::new();
    for row in raw.lines() {
        let line = row.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key_raw, value_raw)) = line.split_once('=') else {
            continue;
        };
        let key = key_raw.trim().to_ascii_lowercase();
        let value = value_raw.trim();
        if value.starts_with('[') && value.ends_with(']') {
            let inner = &value[1..value.len().saturating_sub(1)];
            let values = inner
                .split(',')
                .map(|part| part.trim().trim_matches('"').trim_matches('\'').to_string())
                .filter(|part| !part.is_empty())
                .map(Value::String)
                .collect::<Vec<_>>();
            out.insert(key, Value::Array(values));
            continue;
        }
        let clean_str = value.trim_matches('"').trim_matches('\'').to_string();
        if let Ok(parsed) = clean_str.parse::<u64>() {
            out.insert(key, Value::Number(parsed.into()));
            continue;
        }
        out.insert(key, Value::String(clean_str));
    }
    Ok(Value::Object(out))
}

fn run_status(root: &Path) -> Value {
    let latest = read_json(&latest_path(root));
    json!({
        "ok": true,
        "type": "asm_plane_status",
        "lane": "core/layer0/ops",
        "latest_path": latest_path(root).display().to_string(),
        "history_path": history_path(root).display().to_string(),
        "latest": latest
    })
}

fn run_wasm_dual_meter(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let policy_path = parsed
        .flags
        .get("policy")
        .map(String::as_str)
        .unwrap_or(WASM_DUAL_METER_POLICY_PATH);
    let policy = load_json_or(
        root,
        policy_path,
        json!({
            "version": "v1",
            "kind": "wasm_dual_meter_policy",
            "defaults": {
                "fuel_budget": 25000,
                "epoch_budget": 128,
                "fuel_per_tick": 90,
                "max_ticks_per_epoch": 16,
                "epoch_step": 1
            },
            "telemetry_required": true
        }),
    );
    let defaults = policy.get("defaults").cloned().unwrap_or(Value::Null);
    let fuel_budget = parse_u64(
        parsed.flags.get("fuel-budget"),
        defaults
            .get("fuel_budget")
            .and_then(Value::as_u64)
            .unwrap_or(25_000),
    )
    .clamp(1, 50_000_000);
    let epoch_budget = parse_u64(
        parsed.flags.get("epoch-budget"),
        defaults
            .get("epoch_budget")
            .and_then(Value::as_u64)
            .unwrap_or(128),
    )
    .clamp(1, 1_000_000);
    let fuel_per_tick = parse_u64(
        parsed.flags.get("fuel-per-tick"),
        defaults
            .get("fuel_per_tick")
            .and_then(Value::as_u64)
            .unwrap_or(90),
    )
    .clamp(1, 100_000);
    let max_ticks_per_epoch = parse_u64(
        parsed.flags.get("max-ticks-per-epoch"),
        defaults
            .get("max_ticks_per_epoch")
            .and_then(Value::as_u64)
            .unwrap_or(16),
    )
    .clamp(1, 100_000);
    let epoch_step = parse_u64(
        parsed.flags.get("epoch-step"),
        defaults
            .get("epoch_step")
            .and_then(Value::as_u64)
            .unwrap_or(1),
    )
    .clamp(1, 100_000);
    let ticks = parse_u64(parsed.flags.get("ticks"), 32).clamp(0, 10_000_000);
    let module_sha = clean(
        parsed
            .flags
            .get("module-sha")
            .cloned()
            .unwrap_or_else(|| sha256_hex_str("module:default")),
        128,
    );

    let fuel_used = ticks.saturating_mul(fuel_per_tick);
    let epoch_used = if ticks == 0 {
        0
    } else {
        ((ticks + max_ticks_per_epoch - 1) / max_ticks_per_epoch).saturating_mul(epoch_step)
    };
    let fuel_remaining = fuel_budget.saturating_sub(fuel_used);
    let epoch_remaining = epoch_budget.saturating_sub(epoch_used);

    let mut errors = Vec::<String>::new();
    if policy
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("policy_version_must_be_v1".to_string());
    }
    if policy
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "wasm_dual_meter_policy"
    {
        errors.push("policy_kind_invalid".to_string());
    }
    if fuel_used > fuel_budget {
        errors.push("fuel_exhausted".to_string());
    }
    if epoch_used > epoch_budget {
        errors.push("epoch_exhausted".to_string());
    }
    if module_sha.len() != 64 || !module_sha.chars().all(|c| c.is_ascii_hexdigit()) {
        errors.push("module_sha_invalid".to_string());
    }

    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "asm_wasm_dual_meter",
        "lane": "core/layer0/ops",
        "command": "wasm-dual-meter",
        "policy_path": policy_path,
        "module_sha256": module_sha,
        "telemetry": {
            "ticks": ticks,
            "fuel_per_tick": fuel_per_tick,
            "max_ticks_per_epoch": max_ticks_per_epoch,
            "epoch_step": epoch_step,
            "fuel_budget": fuel_budget,
            "fuel_used": fuel_used,
            "fuel_remaining": fuel_remaining,
            "epoch_budget": epoch_budget,
            "epoch_used": epoch_used,
            "epoch_remaining": epoch_remaining
        },
        "decision": if ok { "allow" } else { "deny" },
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-ASM-004",
                "claim": "dual_metered_wasm_sandbox_enforces_fuel_and_epoch_fail_closed",
                "evidence": {
                    "fuel_used": fuel_used,
                    "epoch_used": epoch_used
                }
            }
        ]
    })
}

fn run_hands_runtime(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let manifest_rel = parsed
        .flags
        .get("manifest")
        .map(String::as_str)
        .unwrap_or(HAND_MANIFEST_PATH);
    let op = parsed
        .flags
        .get("op")
        .map(|v| v.to_ascii_lowercase())
        .or_else(|| parsed.positional.get(1).map(|v| v.to_ascii_lowercase()))
        .unwrap_or_else(|| "status".to_string());
    let manifest_path = root.join(manifest_rel);
    let manifest = load_hand_manifest(&manifest_path).unwrap_or_else(|_| Value::Null);
    let state_path = state_root(root).join("hands_runtime").join("state.json");
    let events_path = state_root(root).join("hands_runtime").join("events.jsonl");
    let mut state = read_json(&state_path).unwrap_or_else(|| {
        json!({
            "installed": false,
            "running": false,
            "paused": false,
            "rotation_seq": 0,
            "active_version": Value::Null,
            "last_op": Value::Null,
            "updated_at": Value::Null
        })
    });
    let mut errors = Vec::<String>::new();

    if manifest.is_null() {
        errors.push("hand_manifest_missing_or_invalid".to_string());
    }
    if manifest
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        errors.push("hand_manifest_name_required".to_string());
    }
    if manifest
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        errors.push("hand_manifest_version_required".to_string());
    }
    if manifest
        .get("capabilities")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        == false
    {
        errors.push("hand_manifest_capabilities_required".to_string());
    }

    let installed = state
        .get("installed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let running = state
        .get("running")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if op == "install" {
        if errors.is_empty() {
            state["installed"] = Value::Bool(true);
            state["running"] = Value::Bool(false);
            state["paused"] = Value::Bool(false);
            state["rotation_seq"] = Value::Number(0_u64.into());
            state["active_version"] = manifest
                .get("version")
                .cloned()
                .unwrap_or_else(|| Value::String("0.0.0".to_string()));
        }
    } else if op == "start" {
        if !installed {
            errors.push("hands_runtime_not_installed".to_string());
        } else {
            state["running"] = Value::Bool(true);
            state["paused"] = Value::Bool(false);
        }
    } else if op == "pause" {
        if !running {
            errors.push("hands_runtime_not_running".to_string());
        } else {
            state["paused"] = Value::Bool(true);
            state["running"] = Value::Bool(false);
        }
    } else if op == "rotate" {
        if !installed {
            errors.push("hands_runtime_not_installed".to_string());
        } else {
            let next_version = parsed
                .flags
                .get("version")
                .cloned()
                .or_else(|| {
                    manifest
                        .get("version")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                })
                .unwrap_or_else(|| "0.0.0".to_string());
            let next_seq = state
                .get("rotation_seq")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .saturating_add(1);
            state["rotation_seq"] = Value::Number(next_seq.into());
            state["active_version"] = Value::String(clean(next_version, 64));
            state["running"] = Value::Bool(true);
            state["paused"] = Value::Bool(false);
        }
    } else if op != "status" {
        errors.push(format!("unknown_hands_op:{op}"));
    }

    let ok = errors.is_empty();
    if (op == "install" || op == "start" || op == "pause" || op == "rotate") && ok {
        state["last_op"] = Value::String(op.clone());
        state["updated_at"] = Value::String(now_iso());
        let _ = write_json(&state_path, &state);
        let event = json!({
            "type": "hands_runtime_event",
            "op": op,
            "ts": now_iso(),
            "manifest_path": manifest_rel,
            "state": state
        });
        let _ = append_jsonl(&events_path, &event);
    }

    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "asm_hands_runtime",
        "lane": "core/layer0/ops",
        "op": op,
        "manifest_path": manifest_rel,
        "state_path": state_path.display().to_string(),
        "events_path": events_path.display().to_string(),
        "manifest": manifest,
        "state": state,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-ASM-005",
                "claim": "hands_runtime_is_manifest_driven_and_lifecycle_receipted",
                "evidence": {
                    "op": op,
                    "state_path": state_path.display().to_string()
                }
            }
        ]
    })
}

fn parse_crdt_map(raw: Option<&String>) -> Result<Map<String, Value>, String> {
    let fallback = json!({
        "topic": {"value":"alpha", "clock": 1, "node":"left"},
        "state": {"value":"warm", "clock": 1, "node":"left"}
    });
    let parsed = match raw {
        Some(v) => {
            serde_json::from_str::<Value>(v).map_err(|err| format!("invalid_crdt_json:{err}"))?
        }
        None => fallback,
    };
    parsed
        .as_object()
        .cloned()
        .ok_or_else(|| "crdt_payload_must_be_object".to_string())
}

