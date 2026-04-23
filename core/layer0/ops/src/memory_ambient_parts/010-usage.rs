// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_iso};
use base64::Engine;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
struct MemoryAmbientPolicy {
    enabled: bool,
    rust_authoritative: bool,
    push_attention_queue: bool,
    quiet_non_critical: bool,
    surface_levels: Vec<String>,
    latest_path: PathBuf,
    receipts_path: PathBuf,
    status_path: PathBuf,
    history_path: PathBuf,
    policy_path: PathBuf,
}

fn usage() {
    eprintln!("Usage:");
    eprintln!(
        "  infring-ops memory-ambient run <memory-command> [memory-args...] [--run-context=<value>]"
    );
    eprintln!(
        "  infring-ops memory-ambient run --memory-command=<cmd> [--memory-arg=<arg> ...] [--memory-args-json=<json-array>] [--run-context=<value>]"
    );
    eprintln!("  infring-ops memory-ambient status");
}

fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut raw) = serde_json::to_string_pretty(value) {
        raw.push('\n');
        let _ = fs::write(path, raw);
    }
}

fn append_jsonl(path: &Path, row: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(row) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| {
                std::io::Write::write_all(&mut file, format!("{line}\n").as_bytes())
            });
    }
}

fn parse_cli_flags(argv: &[String]) -> BTreeMap<String, String> {
    crate::contract_lane_utils::parse_cli_flags(argv)
}

fn parse_string_array(value: Option<&Value>, max_items: usize, max_len: usize) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(Some(row), max_len))
                .filter(|row| !row.is_empty())
                .take(max_items)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn collect_flag_values(argv: &[String], key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let flag = format!("--{key}");
    let prefix = format!("--{key}=");
    while i < argv.len() {
        let token = argv[i].trim();
        if token == flag {
            if let Some(next) = argv.get(i + 1) {
                if !next.starts_with("--") {
                    out.push(next.clone());
                    i += 2;
                    continue;
                }
            }
            out.push(String::new());
            i += 1;
            continue;
        }
        if let Some(value) = token.strip_prefix(&prefix) {
            out.push(value.to_string());
        }
        i += 1;
    }
    out
}

fn bool_from_env(name: &str) -> Option<bool> {
    let raw = std::env::var(name).ok()?;
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn clean_text(value: Option<&str>, max_len: usize) -> String {
    let mut out = String::new();
    if let Some(raw) = value {
        for ch in raw.split_whitespace().collect::<Vec<_>>().join(" ").chars() {
            if out.len() >= max_len {
                break;
            }
            out.push(ch);
        }
    }
    out.trim().to_string()
}

fn estimate_tokens(value: &Value) -> i64 {
    let rendered = serde_json::to_string(value).unwrap_or_default();
    ((rendered.chars().count() + 3) / 4) as i64
}

fn parse_arg_value(memory_args: &[String], key: &str) -> Option<String> {
    let exact = format!("--{key}");
    let pref = format!("--{key}=");
    let mut i = 0usize;
    while i < memory_args.len() {
        let token = memory_args[i].as_str();
        if token == exact {
            if let Some(next) = memory_args.get(i + 1) {
                if !next.starts_with("--") {
                    return Some(next.clone());
                }
            }
            return Some(String::new());
        }
        if let Some(value) = token.strip_prefix(&pref) {
            return Some(value.to_string());
        }
        i += 1;
    }
    None
}

fn parse_bool_value(raw: Option<&str>, fallback: bool) -> bool {
    let Some(value) = raw else {
        return fallback;
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn command_label(memory_command: &str) -> String {
    match memory_command {
        "query-index" => "query".to_string(),
        "get-node" => "get".to_string(),
        other => other.to_string(),
    }
}

fn is_nano_memory_command(memory_command: &str) -> bool {
    matches!(
        memory_command,
        "stable-nano-chat" | "stable-nano-train" | "stable-nano-fork"
    )
}

fn memory_batch22_command_claim_ids(memory_command: &str) -> &'static [&'static str] {
    match memory_command {
        "memory-taxonomy" | "stable-memory-taxonomy" => &["V6-MEMORY-011.1", "V6-MEMORY-011.5"],
        "memory-enable-metacognitive" | "stable-memory-enable-metacognitive" => {
            &["V6-MEMORY-011.2"]
        }
        "memory-share" | "stable-memory-share" => &["V6-MEMORY-011.3"],
        "memory-evolve" | "stable-memory-evolve" => &["V6-MEMORY-011.4"],
        "memory-enable-causality" | "stable-memory-enable-causality" => {
            &["V6-MEMORY-012.1", "V6-MEMORY-012.5"]
        }
        "memory-causal-retrieve" | "stable-memory-causal-retrieve" => &["V6-MEMORY-012.2"],
        "memory-benchmark-ama" | "stable-memory-benchmark-ama" => {
            &["V6-MEMORY-012.3", "V6-MEMORY-012.5"]
        }
        "memory-fuse" | "stable-memory-fuse" => &["V6-MEMORY-012.4"],
        _ => &[],
    }
}

fn is_batch22_memory_command(memory_command: &str) -> bool {
    !memory_batch22_command_claim_ids(memory_command).is_empty()
}

fn ensure_digest_field(memory_payload: &mut Value, field: &str, digest_input: Value) {
    let missing = memory_payload
        .get(field)
        .and_then(Value::as_str)
        .map(|value| value.trim().is_empty())
        .unwrap_or(true);
    if !missing {
        return;
    }
    if let Some(map) = memory_payload.as_object_mut() {
        map.insert(
            field.to_string(),
            Value::String(crate::deterministic_receipt_hash(&digest_input)),
        );
    }
}

fn ensure_memory_contract_digests(
    memory_command: &str,
    memory_args: &[String],
    memory_payload: &mut Value,
) {
    match memory_command {
        "memory-enable-metacognitive" | "stable-memory-enable-metacognitive" => {
            ensure_digest_field(
                memory_payload,
                "config_digest",
                json!({
                    "type": memory_payload.get("type").and_then(Value::as_str).unwrap_or("memory_metacognitive_enable"),
                    "config_path": memory_payload.get("config_path").and_then(Value::as_str).unwrap_or(""),
                    "enabled": memory_payload.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                    "note": parse_arg_value(memory_args, "note").unwrap_or_default()
                }),
            );
        }
        "memory-taxonomy" | "stable-memory-taxonomy" => {
            ensure_digest_field(
                memory_payload,
                "taxonomy_digest",
                json!({
                    "type": memory_payload.get("type").and_then(Value::as_str).unwrap_or("memory_taxonomy_4w"),
                    "row_count": memory_payload.get("row_count").and_then(Value::as_u64).unwrap_or(0),
                    "what_counts": memory_payload.get("what_counts").cloned().unwrap_or_else(|| json!({})),
                    "who_counts": memory_payload.get("who_counts").cloned().unwrap_or_else(|| json!({})),
                    "when_missing": memory_payload.get("when_missing").and_then(Value::as_u64).unwrap_or(0),
                    "where_missing": memory_payload.get("where_missing").and_then(Value::as_u64).unwrap_or(0)
                }),
            );
        }
        "memory-share" | "stable-memory-share" => {
            ensure_digest_field(
                memory_payload,
                "consent_scope_digest",
                json!({
                    "type": memory_payload.get("type").and_then(Value::as_str).unwrap_or("memory_share"),
                    "persona": memory_payload.get("persona").and_then(Value::as_str).unwrap_or(""),
                    "scope": memory_payload.get("scope").and_then(Value::as_str).unwrap_or(""),
                    "consent": memory_payload.get("consent").and_then(Value::as_bool).unwrap_or(false)
                }),
            );
        }
        "memory-evolve" | "stable-memory-evolve" => {
            ensure_digest_field(
                memory_payload,
                "evolution_digest",
                json!({
                    "type": memory_payload.get("type").and_then(Value::as_str).unwrap_or("memory_evolve"),
                    "generation": memory_payload.get("generation").and_then(Value::as_u64).unwrap_or(0),
                    "stability_score": memory_payload.get("stability_score").cloned().unwrap_or(Value::Null),
                    "state_path": memory_payload.get("evolution_state_path").and_then(Value::as_str).unwrap_or("")
                }),
            );
        }
        _ => {}
    }
}

fn memory_batch22_claim_evidence(
    memory_command: &str,
    memory_args: &[String],
    memory_payload: &Value,
) -> Vec<Value> {
    match memory_command {
        "memory-taxonomy" | "stable-memory-taxonomy" => vec![
            json!({
                "id": "V6-MEMORY-011.1",
                "claim": "memory_taxonomy_classifies_entries_into_4w_tags_with_deterministic_receipts",
                "evidence": {
                    "memory_command": memory_command,
                    "row_count": memory_payload.get("row_count").and_then(Value::as_u64).unwrap_or(0),
                    "taxonomy_path": memory_payload.get("taxonomy_path").and_then(Value::as_str).unwrap_or(""),
                    "taxonomy_digest": memory_payload.get("taxonomy_digest").and_then(Value::as_str).unwrap_or("")
                }
            }),
            json!({
                "id": "V6-MEMORY-011.5",
                "claim": "taxonomy_commands_emit_dashboard_ready_health_metrics_with_deterministic_receipts",
                "evidence": {
                    "memory_command": memory_command,
                    "when_missing": memory_payload.get("when_missing").and_then(Value::as_u64).unwrap_or(0),
                    "what_bucket_count": memory_payload.get("what_counts").and_then(Value::as_object).map(|m| m.len()).unwrap_or(0),
                    "taxonomy_digest": memory_payload.get("taxonomy_digest").and_then(Value::as_str).unwrap_or("")
                }
            }),
        ],
        "memory-enable-metacognitive" | "stable-memory-enable-metacognitive" => vec![json!({
            "id": "V6-MEMORY-011.2",
            "claim": "metacognitive_enable_persists_config_and_journal_with_deterministic_receipts",
            "evidence": {
                "memory_command": memory_command,
                "enabled": memory_payload.get("enabled").and_then(Value::as_bool).unwrap_or(false),
                "config_path": memory_payload.get("config_path").and_then(Value::as_str).unwrap_or(""),
                "config_digest": memory_payload.get("config_digest").and_then(Value::as_str).unwrap_or("")
            }
        })],
        "memory-share" | "stable-memory-share" => vec![json!({
            "id": "V6-MEMORY-011.3",
            "claim": "memory_share_enforces_consent_scoped_multi_agent_sharing_with_deterministic_receipts",
            "evidence": {
                "memory_command": memory_command,
                "persona": memory_payload.get("persona").and_then(Value::as_str).unwrap_or(""),
                "scope": memory_payload.get("scope").and_then(Value::as_str).unwrap_or(""),
                "consent": memory_payload.get("consent").and_then(Value::as_bool).unwrap_or(false),
                "consent_scope_digest": memory_payload.get("consent_scope_digest").and_then(Value::as_str).unwrap_or("")
            }
        })],
        "memory-evolve" | "stable-memory-evolve" => vec![json!({
            "id": "V6-MEMORY-011.4",
            "claim": "memory_evolve_writes_longitudinal_snapshots_with_generation_and_stability_receipts",
            "evidence": {
                "memory_command": memory_command,
                "generation": memory_payload.get("generation").and_then(Value::as_u64).unwrap_or(0),
                "stability_score": memory_payload.get("stability_score").cloned().unwrap_or(Value::Null),
                "evolution_state_path": memory_payload.get("evolution_state_path").and_then(Value::as_str).unwrap_or(""),
                "evolution_digest": memory_payload.get("evolution_digest").and_then(Value::as_str).unwrap_or("")
            }
        })],
        "memory-enable-causality" | "stable-memory-enable-causality" => vec![
            json!({
                "id": "V6-MEMORY-012.1",
                "claim": "memory_enable_causality_materializes_causality_graph_artifacts_with_edge_receipts",
                "evidence": {
                    "memory_command": memory_command,
                    "node_count": memory_payload.get("node_count").and_then(Value::as_u64).unwrap_or(0),
                    "edge_count": memory_payload.get("edge_count").and_then(Value::as_u64).unwrap_or(0),
                    "graph_path": memory_payload.get("graph_path").and_then(Value::as_str).unwrap_or("")
                }
            }),
            json!({
                "id": "V6-MEMORY-012.5",
                "claim": "causality_activation_commands_route_through_rust_core_with_deterministic_receipts",
                "evidence": {
                    "memory_command": memory_command,
                    "graph_path": memory_payload.get("graph_path").and_then(Value::as_str).unwrap_or("")
                }
            }),
        ],
        "memory-causal-retrieve" | "stable-memory-causal-retrieve" => vec![json!({
            "id": "V6-MEMORY-012.2",
            "claim": "memory_causal_retrieve_executes_deterministic_multi_hop_traversal_with_trace_receipts",
            "evidence": {
                "memory_command": memory_command,
                "depth": parse_arg_value(memory_args, "depth").and_then(|v| v.parse::<u64>().ok()).unwrap_or(2),
                "trace_count": memory_payload.get("trace_count").and_then(Value::as_u64).unwrap_or(0),
                "query": memory_payload.get("query").and_then(Value::as_str).unwrap_or("")
            }
        })],
        "memory-benchmark-ama" | "stable-memory-benchmark-ama" => vec![
            json!({
                "id": "V6-MEMORY-012.3",
                "claim": "memory_benchmark_ama_emits_reproducible_scored_benchmark_receipts",
                "evidence": {
                    "memory_command": memory_command,
                    "ama_score": memory_payload.get("ama_score").cloned().unwrap_or(Value::Null),
                    "pass": memory_payload.get("pass").and_then(Value::as_bool).unwrap_or(false),
                    "benchmark_path": memory_payload.get("benchmark_path").and_then(Value::as_str).unwrap_or("")
                }
            }),
            json!({
                "id": "V6-MEMORY-012.5",
                "claim": "ama_benchmark_commands_route_through_rust_core_with_deterministic_receipts",
                "evidence": {
                    "memory_command": memory_command,
                    "benchmark_path": memory_payload.get("benchmark_path").and_then(Value::as_str).unwrap_or("")
                }
            }),
        ],
        "memory-fuse" | "stable-memory-fuse" => vec![json!({
            "id": "V6-MEMORY-012.4",
            "claim": "memory_fuse_computes_4w_causality_metacognition_fusion_snapshots_with_score_receipts",
            "evidence": {
                "memory_command": memory_command,
                "fusion_score": memory_payload.get("fusion_score").cloned().unwrap_or(Value::Null),
                "fusion_state_path": memory_payload.get("fusion_state_path").and_then(Value::as_str).unwrap_or("")
            }
        })],
        _ => Vec::new(),
    }
}

