// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_iso};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn receipt_hash(v: &Value) -> String {
    deterministic_receipt_hash(v)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn finalize_model_router_receipt(out: &mut Value) {
    let Some(map) = out.as_object_mut() else {
        return;
    };
    if !map.contains_key("lane") {
        map.insert(
            "lane".to_string(),
            Value::String("core/layer0/ops".to_string()),
        );
    }
    if !map.contains_key("strict") {
        map.insert("strict".to_string(), Value::Bool(true));
    }
}

fn flag_value(argv: &[String], key: &str) -> Option<String> {
    let pref = format!("--{key}=");
    let mut i = 0usize;
    while i < argv.len() {
        let tok = argv[i].trim();
        if let Some(v) = tok.strip_prefix(&pref) {
            return Some(v.to_string());
        }
        if tok == format!("--{key}") {
            if let Some(next) = argv.get(i + 1) {
                if !next.starts_with("--") {
                    return Some(next.clone());
                }
            }
        }
        i += 1;
    }
    None
}

fn parse_bool_flag(raw: Option<String>, fallback: bool) -> bool {
    let Some(value) = raw else {
        return fallback;
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn command_claim_ids(command: &str) -> &'static [&'static str] {
    match command {
        "optimize" | "optimize-cheap" | "optimize-minimax" => &["V6-MODEL-003.5"],
        "reset-agent" | "agent-reset" => &["V6-MODEL-003.4"],
        "night-schedule" | "schedule-night" => &["V6-MODEL-003.6"],
        "adapt-repo" | "repo-adapt" => &["V6-MODEL-003.3"],
        "compact-context" | "compact" => &["V6-MODEL-003.1"],
        "decompose-task" | "decompose" => &["V6-MODEL-003.2"],
        "bitnet-backend" | "backend-bitnet" => &["V6-MODEL-004.1", "V6-MODEL-004.5"],
        "bitnet-auto-route" | "auto-route-bitnet" => &["V6-MODEL-004.2", "V6-MODEL-004.5"],
        "bitnet-use" | "use-bitnet" | "convert-bitnet" => &["V6-MODEL-004.3", "V6-MODEL-004.5"],
        "bitnet-telemetry" | "telemetry-bitnet" => &["V6-MODEL-004.4", "V6-MODEL-004.5"],
        "bitnet-attest" | "attest-bitnet" => &["V6-MODEL-004.5"],
        _ => &[],
    }
}

fn model_router_conduit_enforcement(args: &[String], command: &str, strict: bool) -> Value {
    let bypass_requested = parse_bool_flag(flag_value(args, "bypass"), false)
        || parse_bool_flag(flag_value(args, "client-bypass"), false);
    let ok = !bypass_requested;
    let claim_rows = command_claim_ids(command)
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "claim": "model_router_commands_route_through_core_authority_with_fail_closed_bypass_denial",
                "evidence": {
                    "command": command,
                    "bypass_requested": bypass_requested
                }
            })
        })
        .collect::<Vec<_>>();
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "type": "model_router_conduit_enforcement",
        "command": command,
        "strict": strict,
        "bypass_requested": bypass_requested,
        "errors": if ok { Value::Array(Vec::new()) } else { json!(["conduit_bypass_rejected"]) },
        "claim_evidence": claim_rows
    });
    finalize_model_router_receipt(&mut out);
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn select_route_model(
    provider_online: bool,
    preferred_model: &str,
    fallback_model: &str,
) -> (String, bool) {
    if provider_online {
        (preferred_model.to_string(), false)
    } else {
        (fallback_model.to_string(), true)
    }
}

fn model_router_state_paths(root: &Path) -> (PathBuf, PathBuf) {
    let dir = root.join("local/state/ops/model_router");
    (dir.join("latest.json"), dir.join("history.jsonl"))
}

fn provider_profile_path(root: &Path) -> PathBuf {
    root.join("local/state/ops/model_router/provider_profile.json")
}

fn reset_state_path(root: &Path) -> PathBuf {
    root.join("local/state/ops/model_router/reset_state.json")
}

fn night_schedule_path(root: &Path) -> PathBuf {
    root.join("local/state/ops/model_router/night_schedule.json")
}

fn bitnet_backend_path(root: &Path) -> PathBuf {
    root.join("local/state/ops/model_router/bitnet_backend.json")
}

fn bitnet_auto_route_path(root: &Path) -> PathBuf {
    root.join("local/state/ops/model_router/bitnet_auto_route.json")
}

fn bitnet_conversion_path(root: &Path) -> PathBuf {
    root.join("local/state/ops/model_router/bitnet_conversion.json")
}

fn bitnet_telemetry_path(root: &Path) -> PathBuf {
    root.join("local/state/ops/model_router/bitnet_telemetry.json")
}

fn bitnet_attestation_path(root: &Path) -> PathBuf {
    root.join("local/state/ops/model_router/bitnet_attestation.json")
}

fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

fn write_json(path: &Path, value: &Value) {
    ensure_parent(path);
    if let Ok(mut body) = serde_json::to_string_pretty(value) {
        body.push('\n');
        let _ = fs::write(path, body);
    }
}

fn append_jsonl(path: &Path, value: &Value) {
    ensure_parent(path);
    if let Ok(line) = serde_json::to_string(value) {
        use std::io::Write;
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| file.write_all(format!("{line}\n").as_bytes()));
    }
}

fn f64_flag(args: &[String], key: &str, fallback: f64, lo: f64, hi: f64) -> f64 {
    flag_value(args, key)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn i64_flag(args: &[String], key: &str, fallback: i64, lo: i64, hi: i64) -> i64 {
    flag_value(args, key)
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn non_flag_positional(args: &[String], skip: usize) -> Option<String> {
    args.iter()
        .skip(skip)
        .find(|row| !row.starts_with("--"))
        .cloned()
}

fn optimize_cheapest_receipt(root: &Path, args: &[String]) -> Value {
    let apply = parse_bool_flag(flag_value(args, "apply"), true);
    let profile = flag_value(args, "profile")
        .or_else(|| non_flag_positional(args, 1))
        .unwrap_or_else(|| "minimax".to_string());
    let compact_lines = i64_flag(args, "compact-lines", 24, 8, 128);
    let target_cost_per_million = f64_flag(args, "target-cost", 0.30, 0.01, 500.0);
    let baseline_cost_per_million = f64_flag(args, "baseline-cost", 5.0, 0.01, 5000.0);
    let quality_target_pct = f64_flag(args, "quality-target-pct", 95.0, 10.0, 100.0);
    let preferred_model = flag_value(args, "model").unwrap_or_else(|| "minimax/m2.5".to_string());
    let provider_url = flag_value(args, "provider-url")
        .unwrap_or_else(|| "https://api.minimax.chat/v1".to_string());
    let key_env = flag_value(args, "key-env").unwrap_or_else(|| "MINIMAX_API_KEY".to_string());
    let savings_pct =
        ((baseline_cost_per_million - target_cost_per_million) / baseline_cost_per_million) * 100.0;
    let profile_path = provider_profile_path(root);
    let profile_digest = receipt_hash(&json!({
        "profile": profile,
        "preferred_model": preferred_model,
        "provider_url": provider_url,
        "target_cost_per_million": target_cost_per_million,
        "quality_target_pct": quality_target_pct
    }));
    let profile_state = json!({
        "version": "v1",
        "updated_at": now_iso(),
        "profile": profile,
        "preferred_model": preferred_model,
        "provider_url": provider_url,
        "target_cost_per_million": target_cost_per_million,
        "baseline_cost_per_million": baseline_cost_per_million,
        "quality_target_pct": quality_target_pct,
        "profile_digest": profile_digest
    });
    if apply {
        write_json(&profile_path, &profile_state);
    }

    let mut out = json!({
        "ok": true,
        "type": "model_router_optimize_cheap",
        "ts": now_iso(),
        "profile": profile,
        "apply": apply,
        "plan": {
            "memory_compaction_lines": compact_lines,
            "hierarchical_subtasks": true,
            "provider_swap_enabled": true,
            "preferred_model": preferred_model,
            "provider_url": provider_url,
            "key_env": key_env,
            "target_cost_per_million": target_cost_per_million,
            "baseline_cost_per_million": baseline_cost_per_million,
            "quality_target_pct": quality_target_pct,
            "estimated_savings_pct": savings_pct,
            "fallback_chain": ["minimax/m2.5", "kimi-k2.5:cloud", "llama-4-maverick:cloud"],
            "profile_digest": profile_digest
        },
        "profile_state_path": profile_path.display().to_string(),
        "claim_evidence": [
            {
                "id": "V6-MODEL-003.5",
                "claim": "dynamic_provider_abstraction_routes_cheap_model_profiles_with_deterministic_receipts",
                "evidence": {
                    "profile": profile,
                    "preferred_model": preferred_model,
                    "provider_url": provider_url,
                    "memory_compaction_lines": compact_lines,
                    "estimated_savings_pct": savings_pct,
                    "profile_digest": profile_digest
                }
            }
        ]
    });
    finalize_model_router_receipt(&mut out);
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    let (latest_path, history_path) = model_router_state_paths(root);
    write_json(&latest_path, &out);
    append_jsonl(&history_path, &out);
    out
}

fn reset_agent_receipt(root: &Path, args: &[String]) -> Value {
    let preserve_identity = parse_bool_flag(flag_value(args, "preserve-identity"), true);
    let scope = flag_value(args, "scope").unwrap_or_else(|| "routing+session-cache".to_string());
    let dry_run = parse_bool_flag(flag_value(args, "dry-run"), false);
    let (latest_path, history_path) = model_router_state_paths(root);
    let latest = read_json(&latest_path);
    let preserved_keys = if preserve_identity {
        vec![
            "identity".to_string(),
            "profile".to_string(),
            "night_schedule".to_string(),
        ]
    } else {
        vec!["profile".to_string(), "night_schedule".to_string()]
    };
    let checkpoint = json!({
        "version": "v1",
        "ts": now_iso(),
        "scope": scope,
        "preserve_identity": preserve_identity,
        "dry_run": dry_run,
        "previous_receipt_hash": latest
            .as_ref()
            .and_then(|v| v.get("receipt_hash"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        "preserved_keys": preserved_keys
    });
    if !dry_run {
        write_json(&reset_state_path(root), &checkpoint);
    }
    let mut out = json!({
        "ok": true,
        "type": "model_router_agent_reset",
        "ts": now_iso(),
        "scope": scope,
        "preserve_identity": preserve_identity,
        "dry_run": dry_run,
        "state_preservation": checkpoint,
        "reset_state_path": reset_state_path(root).display().to_string(),
        "claim_evidence": [
            {
                "id": "V6-MODEL-003.4",
                "claim": "agent_reset_routes_to_core_lane_with_deterministic_identity_preserving_receipts",
                "evidence": {
                    "preserve_identity": preserve_identity,
                    "scope": scope
                }
            }
        ]
    });
    finalize_model_router_receipt(&mut out);
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    write_json(&latest_path, &out);
    append_jsonl(&history_path, &out);
    out
}

fn night_scheduler_receipt(root: &Path, args: &[String]) -> Value {
    let start_hour = i64_flag(args, "start-hour", 0, 0, 23);
    let end_hour = i64_flag(args, "end-hour", 6, 0, 23);
    let timezone = flag_value(args, "timezone").unwrap_or_else(|| "America/Denver".to_string());
    let cheap_model = flag_value(args, "cheap-model").unwrap_or_else(|| "minimax/m2.5".to_string());
    let heavy_threshold = flag_value(args, "heavy-threshold")
        .unwrap_or_else(|| "complexity:high_or_risk:high".to_string());
    let max_hourly_budget_usd = f64_flag(args, "max-hourly-budget-usd", 0.25, 0.01, 500.0);
    let daytime_preferred_model =
        flag_value(args, "daytime-model").unwrap_or_else(|| "ollama/llama3.2:latest".to_string());
    let window_hours = if end_hour >= start_hour {
        end_hour - start_hour
    } else {
        (24 - start_hour) + end_hour
    };
    let schedule_digest = receipt_hash(&json!({
        "start_hour": start_hour,
        "end_hour": end_hour,
        "timezone": timezone,
        "cheap_model": cheap_model,
        "daytime_model": daytime_preferred_model,
        "max_hourly_budget_usd": max_hourly_budget_usd
    }));
    let schedule_state = json!({
        "version": "v1",
        "updated_at": now_iso(),
        "start_hour": start_hour,
        "end_hour": end_hour,
        "window_hours": window_hours,
        "timezone": timezone,
        "cheap_model": cheap_model,
        "daytime_model": daytime_preferred_model,
        "heavy_threshold": heavy_threshold,
        "max_hourly_budget_usd": max_hourly_budget_usd,
        "schedule_digest": schedule_digest
    });
    write_json(&night_schedule_path(root), &schedule_state);
    let mut out = json!({
        "ok": true,
        "type": "model_router_night_schedule",
        "ts": now_iso(),
        "schedule": {
            "start_hour": start_hour,
            "end_hour": end_hour,
            "window_hours": window_hours,
            "timezone": timezone,
            "cheap_model": cheap_model,
            "daytime_model": daytime_preferred_model,
            "heavy_threshold": heavy_threshold,
            "max_hourly_budget_usd": max_hourly_budget_usd,
            "schedule_digest": schedule_digest
        },
        "night_schedule_path": night_schedule_path(root).display().to_string(),
        "cost_triggers": [
            {
                "condition": "within_night_window",
                "action": "route_to_cheap_model"
            },
            {
                "condition": "estimated_cost_exceeds_hourly_budget",
                "action": "decompose_and_defer_non_urgent_tasks"
            }
        ],
        "claim_evidence": [
            {
                "id": "V6-MODEL-003.6",
                "claim": "cost_aware_night_scheduler_emits_deterministic_windowed_routing_receipts",
                "evidence": {
                    "start_hour": start_hour,
                    "end_hour": end_hour,
                    "cheap_model": cheap_model,
                    "max_hourly_budget_usd": max_hourly_budget_usd
                }
            }
        ]
    });
    finalize_model_router_receipt(&mut out);
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    let (latest_path, history_path) = model_router_state_paths(root);
    write_json(&latest_path, &out);
    append_jsonl(&history_path, &out);
    out
}

