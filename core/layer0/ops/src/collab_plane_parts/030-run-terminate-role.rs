fn run_terminate_role(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let contract = load_json_or(
        root,
        TERMINATION_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_role_termination_contract",
            "allowed_actions": ["terminate-role", "remove-role", "revoke-role", "stop-role", "archive-role"],
            "require_shadow": true
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_role_termination_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_role_termination_contract"
    {
        errors.push("collab_role_termination_contract_kind_invalid".to_string());
    }

    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let action_clean = clean(action, 40).to_ascii_lowercase();
    let allowed_actions = contract
        .get("allowed_actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("terminate-role")])
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 40).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if strict && !allowed_actions.iter().any(|row| row == &action_clean) {
        errors.push("collab_role_termination_action_invalid".to_string());
    }

    let shadow = clean(
        parsed
            .flags
            .get("shadow")
            .cloned()
            .or_else(|| parsed.flags.get("agent").cloned())
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        120,
    );
    let require_shadow = contract
        .get("require_shadow")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if strict && require_shadow && shadow.is_empty() {
        errors.push("collab_role_termination_shadow_required".to_string());
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_terminate_role",
            "errors": errors
        });
    }

    let reason = clean(
        parsed
            .flags
            .get("reason")
            .cloned()
            .unwrap_or_else(|| action_clean.clone()),
        120,
    );
    let limits = parse_launcher_limits(&load_json_or(
        root,
        LAUNCHER_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_role_launcher_contract",
            "limits": {
                "base_max_active_agents": BASE_STABLE_MAX_ACTIVE_AGENTS,
                "max_active_agents": DEFAULT_STABLE_MAX_ACTIVE_AGENTS,
                "max_agents_per_cell": DEFAULT_MAX_AGENTS_PER_CELL,
                "director_fanout_cells": DEFAULT_DIRECTOR_FANOUT_CELLS,
                "max_directors": DEFAULT_MAX_DIRECTORS,
                "decentralized_min_agents": DEFAULT_DECENTRALIZED_MIN_AGENTS,
                "auto_director_spawn": true
            }
        }),
    ));
    let team_path = team_state_path(root, &team);
    let mut team_state = read_json(&team_path).unwrap_or_else(|| default_team_state(&team));

    let mut agents = team_state
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let before_agents = agents.len();
    agents.retain(|row| {
        if shadow.is_empty() {
            return true;
        }
        row.get("shadow").and_then(Value::as_str) != Some(shadow.as_str())
    });
    let removed_count = before_agents.saturating_sub(agents.len());
    let orphaned_director_gc = prune_orphan_directors(&mut agents);
    team_state["agents"] = Value::Array(agents.clone());

    let mut tasks = team_state
        .get("tasks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let before_tasks = tasks.len();
    tasks.retain(|row| {
        if shadow.is_empty() {
            return true;
        }
        let assigned = ["shadow", "agent", "assignee", "owner"]
            .iter()
            .any(|key| row.get(key).and_then(Value::as_str) == Some(shadow.as_str()));
        !assigned
    });
    let released_task_count = before_tasks.saturating_sub(tasks.len());
    team_state["tasks"] = Value::Array(tasks.clone());

    let mut pending_queue = team_state
        .get("pending_queue")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let before_pending_queue = pending_queue.len();
    pending_queue.retain(|row| {
        if shadow.is_empty() {
            return true;
        }
        let assigned = ["shadow", "agent", "assignee", "owner", "run_owner"]
            .iter()
            .any(|key| row.get(key).and_then(Value::as_str) == Some(shadow.as_str()));
        !assigned
    });
    let cleared_pending_queue_count = before_pending_queue.saturating_sub(pending_queue.len());
    if before_pending_queue > 0 || cleared_pending_queue_count > 0 {
        team_state["pending_queue"] = Value::Array(pending_queue.clone());
    }

    let mut handoffs = team_state
        .get("handoffs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if removed_count > 0 || released_task_count > 0 {
        handoffs.push(json!({
            "type": "agent_terminated",
            "team": team,
            "shadow": shadow,
            "action": action_clean,
            "reason": reason,
            "terminated_at": crate::now_iso(),
            "removed_count": removed_count,
            "released_task_count": released_task_count,
            "cleared_pending_queue_count": cleared_pending_queue_count,
            "orphaned_director_gc": orphaned_director_gc,
            "termination_hash": sha256_hex_str(&format!("{}:{}:{}:{}", team, shadow, action_clean, reason))
        }));
        if handoffs.len() > DEFAULT_HANDOFF_RETAIN {
            handoffs = handoffs[handoffs.len().saturating_sub(DEFAULT_HANDOFF_RETAIN)..].to_vec();
        }
    }
    team_state["handoffs"] = Value::Array(handoffs.clone());
    refresh_team_topology(&mut team_state, limits);
    let _ = write_json(&team_path, &team_state);

    if removed_count > 0 || released_task_count > 0 {
        let _ = append_jsonl(
            &state_root(root).join("terminate").join("history.jsonl"),
            &json!({
                "type": "collab_role_termination",
                "team": team,
                "shadow": shadow,
                "action": action_clean,
                "reason": reason,
                "removed_count": removed_count,
                "released_task_count": released_task_count,
                "cleared_pending_queue_count": cleared_pending_queue_count,
                "orphaned_director_gc": orphaned_director_gc,
                "ts": crate::now_iso()
            }),
        );
    }

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "collab_plane_terminate_role",
        "lane": "core/layer0/ops",
        "team": team,
        "shadow": shadow,
        "action": action_clean,
        "reason": reason,
        "removed_count": removed_count,
        "orphaned_director_gc": orphaned_director_gc,
        "released_task_count": released_task_count,
        "cleared_pending_queue_count": cleared_pending_queue_count,
        "tool_stream_reset": removed_count > 0,
        "team_state": {
            "agent_count": agents.len(),
            "task_count": tasks.len(),
            "pending_queue_count": pending_queue.len(),
            "handoff_count": handoffs.len(),
            "topology": team_state.get("topology").cloned().unwrap_or_else(|| json!({}))
        },
        "artifact": {
            "path": team_path.display().to_string(),
            "sha256": sha256_hex_str(&team_state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COLLAB-001.2",
                "claim": "role_lifecycle_supports_deterministic_termination_with_receipts_and_team_state_cleanup",
                "evidence": {
                    "team": team,
                    "shadow": shadow,
                    "removed_count": removed_count,
                    "released_task_count": released_task_count,
                    "cleared_pending_queue_count": cleared_pending_queue_count,
                    "orphaned_director_gc": orphaned_director_gc
                }
            },
            {
                "id": "V6-AGENT-LIFECYCLE-001.2",
                "claim": "auto_termination_path_removes_idle_agents_from_authority_state",
                "evidence": {
                    "team": team,
                    "shadow": shadow,
                    "removed_count": removed_count
                }
            },
            {
                "id": "V6-COLLAB-002.3",
                "claim": "decentralized_role_gc_prunes_orphaned_directors_when_worker_cells_empty",
                "evidence": {
                    "team": team,
                    "shadow": shadow,
                    "orphaned_director_gc": orphaned_director_gc
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
