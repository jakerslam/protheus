// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::flow_plane (authoritative)

use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_plane_conduit_enforcement, canonical_json_string,
    conduit_bypass_requested, emit_plane_receipt, load_json_or, parse_bool, parse_u64,
    plane_status, print_json, read_json, scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, parse_args};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "FLOW_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "flow_plane";

const CANVAS_COMPILE_CONTRACT_PATH: &str =
    "planes/contracts/flow/canvas_execution_graph_contract_v1.json";
const PLAYGROUND_CONTRACT_PATH: &str = "planes/contracts/flow/step_playground_contract_v1.json";
const COMPONENT_MARKETPLACE_CONTRACT_PATH: &str =
    "planes/contracts/flow/component_marketplace_contract_v1.json";
const COMPONENT_MARKETPLACE_MANIFEST_PATH: &str =
    "planes/contracts/flow/component_marketplace_manifest_v1.json";
const EXPORT_CONTRACT_PATH: &str = "planes/contracts/flow/export_compiler_contract_v1.json";
const TEMPLATE_GOVERNANCE_CONTRACT_PATH: &str =
    "planes/contracts/flow/template_governance_contract_v1.json";
const TEMPLATE_MANIFEST_PATH: &str = "planes/contracts/flow/template_pack_manifest_v1.json";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops flow-plane status");
    println!("  protheus-ops flow-plane compile [--canvas-json=<json>|--canvas-path=<path>] [--strict=1|0]");
    println!(
        "  protheus-ops flow-plane run [--run-id=<id>] [--strict=1|0]   # alias of playground --op=play"
    );
    println!("  protheus-ops flow-plane playground --op=<play|pause|step|resume|inspect> [--run-id=<id>] [--strict=1|0]");
    println!("  protheus-ops flow-plane component-marketplace [--manifest=<path>] [--components-root=<path>] [--component-id=<id>] [--custom-source-path=<path>] [--strict=1|0]");
    println!("  protheus-ops flow-plane export [--format=json|api|mcp] [--from-path=<path>] [--package-version=<v>] [--strict=1|0]");
    println!(
        "  protheus-ops flow-plane install [--manifest=<path>] [--templates-root=<path>] [--strict=1|0]   # alias of template-governance"
    );
    println!("  protheus-ops flow-plane template-governance [--manifest=<path>] [--templates-root=<path>] [--strict=1|0]");
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(root, STATE_ENV, STATE_SCOPE, "flow_plane_error", payload)
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "flow_conduit_enforcement",
        "core/layer0/ops/flow_plane",
        bypass_requested,
        "visual_builder_compile_run_debug_export_install_actions_route_through_conduit_with_bypass_rejection",
        &["V6-FLOW-001.6"],
    )
}

fn status(root: &Path) -> Value {
    plane_status(root, STATE_ENV, STATE_SCOPE, "flow_plane_status")
}

fn parse_canvas_input(root: &Path, parsed: &crate::ParsedArgs) -> Result<(String, Value), String> {
    if let Some(raw) = parsed.flags.get("canvas-json") {
        let value =
            serde_json::from_str::<Value>(raw).map_err(|_| "canvas_json_invalid".to_string())?;
        return Ok(("canvas-json".to_string(), value));
    }
    if let Some(rel_or_abs) = parsed.flags.get("canvas-path") {
        let path = if Path::new(rel_or_abs).is_absolute() {
            PathBuf::from(rel_or_abs)
        } else {
            root.join(rel_or_abs)
        };
        let value =
            read_json(&path).ok_or_else(|| format!("canvas_path_not_found:{}", path.display()))?;
        return Ok((path.display().to_string(), value));
    }
    Err("canvas_required".to_string())
}

fn normalize_tooling_capability_token(raw: &str) -> String {
    clean(raw, 80)
        .to_ascii_lowercase()
        .replace('-', "_")
        .replace(' ', "_")
}

fn collect_canvas_tooling_capabilities(nodes: &[Value]) -> Vec<String> {
    let mut capabilities = BTreeSet::<String>::new();
    for node in nodes {
        for key in ["tool", "tool_name", "capability", "lane", "provider"] {
            let token = node
                .get(key)
                .and_then(Value::as_str)
                .map(normalize_tooling_capability_token)
                .unwrap_or_default();
            if !token.is_empty() {
                capabilities.insert(token);
            }
        }
        if let Some(tags) = node.get("tags").and_then(Value::as_array) {
            for tag in tags {
                let token = tag
                    .as_str()
                    .map(normalize_tooling_capability_token)
                    .unwrap_or_default();
                if !token.is_empty() {
                    capabilities.insert(token);
                }
            }
        }
    }
    capabilities.into_iter().collect()
}

fn run_compile(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CANVAS_COMPILE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "flow_canvas_execution_graph_contract",
            "allowed_node_types": ["source", "transform", "sink", "component"],
            "max_nodes": 512,
            "max_edges": 4096
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("flow_compile_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "flow_canvas_execution_graph_contract"
    {
        errors.push("flow_compile_contract_kind_invalid".to_string());
    }

    let (source_hint, canvas) = match parse_canvas_input(root, parsed) {
        Ok(v) => v,
        Err(err) => {
            errors.push(err);
            ("".to_string(), Value::Null)
        }
    };
    if canvas.is_null() {
        errors.push("canvas_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "flow_plane_compile",
            "errors": errors
        });
    }

    let version = canvas
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let kind = canvas
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if strict && version != "v1" {
        errors.push("canvas_version_must_be_v1".to_string());
    }
    if strict && kind != "flow_canvas_graph" {
        errors.push("canvas_kind_invalid".to_string());
    }

    let nodes = canvas
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let edges = canvas
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let max_nodes = contract
        .get("max_nodes")
        .and_then(Value::as_u64)
        .unwrap_or(512) as usize;
    let max_edges = contract
        .get("max_edges")
        .and_then(Value::as_u64)
        .unwrap_or(4096) as usize;
    let allowed_types = contract
        .get("allowed_node_types")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 60))
        .collect::<Vec<_>>();

    if nodes.is_empty() {
        errors.push("canvas_nodes_required".to_string());
    }
    if strict && nodes.len() > max_nodes {
        errors.push("canvas_node_limit_exceeded".to_string());
    }
    if strict && edges.len() > max_edges {
        errors.push("canvas_edge_limit_exceeded".to_string());
    }

    let mut node_ids = BTreeSet::<String>::new();
    let mut node_meta = BTreeMap::<String, String>::new();
    for node in &nodes {
        let id = clean(
            node.get("id").and_then(Value::as_str).unwrap_or_default(),
            120,
        );
        let typ = clean(
            node.get("type").and_then(Value::as_str).unwrap_or_default(),
            60,
        );
        if id.is_empty() {
            errors.push("node_id_required".to_string());
            continue;
        }
        if !node_ids.insert(id.clone()) {
            errors.push(format!("duplicate_node_id:{id}"));
        }
        if strict && !allowed_types.iter().any(|row| row == &typ) {
            errors.push(format!("node_type_not_allowed:{id}:{typ}"));
        }
        node_meta.insert(id, typ);
    }

    let mut indegree = BTreeMap::<String, usize>::new();
    let mut adjacency = BTreeMap::<String, Vec<String>>::new();
    for id in &node_ids {
        indegree.insert(id.clone(), 0usize);
        adjacency.insert(id.clone(), Vec::new());
    }

    for edge in &edges {
        let from = clean(
            edge.get("from").and_then(Value::as_str).unwrap_or_default(),
            120,
        );
        let to = clean(
            edge.get("to").and_then(Value::as_str).unwrap_or_default(),
            120,
        );
        if from.is_empty() || to.is_empty() {
            errors.push("edge_from_to_required".to_string());
            continue;
        }
        if !node_ids.contains(&from) || !node_ids.contains(&to) {
            errors.push(format!("edge_ref_missing_node:{from}->{to}"));
            continue;
        }
        adjacency.entry(from).or_default().push(to.clone());
        *indegree.entry(to).or_insert(0usize) += 1;
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "flow_plane_compile",
            "errors": errors
        });
    }

    let mut queue = indegree
        .iter()
        .filter_map(|(id, deg)| if *deg == 0 { Some(id.clone()) } else { None })
        .collect::<Vec<_>>();
    queue.sort();
    let mut q = VecDeque::from(queue);
    let mut execution_order = Vec::<String>::new();
    while let Some(id) = q.pop_front() {
        execution_order.push(id.clone());
        let mut children = adjacency.get(&id).cloned().unwrap_or_default();
        children.sort();
        for child in children {
            if let Some(deg) = indegree.get_mut(&child) {
                *deg = deg.saturating_sub(1);
                if *deg == 0 {
                    q.push_back(child);
                }
            }
        }
    }
    if strict && execution_order.len() != node_ids.len() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "flow_plane_compile",
            "errors": ["cycle_detected_in_canvas_graph"]
        });
    }

    let compiled_nodes = execution_order
        .iter()
        .enumerate()
        .map(|(idx, id)| {
            json!({
                "id": id,
                "type": node_meta.get(id).cloned().unwrap_or_else(|| "component".to_string()),
                "execution_index": idx
            })
        })
        .collect::<Vec<_>>();
    let stage_receipts = vec![
        json!({
            "stage": "schema_validate",
            "node_count": nodes.len(),
            "edge_count": edges.len(),
            "source_hint": source_hint
        }),
        json!({
            "stage": "compile_graph",
            "execution_nodes": compiled_nodes.len(),
            "order_sha256": sha256_hex_str(&execution_order.join(","))
        }),
    ];
    let tooling_capabilities = collect_canvas_tooling_capabilities(&nodes);
    let tooling_capability_count = tooling_capabilities.len();

    let compiled = json!({
        "version": "v1",
        "kind": "flow_execution_graph",
        "source_hint": source_hint,
        "execution_order": execution_order,
        "nodes": compiled_nodes,
        "edges": edges,
        "tooling_capabilities": tooling_capabilities,
        "stage_receipts": stage_receipts
    });
    let artifact_path = state_root(root).join("compile").join("latest.json");
    let _ = write_json(&artifact_path, &compiled);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "flow_plane_compile",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&compiled.to_string())
        },
        "compiled_graph": compiled,
        "claim_evidence": [
            {
                "id": "V6-FLOW-001.1",
                "claim": "canvas_graph_compiles_into_execution_graph_with_live_schema_validation_and_deterministic_compile_receipts",
                "evidence": {
                    "node_count": nodes.len(),
                    "edge_count": edges.len(),
                    "execution_count": execution_order.len(),
                    "tooling_capability_count": tooling_capability_count
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
