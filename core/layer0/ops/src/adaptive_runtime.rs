// SPDX-License-Identifier: Apache-2.0
use crate::{clean, deterministic_receipt_hash, now_iso, parse_args};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const POLICY_REL: &str = "client/runtime/config/adaptive_runtime_policy.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdaptiveRuntimePolicy {
    schema_id: String,
    schema_version: String,
    enabled: bool,
    cadence_hours: u64,
    max_reflex_updates_per_tick: u64,
    max_strategy_updates_per_tick: u64,
    max_habit_updates_per_tick: u64,
    state_path: String,
    receipts_path: String,
}

pub fn run(root: &Path, args: &[String]) -> i32 {
    let parsed = parse_args(args);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    match cmd.as_str() {
        "tick" | "run" | "evaluate" => tick(root, &parsed),
        "status" => status(root, &parsed),
        _ => {
            print_json(&json!({
                "ok": false,
                "type": "adaptive_runtime",
                "error": "unknown_command",
                "command": cmd
            }));
            1
        }
    }
}

fn default_policy() -> AdaptiveRuntimePolicy {
    AdaptiveRuntimePolicy {
        schema_id: "adaptive_runtime_policy".to_string(),
        schema_version: "1.0".to_string(),
        enabled: true,
        cadence_hours: 4,
        max_reflex_updates_per_tick: 24,
        max_strategy_updates_per_tick: 12,
        max_habit_updates_per_tick: 16,
        state_path: "core/local/state/adaptive_runtime/latest.json".to_string(),
        receipts_path: "core/local/state/adaptive_runtime/receipts.jsonl".to_string(),
    }
}

fn load_policy(path: &Path) -> Result<AdaptiveRuntimePolicy, String> {
    if !path.exists() {
        return Ok(default_policy());
    }
    let raw = fs::read_to_string(path).map_err(|err| format!("policy_read_failed:{err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("policy_decode_failed:{err}"))
}

fn parse_u64_flag(raw: Option<&String>, fallback: u64) -> u64 {
    let Some(value) = raw else {
        return fallback;
    };
    value.trim().parse::<u64>().unwrap_or(fallback)
}

fn tick(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let policy_path = parsed
        .flags
        .get("policy")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(POLICY_REL));
    let policy = match load_policy(&policy_path) {
        Ok(value) => value,
        Err(err) => {
            print_json(&error_receipt(
                "adaptive_runtime",
                "policy_load_failed",
                Some(err),
            ));
            return 1;
        }
    };

    let reflex_updates = parse_u64_flag(parsed.flags.get("reflex-updates"), 0);
    let strategy_updates = parse_u64_flag(parsed.flags.get("strategy-updates"), 0);
    let habit_updates = parse_u64_flag(parsed.flags.get("habit-updates"), 0);
    let source = clean(
        parsed
            .flags
            .get("source")
            .cloned()
            .unwrap_or_else(|| "unspecified".to_string()),
        80,
    );

    let reflex_over = reflex_updates > policy.max_reflex_updates_per_tick;
    let strategy_over = strategy_updates > policy.max_strategy_updates_per_tick;
    let habit_over = habit_updates > policy.max_habit_updates_per_tick;
    let budget_blocked = reflex_over || strategy_over || habit_over;
    let admitted = policy.enabled && !budget_blocked;

    let ts = now_iso();
    let mut receipt = json!({
        "ok": admitted,
        "type": "adaptive_runtime_tick",
        "schema_id": "adaptive_runtime_receipt",
        "schema_version": "1.0",
        "ts": ts,
        "date": ts[..10].to_string(),
        "authority": "core.layer0.adaptive_runtime",
        "source": source,
        "cadence_hours": policy.cadence_hours,
        "policy_enabled": policy.enabled,
        "updates": {
            "reflex": reflex_updates,
            "strategy": strategy_updates,
            "habit": habit_updates
        },
        "caps": {
            "reflex": policy.max_reflex_updates_per_tick,
            "strategy": policy.max_strategy_updates_per_tick,
            "habit": policy.max_habit_updates_per_tick
        },
        "budget_blocked": budget_blocked,
        "blocking_reasons": {
            "reflex_cap_exceeded": reflex_over,
            "strategy_cap_exceeded": strategy_over,
            "habit_cap_exceeded": habit_over
        },
        "decision": if !policy.enabled {
            "policy_disabled"
        } else if budget_blocked {
            "budget_blocked"
        } else {
            "admitted"
        },
        "claim_evidence": [{
            "id": "adaptive_runtime_core_authority",
            "claim": "core_lane_enforces_adaptation_tick_caps_and_emits_deterministic_receipts",
            "evidence": {
                "policy_enabled": policy.enabled,
                "budget_blocked": budget_blocked,
                "cadence_hours": policy.cadence_hours
            }
        }],
        "policy": {
            "path": policy_path.to_string_lossy().to_string()
        }
    });
    receipt["receipt_hash"] = Value::String(deterministic_receipt_hash(&receipt));

    let state_path = root.join(clean(&policy.state_path, 260));
    let receipts_path = root.join(clean(&policy.receipts_path, 260));
    if write_json_atomic(&state_path, &receipt).is_err() {
        print_json(&error_receipt(
            "adaptive_runtime",
            "state_write_failed",
            Some(state_path.to_string_lossy().to_string()),
        ));
        return 1;
    }
    let _ = append_jsonl(&receipts_path, &receipt);
    print_json(&receipt);
    if admitted {
        0
    } else {
        1
    }
}

fn status(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let policy_path = parsed
        .flags
        .get("policy")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(POLICY_REL));
    let policy = match load_policy(&policy_path) {
        Ok(value) => value,
        Err(err) => {
            print_json(&error_receipt(
                "adaptive_runtime_status",
                "policy_load_failed",
                Some(err),
            ));
            return 1;
        }
    };

    let state_path = root.join(clean(&policy.state_path, 260));
    if !state_path.exists() {
        print_json(&json!({
            "ok": false,
            "type": "adaptive_runtime_status",
            "error": "state_missing",
            "policy_enabled": policy.enabled,
            "cadence_hours": policy.cadence_hours,
            "state_path": state_path.to_string_lossy().to_string()
        }));
        return 1;
    }

    match fs::read_to_string(&state_path) {
        Ok(raw) => {
            let mut payload = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| {
                json!({
                    "ok": false,
                    "type": "adaptive_runtime_status",
                    "error": "state_decode_failed",
                    "state_path": state_path.to_string_lossy().to_string()
                })
            });
            payload["type"] = Value::String("adaptive_runtime_status".to_string());
            payload["policy_enabled"] = Value::Bool(policy.enabled);
            payload["cadence_hours"] = Value::Number(policy.cadence_hours.into());
            print_json(&payload);
            if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_json(&error_receipt(
                "adaptive_runtime_status",
                "state_read_failed",
                Some(err.to_string()),
            ));
            1
        }
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("mkdir_failed:{err}"))?;
    }
    let tmp = path.with_extension("tmp");
    let encoded =
        serde_json::to_string_pretty(value).map_err(|err| format!("encode_failed:{err}"))?;
    fs::write(&tmp, format!("{encoded}\n")).map_err(|err| format!("tmp_write_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("rename_failed:{err}"))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("mkdir_failed:{err}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open_failed:{err}"))?;
    let line = serde_json::to_string(value).map_err(|err| format!("encode_failed:{err}"))?;
    use std::io::Write;
    file.write_all(format!("{line}\n").as_bytes())
        .map_err(|err| format!("append_failed:{err}"))
}

fn error_receipt(kind: &str, code: &str, detail: Option<String>) -> Value {
    let mut out = json!({
        "ok": false,
        "type": kind,
        "error": code,
        "detail": detail
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
    );
}
