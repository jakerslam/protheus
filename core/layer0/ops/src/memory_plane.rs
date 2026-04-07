// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::memory_plane (authoritative)
use crate::contract_lane_utils as lane_utils;
use crate::{client_state_root, deterministic_receipt_hash, now_iso};
use protheus_memory_core_v1::{
    memory_scope_authority_matrix, owner_export_redaction_matrix, task_fabric_lease_cas_rules,
    trust_state_transition_matrix, DefaultVerityMemoryPolicy, UnifiedMemoryHeap,
};
use serde_json::{json, Value};
use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

const LANE_ID: &str = "memory_plane";

#[derive(Debug, Clone)]
struct MemoryPlanePolicy {
    max_graph_nodes: usize,
    max_federation_entries: usize,
}

#[path = "memory_plane_federation.rs"]
mod memory_plane_federation;
use memory_plane_federation::{
    federation_pull_payload, federation_status_payload, federation_sync_payload,
};

#[cfg(test)]
#[path = "memory_plane_tests.rs"]
mod tests;

fn usage() {
    println!("Usage:");
    println!("  protheus-ops memory-plane causal-temporal-graph <record|blame|status> [flags]");
    println!("  protheus-ops memory-plane memory-federation-plane <sync|pull|status> [flags]");
    println!("  protheus-ops memory-plane unified-heap status");
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn receipt_hash(v: &Value) -> String {
    deterministic_receipt_hash(v)
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    lane_utils::parse_flag(argv, key, true)
}

fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw, fallback)
}

fn clean_id(raw: Option<&str>, fallback: &str) -> String {
    lane_utils::clean_token(raw, fallback)
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(raw, max_len)
}

fn parse_csv(raw: Option<&str>, max_items: usize) -> Vec<String> {
    raw.unwrap_or("")
        .split(',')
        .map(|v| clean_id(Some(v), ""))
        .filter(|v| !v.is_empty())
        .take(max_items)
        .collect::<Vec<_>>()
}

fn parse_json(raw: Option<&str>) -> Result<Value, String> {
    let text = raw.ok_or_else(|| "missing_json_payload".to_string())?;
    serde_json::from_str::<Value>(text).map_err(|err| format!("invalid_json_payload:{err}"))
}

fn write_json(path: &Path, payload: &Value) -> Result<(), String> {
    lane_utils::write_json(path, payload)
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, row)
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn rel_path(root: &Path, path: &Path) -> String {
    lane_utils::rel_path(root, path)
}

fn policy_path(root: &Path) -> PathBuf {
    root.join("client")
        .join("runtime")
        .join("config")
        .join("memory_plane_policy.json")
}

fn default_policy() -> MemoryPlanePolicy {
    MemoryPlanePolicy {
        max_graph_nodes: 25_000,
        max_federation_entries: 25_000,
    }
}

fn load_policy(root: &Path) -> MemoryPlanePolicy {
    let mut out = default_policy();
    if let Some(v) = read_json(&policy_path(root)) {
        out.max_graph_nodes = v
            .get("max_graph_nodes")
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .filter(|n| *n >= 100)
            .unwrap_or(out.max_graph_nodes);
        out.max_federation_entries = v
            .get("max_federation_entries")
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .filter(|n| *n >= 100)
            .unwrap_or(out.max_federation_entries);
    }
    out
}

fn graph_latest_path(root: &Path) -> PathBuf {
    client_state_root(root)
        .join("memory")
        .join("causal_temporal_graph")
        .join("latest.json")
}

fn graph_history_path(root: &Path) -> PathBuf {
    client_state_root(root)
        .join("memory")
        .join("causal_temporal_graph")
        .join("history.jsonl")
}

fn load_graph(root: &Path) -> Value {
    read_json(&graph_latest_path(root)).unwrap_or_else(|| {
        json!({
            "version": 1,
            "nodes": {},
            "edges": []
        })
    })
}

fn graph_record_payload(
    root: &Path,
    policy: &MemoryPlanePolicy,
    argv: &[String],
) -> Result<Value, String> {
    let event_id = clean_id(parse_flag(argv, "event-id").as_deref(), "event");
    let actor = clean_id(parse_flag(argv, "actor").as_deref(), "unknown");
    let summary = clean_text(parse_flag(argv, "summary").as_deref(), 240);
    let caused_by = parse_csv(parse_flag(argv, "caused-by").as_deref(), 32);
    let apply = parse_bool(parse_flag(argv, "apply").as_deref(), true);
    let ts = now_iso();

    let mut graph = load_graph(root);
    let mut nodes = graph
        .get("nodes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if nodes.len() + 1 > policy.max_graph_nodes {
        return Err("graph_capacity_exceeded".to_string());
    }
    nodes.insert(
        event_id.clone(),
        json!({
            "event_id": event_id,
            "ts": ts,
            "actor": actor,
            "summary": summary,
            "caused_by": caused_by
        }),
    );

    let mut edge_set = BTreeSet::new();
    if let Some(edges) = graph.get("edges").and_then(Value::as_array) {
        for edge in edges {
            let from = edge.get("from").and_then(Value::as_str).unwrap_or("");
            let to = edge.get("to").and_then(Value::as_str).unwrap_or("");
            if !from.is_empty() && !to.is_empty() {
                edge_set.insert((from.to_string(), to.to_string()));
            }
        }
    }
    for parent in parse_csv(parse_flag(argv, "caused-by").as_deref(), 32) {
        edge_set.insert((parent, event_id.clone()));
    }

    let edges = edge_set
        .into_iter()
        .map(|(from, to)| json!({ "from": from, "to": to }))
        .collect::<Vec<_>>();
    graph["nodes"] = Value::Object(nodes);
    graph["edges"] = Value::Array(edges);
    graph["updated_at"] = Value::String(ts.clone());

    if apply {
        write_json(&graph_latest_path(root), &graph)?;
        append_jsonl(
            &graph_history_path(root),
            &json!({
                "type": "causal_temporal_graph_record",
                "event_id": event_id,
                "ts": ts,
                "node_count": graph.get("nodes").and_then(Value::as_object).map(|m| m.len()).unwrap_or(0),
            }),
        )?;
    }

    let mut out = json!({
        "ok": true,
        "type": "causal_temporal_graph_record",
        "lane": LANE_ID,
        "event_id": event_id,
        "apply": apply,
        "graph_path": rel_path(root, &graph_latest_path(root)),
        "node_count": graph.get("nodes").and_then(Value::as_object).map(|m| m.len()).unwrap_or(0),
        "edge_count": graph.get("edges").and_then(Value::as_array).map(|m| m.len()).unwrap_or(0),
        "claim_evidence": [{
            "id": "causality_recorded_for_blame",
            "claim": "events_record_parents_for_deterministic_blame_attribution",
            "evidence": {
                "event_id": event_id,
                "caused_by_count": parse_csv(parse_flag(argv, "caused-by").as_deref(), 32).len()
            }
        }]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}

fn graph_blame_payload(root: &Path, argv: &[String]) -> Result<Value, String> {
    let event_id = clean_id(parse_flag(argv, "event-id").as_deref(), "");
    if event_id.is_empty() {
        return Err("event_id_missing".to_string());
    }
    let max_depth = parse_flag(argv, "max-depth")
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(8);
    let graph = load_graph(root);
    let nodes = graph
        .get("nodes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if !nodes.contains_key(&event_id) {
        return Err("event_not_found".to_string());
    }

    let mut ancestry = Vec::new();
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::new();
    queue.push_back((event_id.clone(), 0usize));
    seen.insert(event_id.clone());

    while let Some((current, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        if let Some(node) = nodes.get(&current) {
            let parents = node
                .get("caused_by")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for parent in parents {
                let parent_id = clean_id(parent.as_str(), "");
                if parent_id.is_empty() || !seen.insert(parent_id.clone()) {
                    continue;
                }
                ancestry.push(json!({
                    "event_id": parent_id,
                    "distance": depth + 1
                }));
                queue.push_back((parent_id, depth + 1));
            }
        }
    }

    let mut out = json!({
        "ok": true,
        "type": "causal_temporal_graph_blame",
        "lane": LANE_ID,
        "event_id": event_id,
        "max_depth": max_depth,
        "ancestry": ancestry,
        "root_causes": ancestry.iter().filter_map(|row| {
            let id = row.get("event_id").and_then(Value::as_str)?;
            let node = nodes.get(id)?;
            let parent_count = node.get("caused_by").and_then(Value::as_array).map(|r| r.len()).unwrap_or(0);
            if parent_count == 0 { Some(Value::String(id.to_string())) } else { None }
        }).collect::<Vec<_>>()
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}

fn graph_status_payload(root: &Path) -> Value {
    let graph = load_graph(root);
    let mut out = json!({
        "ok": true,
        "type": "causal_temporal_graph_status",
        "lane": LANE_ID,
        "graph_path": rel_path(root, &graph_latest_path(root)),
        "history_path": rel_path(root, &graph_history_path(root)),
        "node_count": graph.get("nodes").and_then(Value::as_object).map(|m| m.len()).unwrap_or(0),
        "edge_count": graph.get("edges").and_then(Value::as_array).map(|m| m.len()).unwrap_or(0),
        "updated_at": graph.get("updated_at").cloned().unwrap_or(Value::Null)
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn unified_heap_status_payload() -> Value {
    let heap = UnifiedMemoryHeap::new(DefaultVerityMemoryPolicy);
    let mut out = json!({
        "ok": true,
        "type": "unified_memory_heap_status",
        "lane": LANE_ID,
        "authority": "core/layer2/memory",
        "matrices": {
            "scope_authority": memory_scope_authority_matrix(),
            "trust_state_transition": trust_state_transition_matrix(),
            "owner_export_redaction": owner_export_redaction_matrix(),
            "task_fabric_lease_cas": task_fabric_lease_cas_rules(),
        },
        "counts": {
            "receipts": heap.receipts().len(),
            "graph_edges": heap.graph_subsystem().edges().len(),
            "objects": heap.record_store().all_objects().len(),
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn cli_error(argv: &[String], err: &str, exit_code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "memory_plane_cli_error",
        "lane": LANE_ID,
        "argv": argv,
        "error": err,
        "exit_code": exit_code,
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() {
        usage();
        print_json_line(&cli_error(argv, "missing_surface", 2));
        return 2;
    }
    let policy = load_policy(root);
    let surface = argv[0].trim().to_ascii_lowercase();
    let command = argv
        .get(1)
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    let result = match (surface.as_str(), command.as_str()) {
        ("causal-temporal-graph", "record") => graph_record_payload(root, &policy, &argv[2..]),
        ("causal-temporal-graph", "blame") => graph_blame_payload(root, &argv[2..]),
        ("causal-temporal-graph", "status") => Ok(graph_status_payload(root)),
        ("memory-federation-plane", "sync") => federation_sync_payload(root, &policy, &argv[2..]),
        ("memory-federation-plane", "pull") => Ok(federation_pull_payload(root, &argv[2..])),
        ("memory-federation-plane", "status") => Ok(federation_status_payload(root)),
        ("unified-heap", "status") => Ok(unified_heap_status_payload()),
        _ => Err("unknown_command".to_string()),
    };

    match result {
        Ok(payload) => {
            let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&payload);
            if ok {
                0
            } else {
                1
            }
        }
        Err(err) => {
            if err == "unknown_command" {
                usage();
            }
            print_json_line(&cli_error(argv, &err, 2));
            2
        }
    }
}
