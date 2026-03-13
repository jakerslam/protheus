// SPDX-License-Identifier: Apache-2.0

use crate::v8_kernel::{parse_bool, sha256_hex_str};
use crate::{clean, ParsedArgs};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GraphData {
    pub nodes: Vec<String>,
    pub out_adj: Vec<Vec<(usize, f64)>>,
    pub undirected_adj: Vec<Vec<(usize, f64)>>,
    pub existing_edges: BTreeSet<(usize, usize)>,
    pub graph_hash: String,
    pub directed: bool,
    pub source: String,
    pub warnings: Vec<String>,
}

fn parse_graph_file(root: &Path, value: &str, source: String) -> Result<GraphData, String> {
    let path = root.join(value);
    let raw = std::fs::read_to_string(&path)
        .map_err(|err| format!("graph_file_read_failed:{}:{err}", path.display()))?;
    parse_graph_json_value(
        serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("graph_file_parse_failed:{}:{err}", path.display()))?,
        source,
    )
}

fn demo_graph_json() -> Value {
    json!({
        "directed": false,
        "nodes": [
            {"id": "alpha"},
            {"id": "beta"},
            {"id": "gamma"},
            {"id": "delta"},
            {"id": "epsilon"},
            {"id": "zeta"}
        ],
        "edges": [
            {"from": "alpha", "to": "beta"},
            {"from": "beta", "to": "gamma"},
            {"from": "alpha", "to": "gamma"},
            {"from": "delta", "to": "epsilon"},
            {"from": "epsilon", "to": "zeta"},
            {"from": "delta", "to": "zeta"},
            {"from": "gamma", "to": "delta", "weight": 0.2}
        ]
    })
}

fn dataset_candidates(root: &Path, dataset: &str) -> Vec<PathBuf> {
    match dataset {
        "memory-vault" => vec![
            root.join("core/local/state/memory/causal_temporal_graph/latest.json"),
            root.join("core/local/state/ops/rag/memory_graph.json"),
            root.join("client/runtime/local/state/memory/causal_temporal_graph/latest.json"),
        ],
        "code-graph" => vec![
            root.join("core/local/state/graph/code_graph/latest.json"),
            root.join("core/local/state/ops/graph/code_graph/latest.json"),
            root.join("client/runtime/local/state/graph/code_graph/latest.json"),
        ],
        _ => Vec::new(),
    }
}

pub fn parse_graph_input(root: &Path, parsed: &ParsedArgs) -> Result<GraphData, String> {
    if let Some(raw) = parsed.flags.get("graph-json") {
        let value = serde_json::from_str::<Value>(raw)
            .map_err(|err| format!("graph_json_invalid:{err}"))?;
        return parse_graph_json_value(value, "flag:graph-json".to_string());
    }
    if let Some(encoded) = parsed.flags.get("graph-json-base64") {
        let bytes = BASE64_STANDARD
            .decode(encoded.as_bytes())
            .map_err(|err| format!("graph_json_base64_invalid:{err}"))?;
        let raw =
            String::from_utf8(bytes).map_err(|err| format!("graph_json_utf8_invalid:{err}"))?;
        let value = serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("graph_json_invalid:{err}"))?;
        return parse_graph_json_value(value, "flag:graph-json-base64".to_string());
    }
    if let Some(file) = parsed.flags.get("graph-file") {
        return parse_graph_file(root, file, "flag:graph-file".to_string());
    }

    let dataset = parsed
        .flags
        .get("dataset")
        .map(|v| clean(v, 80).to_ascii_lowercase())
        .unwrap_or_else(|| "memory-vault".to_string());
    for candidate in dataset_candidates(root, &dataset) {
        if !candidate.exists() {
            continue;
        }
        let raw = std::fs::read_to_string(&candidate)
            .map_err(|err| format!("dataset_read_failed:{}:{err}", candidate.display()))?;
        let value = serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("dataset_parse_failed:{}:{err}", candidate.display()))?;
        let mut graph =
            parse_graph_json_value(value, format!("dataset:{dataset}:{}", candidate.display()))?;
        graph.warnings = Vec::new();
        return Ok(graph);
    }

    let fail_if_missing = parse_bool(parsed.flags.get("fail-if-missing-dataset"), false);
    if fail_if_missing {
        return Err(format!("dataset_not_found:{dataset}"));
    }

    let mut graph = parse_graph_json_value(demo_graph_json(), format!("dataset:{dataset}:demo"))?;
    graph
        .warnings
        .push(format!("dataset_missing_fallback_demo:{dataset}"));
    Ok(graph)
}

fn node_id(value: &Value) -> Option<String> {
    if let Some(raw) = value.as_str() {
        let out = clean(raw, 120);
        return if out.is_empty() { None } else { Some(out) };
    }
    let obj = value.as_object()?;
    for key in ["id", "name", "node"] {
        if let Some(raw) = obj.get(key).and_then(Value::as_str) {
            let out = clean(raw, 120);
            if !out.is_empty() {
                return Some(out);
            }
        }
    }
    None
}

fn edge_end(value: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(raw) = value.get(*key).and_then(Value::as_str) {
            let out = clean(raw, 120);
            if !out.is_empty() {
                return Some(out);
            }
        }
    }
    None
}

fn parse_graph_json_value(mut value: Value, source: String) -> Result<GraphData, String> {
    if let Some(nested) = value.get_mut("graph") {
        value = nested.take();
    }
    let obj = value
        .as_object()
        .ok_or_else(|| "graph_payload_not_object".to_string())?;
    let directed = obj
        .get("directed")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut node_set = BTreeSet::<String>::new();
    if let Some(nodes) = obj.get("nodes").and_then(Value::as_array) {
        for row in nodes {
            if let Some(id) = node_id(row) {
                node_set.insert(id);
            }
        }
    }

    let mut parsed_edges = Vec::<(String, String, f64)>::new();
    if let Some(edges) = obj.get("edges").and_then(Value::as_array) {
        for row in edges {
            let Some(edge_obj) = row.as_object() else {
                continue;
            };
            let Some(from) = edge_end(edge_obj, &["from", "source", "u"]) else {
                continue;
            };
            let Some(to) = edge_end(edge_obj, &["to", "target", "v"]) else {
                continue;
            };
            let weight = edge_obj
                .get("weight")
                .and_then(Value::as_f64)
                .unwrap_or(1.0)
                .max(1e-9);
            node_set.insert(from.clone());
            node_set.insert(to.clone());
            parsed_edges.push((from, to, weight));
        }
    }

    if node_set.is_empty() {
        return Err("graph_payload_has_no_nodes".to_string());
    }

    let nodes = node_set.into_iter().collect::<Vec<_>>();
    let index = nodes
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect::<BTreeMap<_, _>>();
    let mut out_weights = BTreeMap::<(usize, usize), f64>::new();
    let mut und_weights = BTreeMap::<(usize, usize), f64>::new();
    let mut existing_edges = BTreeSet::<(usize, usize)>::new();

    for (from, to, weight) in parsed_edges {
        let Some(from_idx) = index.get(&from).copied() else {
            continue;
        };
        let Some(to_idx) = index.get(&to).copied() else {
            continue;
        };
        let ordered = if from_idx <= to_idx {
            (from_idx, to_idx)
        } else {
            (to_idx, from_idx)
        };

        if directed {
            *out_weights.entry((from_idx, to_idx)).or_insert(0.0) += weight;
        } else {
            *out_weights.entry((from_idx, to_idx)).or_insert(0.0) += weight;
            if from_idx != to_idx {
                *out_weights.entry((to_idx, from_idx)).or_insert(0.0) += weight;
            }
        }
        *und_weights.entry(ordered).or_insert(0.0) += weight;
        existing_edges.insert(ordered);
    }

    let mut out_adj = vec![Vec::<(usize, f64)>::new(); nodes.len()];
    for ((from, to), weight) in out_weights {
        out_adj[from].push((to, weight));
    }
    for row in &mut out_adj {
        row.sort_by_key(|(idx, _)| *idx);
    }

    let mut undirected_adj = vec![Vec::<(usize, f64)>::new(); nodes.len()];
    for ((a, b), weight) in &und_weights {
        undirected_adj[*a].push((*b, *weight));
        if *a != *b {
            undirected_adj[*b].push((*a, *weight));
        }
    }
    for row in &mut undirected_adj {
        row.sort_by_key(|(idx, _)| *idx);
    }

    let canonical_edges = und_weights
        .iter()
        .map(|((a, b), w)| {
            json!({
                "a": nodes[*a],
                "b": nodes[*b],
                "weight": ((*w * 1_000_000.0).round() / 1_000_000.0)
            })
        })
        .collect::<Vec<_>>();
    let graph_hash = sha256_hex_str(
        &serde_json::to_string(&json!({
            "directed": directed,
            "nodes": nodes,
            "edges": canonical_edges
        }))
        .unwrap_or_default(),
    );

    Ok(GraphData {
        nodes,
        out_adj,
        undirected_adj,
        existing_edges,
        graph_hash,
        directed,
        source,
        warnings: Vec::new(),
    })
}
