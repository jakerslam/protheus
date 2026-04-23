// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::conversation_eye_collector_kernel_support::{
    append_jsonl, clamp_u64, clean_edges, clean_tags, clean_text, normalize_index,
    normalize_topics, now_iso, read_json, read_jsonl_tail, resolve_path, sha16, to_iso_week,
    write_json_atomic,
};

const DEFAULT_HISTORY_REL: &str = "local/state/cockpit/inbox/history.jsonl";
const DEFAULT_LATEST_REL: &str = "local/state/cockpit/inbox/latest.json";
const DEFAULT_MEMORY_DIR_REL: &str = "local/state/memory/conversation_eye";

fn usage() {
    println!("conversation-eye-collector-kernel commands:");
    println!(
        "  infring-ops conversation-eye-collector-kernel begin-collection --payload-base64=<json>"
    );
    println!("  infring-ops conversation-eye-collector-kernel preflight --payload-base64=<json>");
    println!(
        "  infring-ops conversation-eye-collector-kernel load-source-rows --payload-base64=<json>"
    );
    println!(
        "  infring-ops conversation-eye-collector-kernel normalize-topics --payload-base64=<json>"
    );
    println!("  infring-ops conversation-eye-collector-kernel load-index --payload-base64=<json>");
    println!("  infring-ops conversation-eye-collector-kernel apply-node --payload-base64=<json>");
    println!(
        "  infring-ops conversation-eye-collector-kernel process-nodes --payload-base64=<json>"
    );
    println!(
        "  infring-ops conversation-eye-collector-kernel append-memory-row --payload-base64=<json>"
    );
    println!(
        "  infring-ops conversation-eye-collector-kernel append-memory-rows --payload-base64=<json>"
    );
    println!("  infring-ops conversation-eye-collector-kernel save-index --payload-base64=<json>");
}

fn nested_obj<'a>(payload: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
    payload.get(key).and_then(Value::as_object)
}

fn nested_u64(payload: &Map<String, Value>, parent: &str, key: &str) -> Option<u64> {
    nested_obj(payload, parent)
        .and_then(|obj| obj.get(key))
        .and_then(Value::as_u64)
}

fn default_memory_jsonl_rel() -> String {
    format!("{}/nodes.jsonl", DEFAULT_MEMORY_DIR_REL)
}

fn default_index_rel() -> String {
    format!("{}/index.json", DEFAULT_MEMORY_DIR_REL)
}

fn resolve_memory_jsonl_path(root: &Path, payload: &Map<String, Value>, key: &str) -> PathBuf {
    let fallback = default_memory_jsonl_rel();
    resolve_path(root, payload, key, fallback.as_str())
}

fn resolve_index_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let fallback = default_index_rel();
    resolve_path(root, payload, "index_path", fallback.as_str())
}

fn denied_node(index: &Value, reason: &str, quota_skipped: bool) -> Value {
    json!({
        "ok": true,
        "allowed": false,
        "reason": reason,
        "quota_skipped": quota_skipped,
        "index": index
    })
}

fn command_begin_collection(root: &Path, payload: &Map<String, Value>) -> Value {
    let max_items = nested_u64(payload, "budgets", "max_items")
        .or_else(|| payload.get("max_items").and_then(Value::as_u64))
        .unwrap_or(3)
        .clamp(1, 32);
    let max_rows = nested_u64(payload, "budgets", "max_rows")
        .or_else(|| payload.get("max_rows").and_then(Value::as_u64))
        .unwrap_or(24)
        .clamp(4, 256);
    let max_work_ms = nested_u64(payload, "budgets", "max_work_ms")
        .or_else(|| payload.get("max_work_ms").and_then(Value::as_u64))
        .unwrap_or(7_000)
        .clamp(1_000, 30_000);
    let weekly_node_limit = payload
        .get("weekly_node_limit")
        .and_then(Value::as_u64)
        .unwrap_or(10)
        .clamp(1, 50);
    let weekly_promotion_overrides = payload
        .get("weekly_promotion_overrides")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(0, 20);
    let eye_id = payload
        .get("eye_id")
        .and_then(Value::as_str)
        .map(|raw| clean_text(Some(raw), 80))
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "conversation_eye".to_string());
    let eye_topics = nested_obj(payload, "eye_config")
        .and_then(|cfg| cfg.get("topics"))
        .cloned()
        .unwrap_or_else(|| {
            payload
                .get("topics")
                .cloned()
                .unwrap_or(Value::Array(Vec::new()))
        });
    let history_path = resolve_path(root, payload, "history_path", DEFAULT_HISTORY_REL);
    let latest_path = resolve_path(root, payload, "latest_path", DEFAULT_LATEST_REL);
    let memory_jsonl_path = resolve_memory_jsonl_path(root, payload, "memory_jsonl_path");
    let index_path = resolve_index_path(root, payload);

    let preflight = command_preflight(
        root,
        lane_utils::payload_obj(&json!({
            "max_items": max_items,
            "history_path": history_path.display().to_string(),
            "latest_path": latest_path.display().to_string(),
            "eye_id": eye_id
        })),
    );
    if preflight.get("ok").and_then(Value::as_bool) != Some(true) {
        return json!({
            "ok": false,
            "success": false,
            "preflight": preflight
        });
    }

    let loaded_rows = command_load_source_rows(
        root,
        lane_utils::payload_obj(&json!({
            "history_path": history_path.display().to_string(),
            "latest_path": latest_path.display().to_string(),
            "max_rows": max_rows
        })),
    );
    let loaded_index = command_load_index(
        root,
        lane_utils::payload_obj(&json!({
            "index_path": index_path.display().to_string()
        })),
    );
    let normalized_topics = command_normalize_topics(lane_utils::payload_obj(&json!({
        "topics": eye_topics
    })));

    json!({
        "ok": true,
        "success": true,
        "eye": eye_id,
        "max_items": max_items,
        "max_rows": max_rows,
        "max_work_ms": max_work_ms,
        "weekly_node_limit": weekly_node_limit,
        "weekly_promotion_overrides": weekly_promotion_overrides,
        "history_path": history_path.display().to_string(),
        "latest_path": latest_path.display().to_string(),
        "memory_jsonl_path": memory_jsonl_path.display().to_string(),
        "index_path": index_path.display().to_string(),
        "source_rows": loaded_rows
            .get("rows")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        "topics": normalized_topics
            .get("topics")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        "index": loaded_index
            .get("index")
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new())),
        "preflight": preflight
    })
}

fn command_preflight(root: &Path, payload: &Map<String, Value>) -> Value {
    let max_items = payload
        .get("max_items")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let history_path = resolve_path(root, payload, "history_path", DEFAULT_HISTORY_REL);
    let latest_path = resolve_path(root, payload, "latest_path", DEFAULT_LATEST_REL);
    let history_exists = history_path.exists();
    let latest_exists = latest_path.exists();

    let mut checks = Vec::<Value>::new();
    let mut failures = Vec::<Value>::new();

    if max_items <= 0.0 {
        failures.push(json!({
            "code": "invalid_budget",
            "message": "budgets.max_items must be > 0"
        }));
    } else {
        checks.push(json!({
            "name": "max_items_valid",
            "ok": true,
            "value": max_items
        }));
    }

    if !history_exists && !latest_exists {
        failures.push(json!({
            "code": "conversation_source_missing",
            "message": format!(
                "missing cockpit context source ({} or {})",
                history_path.display(),
                latest_path.display()
            )
        }));
    } else {
        checks.push(json!({
            "name": "cockpit_source_present",
            "ok": true,
            "history_path": history_path.display().to_string(),
            "latest_path": latest_path.display().to_string()
        }));
    }

    json!({
        "ok": failures.is_empty(),
        "parser_type": "conversation_eye",
        "checks": checks,
        "failures": failures,
        "history_path": history_path.display().to_string(),
        "latest_path": latest_path.display().to_string(),
        "history_exists": history_exists,
        "latest_exists": latest_exists,
    })
}
