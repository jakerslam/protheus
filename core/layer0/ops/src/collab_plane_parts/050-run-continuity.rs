fn run_continuity(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CONTINUITY_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_team_continuity_contract",
            "required_keys": ["team", "agents", "tasks", "handoffs"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_continuity_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_team_continuity_contract"
    {
        errors.push("collab_continuity_contract_kind_invalid".to_string());
    }
    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        30,
    )
    .to_ascii_lowercase();
    if !matches!(op.as_str(), "checkpoint" | "reconstruct" | "status") {
        errors.push("collab_continuity_op_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_continuity",
            "errors": errors
        });
    }

    match op.as_str() {
        "status" => {
            let checkpoint_path = continuity_checkpoint_path(root, &team);
            let reconstruct_path = continuity_reconstruct_path(root, &team);
            let checkpoint = read_json(&checkpoint_path);
            let reconstructed = read_json(&reconstruct_path);
            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "collab_plane_continuity",
                "op": "status",
                "team": team,
                "checkpoint_present": checkpoint.is_some(),
                "reconstructed_present": reconstructed.is_some(),
                "checkpoint_path": checkpoint_path.display().to_string(),
                "reconstruct_path": reconstruct_path.display().to_string(),
                "claim_evidence": [
                    {
                        "id": "V6-COLLAB-001.5",
                        "claim": "team_state_continuity_supports_restart_reconstruction_with_deterministic_audit_receipts",
                        "evidence": {
                            "team": team,
                            "checkpoint_present": checkpoint.is_some(),
                            "reconstructed_present": reconstructed.is_some()
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        "checkpoint" => {
            let mut state = parsed
                .flags
                .get("state-json")
                .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
                .unwrap_or_else(|| default_team_state(&team));
            for key in contract
                .get("required_keys")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
            {
                if !state.get(key).is_some() {
                    state[key] = Value::Null;
                }
            }
            state["checkpoint_ts"] = Value::String(crate::now_iso());
            state["checkpoint_hash"] = Value::String(sha256_hex_str(&state.to_string()));
            let path = continuity_checkpoint_path(root, &team);
            let _ = write_json(&path, &state);
            let _ = append_jsonl(
                &state_root(root).join("continuity").join("history.jsonl"),
                &json!({
                    "type": "collab_checkpoint",
                    "team": team,
                    "path": path.display().to_string(),
                    "ts": crate::now_iso()
                }),
            );
            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "collab_plane_continuity",
                "op": "checkpoint",
                "team": team,
                "checkpoint": state,
                "artifact": {
                    "path": path.display().to_string(),
                    "sha256": sha256_hex_str(&state.to_string())
                },
                "claim_evidence": [
                    {
                        "id": "V6-COLLAB-001.5",
                        "claim": "team_state_continuity_persists_checkpoint_for_recovery_audits",
                        "evidence": {
                            "team": team
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        "reconstruct" => {
            let checkpoint_path = continuity_checkpoint_path(root, &team);
            let Some(checkpoint) = read_json(&checkpoint_path) else {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "collab_plane_continuity",
                    "op": "reconstruct",
                    "errors": [format!("checkpoint_missing:{}", checkpoint_path.display())]
                });
            };
            let mut restored = checkpoint.clone();
            restored["reconstructed_ts"] = Value::String(crate::now_iso());
            restored["daemon_restart_simulated"] = Value::Bool(true);
            restored["reattach_simulated"] = Value::Bool(true);
            let path = continuity_reconstruct_path(root, &team);
            let _ = write_json(&path, &restored);
            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "collab_plane_continuity",
                "op": "reconstruct",
                "team": team,
                "restored": restored,
                "artifact": {
                    "path": path.display().to_string(),
                    "sha256": sha256_hex_str(&restored.to_string())
                },
                "claim_evidence": [
                    {
                        "id": "V6-COLLAB-001.5",
                        "claim": "team_state_reconstruction_restores_auditable_collaboration_state_after_restart",
                        "evidence": {
                            "team": team,
                            "daemon_restart_simulated": true,
                            "reattach_simulated": true
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        _ => json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_continuity",
            "errors": ["collab_continuity_op_invalid"]
        }),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let strict = parse_bool(parsed.flags.get("strict"), true);
    let conduit = if command != "status" {
        Some(conduit_enforcement(root, &parsed, strict, &command))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": "collab_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "dashboard" => run_dashboard(root, &parsed, strict),
        "launch-role" | "launch" => run_launch_role(root, &parsed, strict),
        "terminate-role" | "remove-role" | "revoke-role" | "stop-role" | "archive-role" => {
            run_terminate_role(root, &parsed, strict, command.as_str())
        }
        "schedule" => run_schedule(root, &parsed, strict),
        "throttle" => run_throttle(root, &parsed, strict),
        "continuity" => run_continuity(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "collab_plane_error",
            "error": "unknown_command",
            "command": command
        }),
    };
    if command == "status" {
        print_json(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["dashboard".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "dashboard");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn throttle_persists_plane_policy() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "throttle".to_string(),
            "--team=ops".to_string(),
            "--plane=backlog_delivery_plane".to_string(),
            "--max-depth=75".to_string(),
            "--strategy=priority-shed".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_throttle(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("policy")
                .and_then(|v| v.get("plane"))
                .and_then(Value::as_str),
            Some("backlog_delivery_plane")
        );
        assert_eq!(
            out.get("policy")
                .and_then(|v| v.get("max_depth"))
                .and_then(Value::as_u64),
            Some(75)
        );
    }

    #[test]
    fn terminate_role_removes_shadow_and_is_idempotent() {
        let root = tempfile::tempdir().expect("tempdir");
        let team_path = team_state_path(root.path(), "ops");
        let _ = write_json(
            &team_path,
            &json!({
                "version": "v1",
                "team": "ops",
                "agents": [
                    {"shadow": "ops-a", "role": "analyst", "status": "active", "activated_at": "2026-03-22T00:00:00Z"},
                    {"shadow": "ops-b", "role": "researcher", "status": "active", "activated_at": "2026-03-22T00:00:00Z"}
                ],
                "tasks": [
                    {"id": "task-1", "assignee": "ops-a"},
                    {"id": "task-2", "assignee": "ops-b"}
                ],
                "handoffs": []
            }),
        );

        let parsed = crate::parse_args(&[
            "terminate-role".to_string(),
            "--team=ops".to_string(),
            "--shadow=ops-a".to_string(),
            "--reason=test".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_terminate_role(root.path(), &parsed, true, "terminate-role");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("removed_count").and_then(Value::as_u64), Some(1));
        assert_eq!(
            out.get("released_task_count").and_then(Value::as_u64),
            Some(1)
        );

        let team_state = read_json(&team_path).expect("team state");
        let agents = team_state
            .get("agents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(agents.len(), 1);
        assert_eq!(
            agents[0].get("shadow").and_then(Value::as_str),
            Some("ops-b")
        );

        let out_second = run_terminate_role(root.path(), &parsed, true, "terminate-role");
        assert_eq!(out_second.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out_second.get("removed_count").and_then(Value::as_u64),
            Some(0)
        );
    }

    #[test]
    fn launch_role_auto_spawns_director_and_updates_topology() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "launch-role".to_string(),
            "--team=ops".to_string(),
            "--role=analyst".to_string(),
            "--shadow=ops-a".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_launch_role(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("topology")
                .and_then(|v| v.get("director_count"))
                .and_then(Value::as_u64),
            Some(1)
        );

        let team_state = read_json(&team_state_path(root.path(), "ops")).expect("team state");
        let agents = team_state
            .get("agents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(agents
            .iter()
            .any(|row| row.get("role").and_then(Value::as_str) == Some("director")));
        assert!(agents
            .iter()
            .any(|row| row.get("shadow").and_then(Value::as_str) == Some("ops-a")));
    }

    #[test]
    fn launch_role_enforces_configured_stable_capacity() {
        let root = tempfile::tempdir().expect("tempdir");
        let contract_path = root
            .path()
            .join("planes")
            .join("contracts")
            .join("collab")
            .join("role_launcher_contract_v1.json");
        fs::create_dir_all(contract_path.parent().expect("contract parent"))
            .expect("mkdir contract");
        fs::write(
            &contract_path,
            serde_json::to_string_pretty(&json!({
                "version": "v1",
                "kind": "collab_role_launcher_contract",
                "limits": {
                    "base_max_active_agents": 16,
                    "max_active_agents": 32,
                    "max_agents_per_cell": 8,
                    "director_fanout_cells": 4,
                    "max_directors": 2,
                    "decentralized_min_agents": 4,
                    "auto_director_spawn": false
                },
                "roles": {
                    "analyst": {"default_tools": ["summarize", "score"], "policy_mode": "safe"}
                }
            }))
            .expect("encode contract"),
        )
        .expect("write contract");
        let loaded = load_json_or(root.path(), LAUNCHER_CONTRACT_PATH, json!({}));
        assert_eq!(
            loaded
                .get("limits")
                .and_then(|row| row.get("max_active_agents"))
                .and_then(Value::as_u64),
            Some(32)
        );

        for idx in 0..32 {
            let parsed = crate::parse_args(&[
                "launch-role".to_string(),
                "--team=ops".to_string(),
                "--role=analyst".to_string(),
                format!("--shadow=ops-{idx}"),
                "--strict=1".to_string(),
            ]);
            let out = run_launch_role(root.path(), &parsed, true);
            assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        }

        let overflow = crate::parse_args(&[
            "launch-role".to_string(),
            "--team=ops".to_string(),
            "--role=analyst".to_string(),
            "--shadow=ops-over".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_launch_role(root.path(), &overflow, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert!(out
            .get("errors")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == "collab_role_team_at_capacity"));
    }

    #[test]
    fn terminate_role_prunes_orphaned_director_cells() {
        let root = tempfile::tempdir().expect("tempdir");
        let launch = crate::parse_args(&[
            "launch-role".to_string(),
            "--team=ops".to_string(),
            "--role=researcher".to_string(),
            "--shadow=ops-worker".to_string(),
            "--strict=1".to_string(),
        ]);
        let launched = run_launch_role(root.path(), &launch, true);
        assert_eq!(launched.get("ok").and_then(Value::as_bool), Some(true));

        let terminate = crate::parse_args(&[
            "terminate-role".to_string(),
            "--team=ops".to_string(),
            "--shadow=ops-worker".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_terminate_role(root.path(), &terminate, true, "terminate-role");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("orphaned_director_gc").and_then(Value::as_u64),
            Some(1)
        );

        let team_state = read_json(&team_state_path(root.path(), "ops")).expect("team state");
        let agents = team_state
            .get("agents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            agents.is_empty(),
            "expected worker and director to be fully removed"
        );
    }
}

