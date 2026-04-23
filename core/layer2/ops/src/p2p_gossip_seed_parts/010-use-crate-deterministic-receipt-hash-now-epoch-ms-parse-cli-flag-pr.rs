// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_epoch_ms, parse_cli_flag, print_json_line};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const USAGE: &[&str] = &[
    "Usage:",
    "  infring-ops p2p-gossip-seed status|dashboard",
    "  infring-ops p2p-gossip-seed discover|join [--profile=hyperspace] [--node=<id>] [--apply=1|0]",
    "  infring-ops p2p-gossip-seed compute-proof [--share=1|0] [--matmul-size=<n>] [--credits=<n>]",
    "  infring-ops p2p-gossip-seed gossip [--topic=<topic>] [--breakthrough=<text>]",
    "  infring-ops p2p-gossip-seed idle-rss [--feed=<id>] [--note=<text>]",
    "  infring-ops p2p-gossip-seed ranking-evolve [--metric=ndcg@10] [--delta=<0..1>]",
];

fn parse_bool(raw: Option<String>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

fn parse_f64(raw: Option<String>, fallback: f64) -> f64 {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

fn parse_u64(raw: Option<String>, fallback: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}

fn command_claim_ids(command: &str) -> &'static [&'static str] {
    match command {
        "discover" | "join" => &["V6-NETWORK-004.6", "V6-NETWORK-004.2"],
        "compute-proof" => &["V6-NETWORK-004.1", "V6-NETWORK-004.2", "V6-NETWORK-004.6"],
        "gossip" => &["V6-NETWORK-004.3"],
        "idle-rss" => &["V6-NETWORK-004.4"],
        "ranking-evolve" => &["V6-NETWORK-004.5"],
        "status" | "dashboard" => &["V6-NETWORK-004.2", "V6-NETWORK-004.6"],
        _ => &[],
    }
}

fn conduit_enforcement(argv: &[String], command: &str, strict: bool) -> Value {
    let bypass_requested = parse_bool(parse_cli_flag(argv, "bypass"), false)
        || parse_bool(parse_cli_flag(argv, "client-bypass"), false);
    let ok = !bypass_requested;
    let claim_rows = command_claim_ids(command)
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "claim": "network_commands_route_through_core_runtime_with_fail_closed_bypass_denial",
                "evidence": {
                    "command": command,
                    "bypass_requested": bypass_requested
                }
            })
        })
        .collect::<Vec<_>>();
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "type": "p2p_gossip_seed_conduit_enforcement",
        "command": command,
        "strict": strict,
        "bypass_requested": bypass_requested,
        "errors": if ok { Value::Array(Vec::new()) } else { json!(["conduit_bypass_rejected"]) },
        "claim_evidence": claim_rows
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn state_dir(root: &Path) -> PathBuf {
    root.join("local")
        .join("state")
        .join("ops")
        .join("p2p_gossip_seed")
}

fn latest_path(root: &Path) -> PathBuf {
    state_dir(root).join("latest.json")
}

fn history_path(root: &Path) -> PathBuf {
    state_dir(root).join("history.jsonl")
}

fn reputation_path(root: &Path) -> PathBuf {
    state_dir(root).join("reputation_ledger.json")
}

fn contribution_path(root: &Path) -> PathBuf {
    state_dir(root).join("contribution_ledger.json")
}

fn gossip_log_path(root: &Path) -> PathBuf {
    state_dir(root).join("gossip_log.jsonl")
}

fn idle_feed_log_path(root: &Path) -> PathBuf {
    state_dir(root).join("idle_feed_log.jsonl")
}

fn ranking_state_path(root: &Path) -> PathBuf {
    state_dir(root).join("ranking_metrics.json")
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

fn persist_receipt(root: &Path, value: &Value) {
    write_json(&latest_path(root), value);
    append_jsonl(&history_path(root), value);
}

fn reputations(root: &Path) -> serde_json::Map<String, Value> {
    read_json(&reputation_path(root))
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn write_reputations(root: &Path, map: &serde_json::Map<String, Value>) {
    write_json(&reputation_path(root), &Value::Object(map.clone()));
}

fn contributions(root: &Path) -> serde_json::Map<String, Value> {
    read_json(&contribution_path(root))
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn write_contributions(root: &Path, map: &serde_json::Map<String, Value>) {
    write_json(&contribution_path(root), &Value::Object(map.clone()));
}

fn count_jsonl_rows(path: &Path) -> u64 {
    fs::read_to_string(path)
        .ok()
        .map(|raw| raw.lines().filter(|row| !row.trim().is_empty()).count() as u64)
        .unwrap_or(0)
}

fn usage() {
    for line in USAGE {
        println!("{line}");
    }
}

fn dashboard_receipt(root: &Path) -> Value {
    let latest = read_json(&latest_path(root));
    let rep = reputations(root);
    let contribution_ledger = contributions(root);
    let contribution_nodes = contribution_ledger.len();
    let ranking_state = read_json(&ranking_state_path(root)).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "latest": Value::Null,
            "history": []
        })
    });
    let gossip_events = count_jsonl_rows(&gossip_log_path(root));
    let idle_events = count_jsonl_rows(&idle_feed_log_path(root));
    let nodes = rep.len() as u64;
    let total_rep: f64 = rep.values().filter_map(Value::as_f64).sum();
    let mut out = json!({
        "ok": true,
        "type": "p2p_gossip_seed_dashboard",
        "lane": "core/layer2/ops",
        "ts_epoch_ms": now_epoch_ms(),
        "node_count": nodes,
        "reputation_total": total_rep,
        "reputation_ledger": rep,
        "contribution_ledger": contribution_ledger,
        "ranking_state": ranking_state,
        "event_totals": {
            "gossip_events": gossip_events,
            "idle_feed_events": idle_events
        },
        "latest_event": latest,
        "claim_evidence": [
            {
                "id": "V6-NETWORK-004.2",
                "claim": "reputation_ledger_updates_are_persisted_and_visible_in_dashboard_receipts",
                "evidence": {
                    "node_count": nodes,
                    "reputation_total": total_rep,
                    "contribution_nodes": contribution_nodes
                }
            },
            {
                "id": "V6-NETWORK-004.6",
                "claim": "network_join_compute_share_and_dashboard_are_exposed_as_core_receipted_surfaces",
                "evidence": {
                    "surface": "network dashboard",
                    "node_count": nodes,
                    "gossip_events": gossip_events
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}
