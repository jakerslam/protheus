fn auto_recall_execution_receipt(status: &str, reason: Option<&str>) -> Value {
    let normalized_status = match status.trim().to_ascii_lowercase().as_str() {
        "ok" | "success" | "succeeded" | "ready" => "success",
        "timeout" | "timed_out" | "timed-out" => "timeout",
        "throttled" | "rate_limited" | "rate-limited" | "429" => "throttled",
        _ => "error",
    };
    let normalized_reason = reason.map(|raw| {
        raw.trim()
            .to_ascii_lowercase()
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.') {
                    ch
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .split('_')
            .filter(|row| !row.is_empty())
            .collect::<Vec<_>>()
            .join("_")
    });
    let seed = format!(
        "{}|{}",
        normalized_status,
        normalized_reason.clone().unwrap_or_default()
    );
    json!({
        "call_id": format!("auto-recall-{}", &sha256_hex(&seed)[..16]),
        "status": normalized_status,
        "error_kind": normalized_reason,
        "telemetry": {
            "duration_ms": 0,
            "tokens_used": 0
        }
    })
}

fn with_auto_recall_execution_receipt(
    mut payload: Value,
    status: &str,
    reason: Option<&str>,
) -> Value {
    payload["execution_receipt"] = auto_recall_execution_receipt(status, reason);
    payload
}

fn auto_recall_filed_payload(args: &HashMap<String, String>) -> Value {
    let root = default_workspace_root(args);
    let (matrix_path, policy_path, events_path, latest_path) =
        memory_auto_recall_paths(&root, args);
    let policy = load_auto_recall_policy(&policy_path);

    let node_id = normalize_node_id(
        args.get("node-id")
            .or_else(|| args.get("node_id"))
            .map(String::as_str)
            .unwrap_or(""),
    );
    let tags = parse_tags_arg(args.get("tags").map(String::as_str).unwrap_or(""));
    let dry_run = args
        .get("dry-run")
        .map(|raw| {
            let lower = raw.trim().to_ascii_lowercase();
            lower == "1" || lower == "true" || lower == "yes" || lower == "on"
        })
        .unwrap_or_else(|| {
            policy
                .get("dry_run")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        });

    if node_id.is_empty() || tags.is_empty() {
        let out = with_auto_recall_execution_receipt(json!({
            "ok": false,
            "type": "memory_auto_recall",
            "reason": "missing_node_or_tags",
            "node_id": if node_id.is_empty() { Value::Null } else { Value::String(node_id.clone()) },
            "tags": tags,
            "ts": now_iso()
        }), "error", Some("missing_node_or_tags"));
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    }

    if !policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        let out = with_auto_recall_execution_receipt(json!({
            "ok": true,
            "type": "memory_auto_recall",
            "skipped": true,
            "reason": "disabled",
            "node_id": node_id,
            "tags": tags,
            "ts": now_iso()
        }), "success", Some("disabled"));
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    }

    let Some(matrix) = read_json(&matrix_path) else {
        let out = with_auto_recall_execution_receipt(json!({
            "ok": false,
            "type": "memory_auto_recall",
            "reason": "matrix_unavailable",
            "node_id": node_id,
            "tags": tags,
            "ts": now_iso()
        }), "error", Some("matrix_unavailable"));
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    };

    let max_matrix_age_ms = policy
        .get("max_matrix_age_ms")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_INDEX_MAX_AGE_MS);
    let allow_stale_matrix = parse_bool_value(
        args.get("allow-stale-matrix")
            .or_else(|| args.get("allow_stale_matrix"))
            .map(String::as_str)
            .unwrap_or(""),
        false,
    );
    let matrix_generated_ms = matrix
        .get("generated_at")
        .and_then(Value::as_str)
        .and_then(iso_to_epoch_ms)
        .or_else(|| file_mtime_ms(&matrix_path));
    let freshness = enforce_index_freshness(
        now_epoch_ms(),
        matrix_generated_ms,
        max_matrix_age_ms,
        allow_stale_matrix,
    );
    if !freshness.ok {
        let out = with_auto_recall_execution_receipt(json!({
            "ok": false,
            "type": "memory_auto_recall",
            "reason": freshness.reason_code,
            "stale": freshness.stale,
            "age_ms": freshness.age_ms,
            "threshold_ms": freshness.threshold_ms,
            "node_id": node_id,
            "tags": tags,
            "matrix_path": matrix_path.to_string_lossy().to_string(),
            "ts": now_iso()
        }), "error", Some(freshness.reason_code));
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    }

    let min_shared = policy
        .get("min_shared_tags")
        .and_then(Value::as_u64)
        .unwrap_or(1) as usize;
    let max_matches = policy
        .get("max_matches")
        .and_then(Value::as_u64)
        .unwrap_or(3) as usize;
    let min_priority = policy
        .get("min_priority_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    let mut candidates: BTreeMap<String, Value> = BTreeMap::new();
    if let Some(tag_rows) = matrix.get("tags").and_then(Value::as_array) {
        for source_tag in &tags {
            let Some(tag_row) = tag_rows
                .iter()
                .find(|row| row.get("tag").and_then(Value::as_str).unwrap_or("") == source_tag)
            else {
                continue;
            };
            let nodes = tag_row
                .get("nodes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();

            for node in nodes {
                let candidate_id =
                    normalize_node_id(node.get("node_id").and_then(Value::as_str).unwrap_or(""));
                if candidate_id.is_empty() || candidate_id == node_id {
                    continue;
                }
                let candidate_tags = node
                    .get("tags")
                    .and_then(Value::as_array)
                    .map(|rows| {
                        rows.iter()
                            .filter_map(Value::as_str)
                            .map(normalize_tag)
                            .filter(|tag| !tag.is_empty())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default();
                let shared = intersect_count(&tags, &candidate_tags);
                if shared < min_shared {
                    continue;
                }
                let priority = node
                    .get("priority_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                if priority < min_priority {
                    continue;
                }
                let recency = node
                    .get("recency_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let dream = node
                    .get("dream_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let score =
                    (shared as f64 * 50.0) + (priority * 0.85) + (dream * 12.0) + (recency * 8.0);
                let shared_tags = tags
                    .iter()
                    .filter(|tag| candidate_tags.contains(tag))
                    .cloned()
                    .collect::<Vec<String>>();

                let next = json!({
                    "node_id": candidate_id,
                    "file": node.get("file").cloned().unwrap_or(Value::Null),
                    "date": node.get("date").cloned().unwrap_or(Value::Null),
                    "summary": node.get("summary").cloned().unwrap_or(Value::Null),
                    "level_token": node.get("level_token").cloned().unwrap_or(Value::Null),
                    "priority_score": priority,
                    "score": score,
                    "shared_tags": shared_tags
                });

                let replace = candidates
                    .get(&candidate_id)
                    .and_then(|cur| cur.get("score").and_then(Value::as_f64))
                    .map(|current| score > current)
                    .unwrap_or(true);
                if replace {
                    candidates.insert(candidate_id, next);
                }
            }
        }
    }

    let mut matches = candidates.into_values().collect::<Vec<Value>>();
    matches.sort_by(|a, b| {
        let sb = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let sa = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        sb.total_cmp(&sa).then_with(|| {
            let an = a.get("node_id").and_then(Value::as_str).unwrap_or("");
            let bn = b.get("node_id").and_then(Value::as_str).unwrap_or("");
            an.cmp(bn)
        })
    });
    matches.truncate(max_matches.max(1));

    let ranking_scores = matches
        .iter()
        .map(|row| row.get("score").and_then(Value::as_f64).unwrap_or(0.0))
        .collect::<Vec<f64>>();
    let ranking_ids = matches
        .iter()
        .map(|row| normalize_node_id(row.get("node_id").and_then(Value::as_str).unwrap_or("")))
        .collect::<Vec<String>>();
    let ranking = enforce_descending_ranking(&ranking_scores, &ranking_ids);
    if !ranking.ok {
        let out = with_auto_recall_execution_receipt(json!({
            "ok": false,
            "type": "memory_auto_recall",
            "reason": ranking.reason_code,
            "node_id": node_id,
            "tags": tags,
            "ts": now_iso()
        }), "error", Some(ranking.reason_code));
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    }

    if matches.is_empty() {
        let out = with_auto_recall_execution_receipt(json!({
            "ok": true,
            "type": "memory_auto_recall",
            "skipped": true,
            "reason": "no_matches",
            "node_id": node_id,
            "tags": tags,
            "ts": now_iso()
        }), "success", Some("no_matches"));
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    }

    let attention = if dry_run {
        json!({
            "ok": true,
            "skipped": true,
            "reason": "dry_run_or_queue_disabled",
            "queued": false,
            "routed_via": "none"
        })
    } else {
        json!({
            "ok": true,
            "skipped": true,
            "reason": "queue_not_wired_in_wave1",
            "queued": false,
            "routed_via": "none"
        })
    };

    let out = with_auto_recall_execution_receipt(json!({
        "ok": true,
        "type": "memory_auto_recall",
        "ts": now_iso(),
        "node_id": node_id,
        "tags": tags,
        "matches": matches,
        "match_count": matches.len(),
        "dry_run": dry_run,
        "matrix_path": matrix_path.to_string_lossy().to_string(),
        "freshness": {
            "ok": freshness.ok,
            "stale": freshness.stale,
            "reason_code": freshness.reason_code,
            "age_ms": freshness.age_ms,
            "threshold_ms": freshness.threshold_ms
        },
        "ranking_invariants": {
            "ok": ranking.ok,
            "reason_code": ranking.reason_code
        },
        "attention": attention
    }), "success", None);
    append_jsonl(&events_path, &out);
    write_json(&latest_path, &out);
    out
}

fn auto_recall_status_payload(args: &HashMap<String, String>) -> Value {
    let root = default_workspace_root(args);
    let (matrix_path, policy_path, events_path, latest_path) =
        memory_auto_recall_paths(&root, args);
    let latest = read_json(&latest_path).unwrap_or(Value::Null);
    json!({
        "ok": true,
        "type": "memory_auto_recall_status",
        "policy": load_auto_recall_policy(&policy_path),
        "latest": latest,
        "paths": {
            "events": events_path.to_string_lossy().to_string(),
            "latest": latest_path.to_string_lossy().to_string(),
            "matrix": matrix_path.to_string_lossy().to_string()
        }
    })
}

fn dream_paths(root: &Path, args: &HashMap<String, String>) -> (PathBuf, PathBuf, PathBuf) {
    let matrix_path = args
        .get("matrix-path")
        .cloned()
        .or_else(|| env::var("MEMORY_MATRIX_JSON_PATH").ok())
        .unwrap_or_else(|| {
            "client/runtime/local/state/memory/matrix/tag_memory_matrix.json".to_string()
        });
    let state_path = args
        .get("state-path")
        .cloned()
        .or_else(|| env::var("DREAM_SEQUENCER_STATE_PATH").ok())
        .unwrap_or_else(|| {
            "client/runtime/local/state/memory/dream_sequencer/latest.json".to_string()
        });
    let ledger_path = args
        .get("ledger-path")
        .cloned()
        .or_else(|| env::var("DREAM_SEQUENCER_LEDGER_PATH").ok())
        .unwrap_or_else(|| {
            "client/runtime/local/state/memory/dream_sequencer/runs.jsonl".to_string()
        });

    (
        resolve_path(root, &matrix_path),
        resolve_path(root, &state_path),
        resolve_path(root, &ledger_path),
    )
}

fn dream_run_payload(args: &HashMap<String, String>) -> Value {
    let root = default_workspace_root(args);
    let (matrix_path, state_path, ledger_path) = dream_paths(&root, args);
    let reason = clean_text(
        args.get("reason").map(String::as_str).unwrap_or("manual"),
        120,
    );
    let top_tags = args
        .get("top-tags")
        .or_else(|| args.get("top_tags"))
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(12)
        .clamp(1, 64);
    let apply = args
        .get("apply")
        .map(|raw| {
            let lower = raw.trim().to_ascii_lowercase();
            !(lower == "0" || lower == "false" || lower == "off" || lower == "no")
        })
        .unwrap_or(true);

    let matrix = read_json(&matrix_path).or_else(|| {
        let out = build_matrix_payload(&HashMap::from([
            ("root".to_string(), root.to_string_lossy().to_string()),
            ("apply".to_string(), "true".to_string()),
            ("reason".to_string(), "dream_sequencer_refresh".to_string()),
            (
                "matrix-json-path".to_string(),
                matrix_path.to_string_lossy().to_string(),
            ),
        ]));
        if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            Some(out)
        } else {
            None
        }
    });

    let Some(matrix_payload) = matrix else {
        let fail = json!({
            "ok": false,
            "type": "dream_sequencer",
            "reason": "matrix_unavailable",
            "ts": now_iso()
        });
        append_jsonl(&ledger_path, &fail);
        return fail;
    };

    let top = matrix_payload
        .get("tags")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .take(top_tags)
                .map(|row| {
                    json!({
                        "tag": row.get("tag").cloned().unwrap_or(Value::Null),
                        "tag_priority": row.get("tag_priority").cloned().unwrap_or(Value::from(0.0)),
                        "node_count": row.get("node_count").cloned().unwrap_or(Value::from(0)),
                        "top_nodes": row
                            .get("node_ids")
                            .and_then(Value::as_array)
                            .map(|ids| ids.iter().take(5).cloned().collect::<Vec<Value>>())
                            .unwrap_or_default()
                    })
                })
                .collect::<Vec<Value>>()
        })
        .unwrap_or_default();

    let out = json!({
        "ok": true,
        "type": "dream_sequencer",
        "ts": now_iso(),
        "reason": reason,
        "applied": apply,
        "matrix_path": matrix_path.to_string_lossy().to_string(),
        "stats": matrix_payload.get("stats").cloned().unwrap_or(Value::Null),
        "top_tags": top
    });

    if apply {
        write_json(&state_path, &out);
    }
    append_jsonl(&ledger_path, &out);
    out
}

fn dream_status_payload(args: &HashMap<String, String>) -> Value {
    let root = default_workspace_root(args);
    let (matrix_path, state_path, ledger_path) = dream_paths(&root, args);
    json!({
        "ok": true,
        "type": "dream_sequencer_status",
        "latest": read_json(&state_path).unwrap_or(Value::Null),
        "matrix": if matrix_path.exists() {
            json!({"ok": true, "exists": true, "path": matrix_path.to_string_lossy().to_string()})
        } else {
            json!({"ok": false, "exists": false, "path": matrix_path.to_string_lossy().to_string()})
        },
        "sequencer_state_path": state_path.to_string_lossy().to_string(),
        "sequencer_ledger_path": ledger_path.to_string_lossy().to_string()
    })
}
