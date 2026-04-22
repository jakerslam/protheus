// SPDX-License-Identifier: Apache-2.0

use super::{input::GraphData, ROUTE_TAG, STATE_ENV, STATE_SCOPE};
use crate::directive_kernel;
use crate::v8_kernel::{
    parse_bool, print_json, read_json, scoped_state_root, sha256_hex_str, write_json, write_receipt,
};
use crate::{clean, deterministic_receipt_hash, now_iso, ParsedArgs};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

pub fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn direct_error_payload(type_name: &str, error: &str) -> Value {
    let mut out = json!({
        "ok": false,
        "type": type_name,
        "lane": "core/layer0/ops",
        "error": error,
        "exit_code": 2
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn materialized_path(root: &Path, command: &str, cache_key: &str) -> PathBuf {
    state_root(root)
        .join("materialized")
        .join(command)
        .join(format!("{cache_key}.json"))
}

pub fn emit(root: &Path, payload: Value) -> i32 {
    match write_receipt(root, STATE_ENV, STATE_SCOPE, payload) {
        Ok(out) => {
            print_json(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                2
            }
        }
        Err(err) => {
            let out = direct_error_payload("graph_toolkit_error", &clean(err, 240));
            print_json(&out);
            2
        }
    }
}

fn cache_key(command: &str, graph_hash: &str, params: &Value) -> String {
    sha256_hex_str(&format!(
        "{command}:{graph_hash}:{}",
        crate::deterministic_receipt_hash(params)
    ))
}

fn claim_evidence(id: &str, command: &str, cached: bool) -> Vec<Value> {
    vec![
        json!({
            "id": id,
            "claim": "graph_algorithm_execution_is_native_and_receipted",
            "evidence": {
                "command": command,
                "cached": cached
            }
        }),
        json!({
            "id": "V6-TOOLS-008.5",
            "claim": "graph_runs_are_gate_checked_and_receipted_for_conduit_routing",
            "evidence": {
                "routed_via": ROUTE_TAG,
                "conduit_required": true
            }
        }),
    ]
}

pub fn materialize_and_emit(
    root: &Path,
    parsed: &ParsedArgs,
    command: &str,
    type_name: &str,
    claim_id: &str,
    action: &str,
    graph: &GraphData,
    params: Value,
    computed_result: Value,
) -> i32 {
    if !directive_kernel::action_allowed(root, action) {
        return emit(
            root,
            json!({
                "ok": false,
                "type": type_name,
                "lane": "core/layer0/ops",
                "error": "directive_gate_denied",
                "action": action,
                "routed_via": ROUTE_TAG,
                "conduit_required": true,
                "graph_hash": graph.graph_hash,
                "layer_map": ["0","1","2","client"],
                "claim_evidence": claim_evidence(claim_id, command, false)
            }),
        );
    }

    let refresh = parse_bool(parsed.flags.get("refresh"), false);
    let key = cache_key(command, &graph.graph_hash, &params);
    let artifact = materialized_path(root, command, &key);
    let mut cached = false;
    let result = if !refresh && artifact.exists() {
        match read_json(&artifact).and_then(|v| v.get("result").cloned()) {
            Some(value) => {
                cached = true;
                value
            }
            None => computed_result,
        }
    } else {
        computed_result
    };

    if !cached {
        let materialized = json!({
            "command": command,
            "graph_hash": graph.graph_hash,
            "dataset_source": graph.source,
            "params": params,
            "result": result,
            "ts": now_iso()
        });
        if let Err(err) = write_json(&artifact, &materialized) {
            return emit(root, direct_error_payload(type_name, &clean(err, 220)));
        }
    }

    emit(
        root,
        json!({
            "ok": true,
            "type": type_name,
            "lane": "core/layer0/ops",
            "routed_via": ROUTE_TAG,
            "conduit_required": true,
            "command": command,
            "action": action,
            "graph_hash": graph.graph_hash,
            "dataset_source": graph.source,
            "directed": graph.directed,
            "node_count": graph.nodes.len(),
            "edge_count": graph.existing_edges.len(),
            "cache_key": key,
            "cached": cached,
            "artifact_path": artifact.display().to_string(),
            "params": params,
            "result": result,
            "warnings": graph.warnings,
            "layer_map": ["0","1","2","client"],
            "claim_evidence": claim_evidence(claim_id, command, cached)
        }),
    )
}

pub fn command_status(root: &Path) -> i32 {
    let latest = read_json(&latest_path(root));
    let materialized_root = state_root(root).join("materialized");
    let materialized_count = if materialized_root.exists() {
        walkdir::WalkDir::new(&materialized_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_file())
            .count()
    } else {
        0usize
    };
    emit(
        root,
        json!({
            "ok": true,
            "type": "graph_toolkit_status",
            "lane": "core/layer0/ops",
            "routed_via": ROUTE_TAG,
            "conduit_required": true,
            "latest": latest,
            "state_root": state_root(root).display().to_string(),
            "materialized_artifact_count": materialized_count,
            "layer_map": ["0","1","2","client"]
        }),
    )
}
