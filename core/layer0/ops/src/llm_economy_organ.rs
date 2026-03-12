// SPDX-License-Identifier: Apache-2.0
use crate::{clean, deterministic_receipt_hash, now_iso, parse_args};
use serde_json::{json, Value};
use std::fs;
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

fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut body) = serde_json::to_string_pretty(value) {
        body.push('\n');
        let _ = fs::write(path, body);
    }
}

fn append_jsonl(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(value) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| {
                std::io::Write::write_all(&mut file, format!("{line}\n").as_bytes())
            });
    }
}

fn parse_bool(raw: Option<&String>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
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

fn normalize_target(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "virtuals" | "virtuals_acp" => "virtuals_acp".to_string(),
        "bankrbot" | "bankrbot_defi" => "bankrbot_defi".to_string(),
        "nookplot" | "nookplot_jobs" => "nookplot_jobs".to_string(),
        "owocki" | "owocki_jobs" => "owocki_jobs".to_string(),
        "heurist" | "heurist_marketplace" => "heurist_marketplace".to_string(),
        "daydreams" | "daydreams_marketplace" => "daydreams_marketplace".to_string(),
        "fairscale" | "fairscale_credit" => "fairscale_credit".to_string(),
        "trade_router" | "trade-router" | "trade_router_solana" => "trade_router_solana".to_string(),
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

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops llm-economy-organ run [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ enable <all|virtuals|bankrbot|nookplot|owocki|heurist|daydreams|fairscale|trade_router> [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ dashboard");
        println!("  protheus-ops llm-economy-organ status");
        return 0;
    }

    let latest = latest_path(root);
    let history = history_path(root);

    if command == "status" {
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_organ_status",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "latest": read_json(&latest)
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        print_receipt(&out);
        return 0;
    }

    if command == "dashboard" {
        let latest_payload = read_json(&latest);
        let enabled_map = current_enabled_map(latest_payload.as_ref());
        let enabled_count = enabled_map
            .values()
            .filter(|v| v.as_bool().unwrap_or(false))
            .count();
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_organ_dashboard",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "enabled_count": enabled_count,
            "total_hands": ECONOMY_HANDS.len(),
            "enabled_hands": enabled_map,
            "claim_evidence": [
                {
                    "id": "economy_dashboard_contract",
                    "claim": "economy_dashboard_reports_enabled_default_eyes_hands",
                    "evidence": {
                        "enabled_count": enabled_count,
                        "total_hands": ECONOMY_HANDS.len()
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        print_receipt(&out);
        return 0;
    }

    if command == "enable" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let target_raw = parsed
            .positional
            .get(1)
            .map(String::as_str)
            .unwrap_or("all");
        let target = normalize_target(target_raw);
        let mut enabled = current_enabled_map(read_json(&latest).as_ref());

        if target == "all" {
            for key in ECONOMY_HANDS {
                enabled.insert(key.to_string(), Value::Bool(true));
            }
        } else if ECONOMY_HANDS.contains(&target.as_str()) {
            enabled.insert(target.clone(), Value::Bool(true));
        } else {
            let mut out = json!({
                "ok": false,
                "type": "llm_economy_organ_enable_error",
                "lane": "core/layer0/ops",
                "ts": now_iso(),
                "error": "unknown_target",
                "target": target
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            print_receipt(&out);
            return 2;
        }

        let enabled_count = enabled
            .values()
            .filter(|v| v.as_bool().unwrap_or(false))
            .count();
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_organ_enable",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "target": target,
            "enabled_count": enabled_count,
            "total_hands": ECONOMY_HANDS.len(),
            "enabled_hands": enabled,
            "claim_evidence": [
                {
                    "id": "economy_enable_contract",
                    "claim": "agent_economy_default_eyes_hands_can_be_enabled_with_receipts",
                    "evidence": {
                        "target": target,
                        "enabled_count": enabled_count
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        write_json(&latest, &out);
        append_jsonl(&history, &out);
        print_receipt(&out);
        return 0;
    }

    let apply = parse_bool(parsed.flags.get("apply"), false);
    let mut out = json!({
        "ok": true,
        "type": "llm_economy_organ_run",
        "lane": "core/layer0/ops",
        "ts": now_iso(),
        "apply": apply,
        "model_routing": {
            "budget_band": if apply { "applied" } else { "dry_run" },
            "providers_ranked": [],
            "note": "core_authoritative_placeholder"
        },
        "receipts": {
            "strategy": clean(parsed.flags.get("strategy").cloned().unwrap_or_default(), 120),
            "capital": clean(parsed.flags.get("capital").cloned().unwrap_or_default(), 120)
        }
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    write_json(&latest, &out);
    append_jsonl(&history, &out);
    print_receipt(&out);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_target_maps_known_aliases() {
        assert_eq!(normalize_target("virtuals"), "virtuals_acp");
        assert_eq!(normalize_target("trade-router"), "trade_router_solana");
        assert_eq!(normalize_target(""), "all");
    }

    #[test]
    fn enable_all_writes_enabled_hands_to_latest_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exit = run(
            dir.path(),
            &[
                "enable".to_string(),
                "all".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(exit, 0);
        let latest = read_json(&latest_path(dir.path())).expect("latest");
        assert_eq!(
            latest.get("type").and_then(Value::as_str),
            Some("llm_economy_organ_enable")
        );
        assert_eq!(
            latest.get("enabled_count").and_then(Value::as_u64),
            Some(ECONOMY_HANDS.len() as u64)
        );
    }
}
