// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::{clean, deterministic_receipt_hash, now_iso, parse_args};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn state_root(root: &Path) -> PathBuf {
    if let Ok(v) = std::env::var("LLM_ECONOMY_ORGAN_STATE_ROOT") {
        let s = v.trim();
        if !s.is_empty() {
            return PathBuf::from(s);
        }
    }
    root.join("client")
        .join("local")
        .join("state")
        .join("ops")
        .join("llm_economy_organ")
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn history_path(root: &Path) -> PathBuf {
    state_root(root).join("history.jsonl")
}

fn trust_ledger_path(root: &Path) -> PathBuf {
    state_root(root).join("trust_ledger.json")
}

fn mining_runtime_path(root: &Path) -> PathBuf {
    state_root(root).join("mining_runtime.json")
}

fn trade_intents_path(root: &Path) -> PathBuf {
    state_root(root).join("trade_intents.jsonl")
}

fn trading_profile_path(root: &Path) -> PathBuf {
    state_root(root).join("trading_profile.json")
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn write_json(path: &Path, value: &Value) {
    let _ = lane_utils::write_json(path, value);
}

fn append_jsonl(path: &Path, value: &Value) {
    let _ = lane_utils::append_jsonl(path, value);
}

fn parse_bool(raw: Option<&String>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw.map(String::as_str), fallback)
}

fn parse_f64(raw: Option<&String>, fallback: f64) -> f64 {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

fn parse_u64(raw: Option<&String>, fallback: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}

fn parse_f64_opt(raw: Option<&String>, fallback: f64) -> f64 {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

fn parse_cron_interval_minutes(schedule: &str) -> Option<u64> {
    let first = schedule.split_whitespace().next()?.trim();
    let interval = first.strip_prefix("*/")?;
    interval.parse::<u64>().ok().filter(|v| *v > 0)
}

fn canonical_chain(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "sol" => "solana".to_string(),
        "solana" => "solana".to_string(),
        other => other.to_string(),
    }
}

fn print_receipt(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

const ECONOMY_HANDS: [&str; 8] = [
    "virtuals_acp",
    "bankrbot_defi",
    "nookplot_jobs",
    "owocki_jobs",
    "heurist_marketplace",
    "daydreams_marketplace",
    "fairscale_credit",
    "trade_router_solana",
];
const ECONOMY_CONTRACT_PATH: &str = "planes/contracts/economy/economy_hands_contract_v1.json";

fn load_contract(root: &Path) -> Value {
    read_json(&root.join(ECONOMY_CONTRACT_PATH)).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "kind": "economy_hands_contract",
            "hands": ["virtuals-acp", "bankrbot-defi", "jobs-marketplace", "skills-marketplace"]
        })
    })
}

fn normalize_target(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "virtuals" | "virtuals_acp" => "virtuals_acp".to_string(),
        "bankrbot" | "bankrbot_defi" => "bankrbot_defi".to_string(),
        "nookplot" | "nookplot_jobs" => "nookplot_jobs".to_string(),
        "owocki" | "owocki_jobs" => "owocki_jobs".to_string(),
        "heurist" | "heurist_marketplace" => "heurist_marketplace".to_string(),
        "daydreams" | "daydreams_marketplace" => "daydreams_marketplace".to_string(),
        "fairscale" | "fairscale_credit" => "fairscale_credit".to_string(),
        "trade_router" | "trade-router" | "trade_router_solana" => {
            "trade_router_solana".to_string()
        }
        "all" | "" => "all".to_string(),
        other => other.to_string(),
    }
}

fn current_enabled_map(latest: Option<&Value>) -> serde_json::Map<String, Value> {
    latest
        .and_then(|v| v.get("enabled_hands"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn claim_ids_for_command(command: &str) -> Vec<&'static str> {
    match command {
        "enable" => vec!["V6-ECONOMY-001.8"],
        "virtuals-acp" => vec!["V6-ECONOMY-001.1"],
        "bankrbot-defi" => vec!["V6-ECONOMY-001.2"],
        "jobs-marketplace" => vec!["V6-ECONOMY-001.3"],
        "skills-marketplace" => vec!["V6-ECONOMY-001.4"],
        "fairscale-credit" => vec!["V6-ECONOMY-001.5"],
        "mining-hand" => vec!["V6-ECONOMY-001.6"],
        "trade-router" => vec!["V6-ECONOMY-001.7"],
        "upgrade-trading-hand" => vec!["V6-ECONOMY-002.1", "V6-ECONOMY-002.4"],
        "debate-bullbear" | "agent-debate-bullbear" => vec!["V6-ECONOMY-002.2"],
        "alpaca-execute" | "trading-execute" => vec!["V6-ECONOMY-002.3"],
        "model-support-refresh" => vec!["V6-ECONOMY-002.5"],
        _ => vec!["economy_core_authority"],
    }
}

fn conduit_enforcement(parsed: &crate::ParsedArgs, strict: bool, command: &str) -> Value {
    let bypass_requested = parse_bool(parsed.flags.get("bypass"), false)
        || parse_bool(parsed.flags.get("direct"), false)
        || parse_bool(parsed.flags.get("unsafe-client-route"), false)
        || parse_bool(parsed.flags.get("client-bypass"), false);
    let ok = !bypass_requested;
    let claim_rows = claim_ids_for_command(command)
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "claim": "economy_hands_route_through_layer0_conduit_with_fail_closed_denials",
                "evidence": {
                    "command": clean(command, 80),
                    "bypass_requested": bypass_requested
                }
            })
        })
        .collect::<Vec<_>>();
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "type": "llm_economy_conduit_enforcement",
        "required_path": "core/layer0/ops/llm_economy_organ",
        "command": clean(command, 80),
        "bypass_requested": bypass_requested,
        "errors": if ok { Value::Array(Vec::new()) } else { json!(["conduit_bypass_rejected"]) },
        "claim_evidence": claim_rows
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}
