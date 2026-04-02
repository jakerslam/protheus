// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::path::Path;

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
        "  protheus-ops conversation-eye-collector-kernel begin-collection --payload-base64=<json>"
    );
    println!("  protheus-ops conversation-eye-collector-kernel preflight --payload-base64=<json>");
    println!(
        "  protheus-ops conversation-eye-collector-kernel load-source-rows --payload-base64=<json>"
    );
    println!(
        "  protheus-ops conversation-eye-collector-kernel normalize-topics --payload-base64=<json>"
    );
    println!("  protheus-ops conversation-eye-collector-kernel load-index --payload-base64=<json>");
    println!("  protheus-ops conversation-eye-collector-kernel apply-node --payload-base64=<json>");
    println!(
        "  protheus-ops conversation-eye-collector-kernel process-nodes --payload-base64=<json>"
    );
    println!(
        "  protheus-ops conversation-eye-collector-kernel append-memory-row --payload-base64=<json>"
    );
    println!(
        "  protheus-ops conversation-eye-collector-kernel append-memory-rows --payload-base64=<json>"
    );
    println!("  protheus-ops conversation-eye-collector-kernel save-index --payload-base64=<json>");
}

fn nested_obj<'a>(payload: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
    payload.get(key).and_then(Value::as_object)
}

fn nested_u64(payload: &Map<String, Value>, parent: &str, key: &str) -> Option<u64> {
    nested_obj(payload, parent)
        .and_then(|obj| obj.get(key))
        .and_then(Value::as_u64)
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
    let memory_jsonl_path = resolve_path(
        root,
        payload,
        "memory_jsonl_path",
        &format!("{}/nodes.jsonl", DEFAULT_MEMORY_DIR_REL),
    );
    let index_path = resolve_path(
        root,
        payload,
        "index_path",
        &format!("{}/index.json", DEFAULT_MEMORY_DIR_REL),
    );

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

fn command_load_source_rows(root: &Path, payload: &Map<String, Value>) -> Value {
    let history_path = resolve_path(root, payload, "history_path", DEFAULT_HISTORY_REL);
    let latest_path = resolve_path(root, payload, "latest_path", DEFAULT_LATEST_REL);
    let max_rows = clamp_u64(payload, "max_rows", 24, 1, 512) as usize;

    let history_rows = read_jsonl_tail(&history_path, max_rows);
    if !history_rows.is_empty() {
        return json!({
            "ok": true,
            "rows": history_rows,
            "source": "history"
        });
    }

    let latest = read_json(&latest_path, Value::Null);
    if latest.is_object() {
        return json!({
            "ok": true,
            "rows": [latest],
            "source": "latest"
        });
    }

    json!({
        "ok": true,
        "rows": [],
        "source": "none"
    })
}

fn command_normalize_topics(payload: &Map<String, Value>) -> Value {
    json!({
        "ok": true,
        "topics": normalize_topics(payload)
    })
}

fn command_load_index(root: &Path, payload: &Map<String, Value>) -> Value {
    let index_path = resolve_path(
        root,
        payload,
        "index_path",
        &format!("{}/index.json", DEFAULT_MEMORY_DIR_REL),
    );
    let index = normalize_index(Some(&read_json(&index_path, Value::Object(Map::new()))));

    json!({
        "ok": true,
        "index_path": index_path.display().to_string(),
        "index": index
    })
}

fn command_apply_node(payload: &Map<String, Value>) -> Value {
    let mut index = normalize_index(payload.get("index"));
    let index_obj = lane_utils::payload_obj(&index).clone();

    let node = match payload.get("node").and_then(Value::as_object) {
        Some(v) => v,
        None => {
            return json!({
                "ok": true,
                "allowed": false,
                "reason": "invalid_node",
                "quota_skipped": false,
                "index": index
            });
        }
    };

    let node_id = clean_text(node.get("node_id").and_then(Value::as_str), 120);
    if node_id.is_empty() {
        return json!({
            "ok": true,
            "allowed": false,
            "reason": "missing_node_id",
            "quota_skipped": false,
            "index": index
        });
    }

    let now = clean_text(payload.get("now_ts").and_then(Value::as_str), 80);
    let now = if now.is_empty() { now_iso() } else { now };
    let week_key = to_iso_week(clean_text(node.get("ts").and_then(Value::as_str), 80).as_str());
    let level = node
        .get("level")
        .and_then(Value::as_u64)
        .map(|n| n.clamp(1, 3))
        .unwrap_or(3);
    let weekly_limit = clamp_u64(payload, "weekly_node_limit", 10, 1, 50);
    let promotion_overrides = clamp_u64(payload, "weekly_promotion_overrides", 2, 0, 20);

    let emitted = index_obj
        .get("emitted_node_ids")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if emitted.contains_key(&node_id) {
        return json!({
            "ok": true,
            "allowed": false,
            "reason": "duplicate",
            "quota_skipped": false,
            "index": index
        });
    }

    let mut weekly_counts = index_obj
        .get("weekly_counts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut weekly_promotions = index_obj
        .get("weekly_promotions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let count = weekly_counts
        .get(&week_key)
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let promotions = weekly_promotions
        .get(&week_key)
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let promoted = count >= weekly_limit && level == 1 && promotions < promotion_overrides;
    let allowed = count < weekly_limit || promoted;
    if !allowed {
        return json!({
            "ok": true,
            "allowed": false,
            "reason": "quota",
            "quota_skipped": true,
            "week_key": week_key,
            "index": index,
        });
    }

    let mut emitted_new = emitted;
    emitted_new.insert(node_id.clone(), Value::String(now.clone()));
    weekly_counts.insert(week_key.clone(), Value::Number((count + 1).into()));
    if promoted {
        weekly_promotions.insert(week_key.clone(), Value::Number((promotions + 1).into()));
    }

    index = normalize_index(Some(&json!({
        "updated_ts": now,
        "emitted_node_ids": emitted_new,
        "weekly_counts": weekly_counts,
        "weekly_promotions": weekly_promotions,
    })));

    let tags = clean_tags(node.get("node_tags"));
    let edges_to = clean_edges(node.get("edges_to"));
    let title = clean_text(node.get("title").and_then(Value::as_str), 180);
    let title = if title.is_empty() {
        "[Conversation Eye] synthesized signal".to_string()
    } else {
        title
    };
    let preview = clean_text(node.get("preview").and_then(Value::as_str), 240);
    let preview = if preview.is_empty() {
        "conversation_eye synthesized runtime node".to_string()
    } else {
        preview
    };
    let date = clean_text(node.get("date").and_then(Value::as_str), 20);
    let date = if date.is_empty() {
        now_iso()[..10].to_string()
    } else {
        date
    };

    let recall_matches = payload
        .get("recall")
        .and_then(Value::as_object)
        .and_then(|rec| rec.get("matches"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_object().cloned())
        .take(3)
        .map(|row| {
            json!({
                "node_id": clean_text(row.get("node_id").and_then(Value::as_str), 120),
                "score": row.get("score").and_then(Value::as_f64).unwrap_or(0.0),
                "shared_tags": row
                    .get("shared_tags")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|tag| tag.as_str().map(|raw| clean_text(Some(raw), 64)))
                    .filter(|tag| !tag.is_empty())
                    .take(8)
                    .map(Value::String)
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();

    let recall_queued = payload
        .get("recall")
        .and_then(Value::as_object)
        .and_then(|rec| rec.get("attention"))
        .and_then(Value::as_object)
        .and_then(|attn| attn.get("queued"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let memory_row = json!({
        "ts": now_iso(),
        "source": "conversation_eye",
        "node_id": node_id,
        "hex_id": clean_text(node.get("hex_id").and_then(Value::as_str), 24),
        "node_kind": clean_text(node.get("node_kind").and_then(Value::as_str), 32),
        "level": level,
        "level_token": clean_text(node.get("level_token").and_then(Value::as_str), 16),
        "tags": tags,
        "edges_to": edges_to,
        "title": title,
        "preview": preview,
        "xml": clean_text(node.get("xml").and_then(Value::as_str), 1600),
    });

    let collect_item = json!({
        "collected_at": now_iso(),
        "id": sha16(&format!("{}|{}", clean_text(Some(&node_id), 80), title)),
        "url": format!("https://local.workspace/conversation/{}/{}", date, clean_text(Some(&node_id), 80)),
        "title": title,
        "content_preview": preview,
        "topics": payload.get("topics").cloned().unwrap_or_else(|| Value::Array(normalize_topics(payload))),
        "node_id": clean_text(Some(&node_id), 80),
        "node_hex_id": clean_text(node.get("hex_id").and_then(Value::as_str), 24),
        "node_kind": clean_text(node.get("node_kind").and_then(Value::as_str), 32),
        "node_level": level,
        "node_level_token": clean_text(node.get("level_token").and_then(Value::as_str), 16),
        "node_tags": memory_row.get("tags").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "edges_to": memory_row.get("edges_to").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "recall_matches": recall_matches,
        "recall_queued": recall_queued,
        "bytes": std::cmp::min(8192_usize, title.len() + preview.len() + 160)
    });

    json!({
        "ok": true,
        "allowed": true,
        "promoted": promoted,
        "quota_skipped": false,
        "week_key": week_key,
        "index": index,
        "memory_row": memory_row,
        "collect_item": collect_item,
    })
}

fn command_process_nodes(payload: &Map<String, Value>) -> Value {
    let mut index = normalize_index(payload.get("index"));
    let topics = payload
        .get("topics")
        .cloned()
        .unwrap_or_else(|| Value::Array(normalize_topics(payload)));
    let weekly_node_limit = clamp_u64(payload, "weekly_node_limit", 10, 1, 50);
    let weekly_promotion_overrides = clamp_u64(payload, "weekly_promotion_overrides", 2, 0, 20);
    let max_items = clamp_u64(payload, "max_items", 3, 1, 64) as usize;
    let now_ts = clean_text(payload.get("now_ts").and_then(Value::as_str), 80);
    let now_ts = if now_ts.is_empty() { now_iso() } else { now_ts };
    let candidates = payload
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut memory_rows = Vec::<Value>::new();
    let mut items = Vec::<Value>::new();
    let mut node_writes = 0_u64;
    let mut recall_queued = 0_u64;
    let mut recall_matches = 0_u64;
    let mut quota_skipped = 0_u64;

    for candidate in candidates {
        if items.len() >= max_items {
            break;
        }
        let candidate_obj = match candidate.as_object() {
            Some(v) => v,
            None => continue,
        };
        let node = match candidate_obj.get("node") {
            Some(v) if v.is_object() => v.clone(),
            _ => continue,
        };

        let mut apply_payload = Map::new();
        apply_payload.insert("index".to_string(), index.clone());
        apply_payload.insert("node".to_string(), node);
        apply_payload.insert("topics".to_string(), topics.clone());
        apply_payload.insert(
            "weekly_node_limit".to_string(),
            Value::Number(weekly_node_limit.into()),
        );
        apply_payload.insert(
            "weekly_promotion_overrides".to_string(),
            Value::Number(weekly_promotion_overrides.into()),
        );
        apply_payload.insert("now_ts".to_string(), Value::String(now_ts.clone()));
        if let Some(recall) = candidate_obj.get("recall") {
            apply_payload.insert("recall".to_string(), recall.clone());
        }

        let applied = command_apply_node(&apply_payload);
        index = normalize_index(applied.get("index"));

        if applied
            .get("allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            if let Some(row) = applied.get("memory_row") {
                memory_rows.push(row.clone());
                node_writes = node_writes.saturating_add(1);
            }
            if let Some(item) = applied.get("collect_item") {
                items.push(item.clone());
            }

            if let Some(recall_obj) = candidate_obj.get("recall").and_then(Value::as_object) {
                let matches_len = recall_obj
                    .get("matches")
                    .and_then(Value::as_array)
                    .map(|rows| rows.len() as u64)
                    .unwrap_or(0);
                recall_matches = recall_matches.saturating_add(matches_len);
                if recall_obj
                    .get("attention")
                    .and_then(Value::as_object)
                    .and_then(|obj| obj.get("queued"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    recall_queued = recall_queued.saturating_add(1);
                }
            }
        } else if applied
            .get("quota_skipped")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            quota_skipped = quota_skipped.saturating_add(1);
        }
    }

    json!({
        "ok": true,
        "index": index,
        "memory_rows": memory_rows,
        "items": items,
        "node_writes": node_writes,
        "recall_queued": recall_queued,
        "recall_matches": recall_matches,
        "quota_skipped": quota_skipped,
    })
}

fn command_append_memory_row(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let jsonl_path = resolve_path(
        root,
        payload,
        "jsonl_path",
        &format!("{}/nodes.jsonl", DEFAULT_MEMORY_DIR_REL),
    );
    let row = payload
        .get("row")
        .cloned()
        .unwrap_or(Value::Object(Map::new()));
    append_jsonl(&jsonl_path, &row)?;
    Ok(json!({
        "ok": true,
        "jsonl_path": jsonl_path.display().to_string()
    }))
}

fn command_append_memory_rows(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let jsonl_path = resolve_path(
        root,
        payload,
        "jsonl_path",
        &format!("{}/nodes.jsonl", DEFAULT_MEMORY_DIR_REL),
    );
    let rows = payload
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut appended = 0_u64;
    for row in rows {
        append_jsonl(&jsonl_path, &row)?;
        appended = appended.saturating_add(1);
    }
    Ok(json!({
        "ok": true,
        "jsonl_path": jsonl_path.display().to_string(),
        "appended": appended
    }))
}

fn command_save_index(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let index_path = resolve_path(
        root,
        payload,
        "index_path",
        &format!("{}/index.json", DEFAULT_MEMORY_DIR_REL),
    );
    let index = normalize_index(payload.get("index"));
    write_json_atomic(&index_path, &index)?;
    Ok(json!({
        "ok": true,
        "index_path": index_path.display().to_string(),
        "index": index
    }))
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "begin-collection" => Ok(command_begin_collection(root, payload)),
        "preflight" => Ok(command_preflight(root, payload)),
        "load-source-rows" => Ok(command_load_source_rows(root, payload)),
        "normalize-topics" => Ok(command_normalize_topics(payload)),
        "load-index" => Ok(command_load_index(root, payload)),
        "apply-node" => Ok(command_apply_node(payload)),
        "process-nodes" => Ok(command_process_nodes(payload)),
        "append-memory-row" => command_append_memory_row(root, payload),
        "append-memory-rows" => command_append_memory_rows(root, payload),
        "save-index" => command_save_index(root, payload),
        _ => Err("conversation_eye_collector_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "conversation_eye_collector_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "conversation_eye_collector_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);

    match dispatch(root, &command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                "conversation_eye_collector_kernel",
                out,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "conversation_eye_collector_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn normalize_topics_includes_defaults() {
        let payload = json!({"topics": ["alpha", "decision"]});
        let out = command_normalize_topics(lane_utils::payload_obj(&payload));
        let topics = out
            .get("topics")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(topics
            .iter()
            .any(|row| row.as_str() == Some("conversation")));
        assert!(topics.iter().any(|row| row.as_str() == Some("alpha")));
    }

    #[test]
    fn apply_node_rejects_duplicate() {
        let payload = json!({
            "index": { "emitted_node_ids": {"abc": "2026-01-01T00:00:00Z"} },
            "node": {"node_id": "abc", "ts": "2026-01-01T00:00:00Z", "level": 3}
        });
        let out = command_apply_node(lane_utils::payload_obj(&payload));
        assert_eq!(out.get("allowed").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("reason").and_then(Value::as_str), Some("duplicate"));
    }

    #[test]
    fn process_nodes_batches_and_limits_items() {
        let payload = json!({
            "index": { "emitted_node_ids": {} },
            "topics": ["conversation", "decision"],
            "max_items": 1,
            "candidates": [
                {
                    "node": { "node_id": "n1", "title": "one", "preview":"p1", "ts":"2026-03-27T00:00:00Z", "level": 3 },
                    "recall": { "matches":[{"node_id":"x"}], "attention": {"queued": true} }
                },
                {
                    "node": { "node_id": "n2", "title": "two", "preview":"p2", "ts":"2026-03-27T00:00:01Z", "level": 3 },
                    "recall": { "matches":[{"node_id":"y"}], "attention": {"queued": true} }
                }
            ]
        });
        let out = command_process_nodes(lane_utils::payload_obj(&payload));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(out.get("node_writes").and_then(Value::as_u64), Some(1));
    }

    #[test]
    fn begin_collection_hydrates_runtime_payload() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        let history_path = root.join(DEFAULT_HISTORY_REL);
        if let Some(parent) = history_path.parent() {
            fs::create_dir_all(parent).expect("mkdir history parent");
        }
        fs::write(
            &history_path,
            "{\"id\":\"row1\",\"text\":\"hello from history\"}\n",
        )
        .expect("write history jsonl");

        let out = command_begin_collection(
            root,
            lane_utils::payload_obj(&json!({
                "budgets": { "max_items": 3 }
            })),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("success").and_then(Value::as_bool), Some(true));
        assert!(out
            .get("source_rows")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("topics")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|r| r.as_str() == Some("conversation")))
            .unwrap_or(false));
    }
}
