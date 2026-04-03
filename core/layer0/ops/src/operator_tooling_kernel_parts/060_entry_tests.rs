pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = crate::parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let payload = payload_from_parsed(&parsed);
    let control_runtime_root = control_runtime_root(root, &parsed);

    let result = match command.as_str() {
        "status" => Ok(summary_status(&control_runtime_root, &parsed)),
        "route-model" => {
            let policy_path = routing_policy_path(&control_runtime_root, &parsed);
            let policy = read_json_file(&policy_path).unwrap_or_else(|| json!({}));
            Ok(run_route_model(&policy, &payload, &policy_path))
        }
        "escalate-model" => {
            let policy_path = routing_policy_path(&control_runtime_root, &parsed);
            let policy = read_json_file(&policy_path).unwrap_or_else(|| json!({}));
            Ok(run_escalate_model(&policy, &payload, &policy_path))
        }
        "plan-auto" | "plan-first" => run_plan_auto(&payload),
        "plan-validate" => run_plan_validate(&payload),
        "postflight-validate" | "postflight-check" => run_postflight_validate(&payload),
        "output-validate" | "output-check" => run_output_validate(&payload),
        "state-read" | "read-state" => {
            let path = state_path(&control_runtime_root, &parsed);
            let state = read_json_file(&path).unwrap_or_else(|| json!({}));
            let key_from_flag = parsed.flags.get("key").map(String::as_str);
            let key_from_positional = parsed.positional.get(1).map(String::as_str);
            run_state_read(&state, key_from_flag.or(key_from_positional), &path)
        }
        "state-write" | "write-state" => {
            let path = state_path(&control_runtime_root, &parsed);
            let mut state = read_json_file(&path).unwrap_or_else(|| json!({}));
            run_state_write(&mut state, &payload, &path)
        }
        "decision-log-append" | "append-decision" => {
            let title = parsed
                .flags
                .get("title")
                .map(|v| clean_text(v, 180))
                .or_else(|| {
                    payload
                        .get("title")
                        .and_then(Value::as_str)
                        .map(|v| clean_text(v, 180))
                })
                .unwrap_or_else(|| "Decision".to_string());
            let reason = parsed
                .flags
                .get("reason")
                .map(|v| clean_text(v, 260))
                .or_else(|| {
                    payload
                        .get("reason")
                        .and_then(Value::as_str)
                        .map(|v| clean_text(v, 260))
                })
                .unwrap_or_default();
            let verify = parsed
                .flags
                .get("verify")
                .map(|v| clean_text(v, 260))
                .or_else(|| {
                    payload
                        .get("verify")
                        .and_then(Value::as_str)
                        .map(|v| clean_text(v, 260))
                })
                .unwrap_or_default();
            let rollback = parsed
                .flags
                .get("rollback")
                .map(|v| clean_text(v, 260))
                .or_else(|| {
                    payload
                        .get("rollback")
                        .and_then(Value::as_str)
                        .map(|v| clean_text(v, 260))
                })
                .unwrap_or_default();
            let details = details_from_flag_or_payload(&parsed, &payload);
            let path = decision_log_path(&control_runtime_root, &parsed);
            match append_decision_markdown(&path, &title, &reason, &verify, &rollback, &details) {
                Ok(()) => Ok(with_receipt(json!({
                    "ok": true,
                    "type": "operator_tooling_decision_log_append",
                    "decision_log_path": path.to_string_lossy().to_string(),
                    "title": title
                }))),
                Err(err) => Err(err),
            }
        }
        "safe-apply" => run_safe_apply(&control_runtime_root, &parsed, &payload),
        "memory-search" => {
            let query = parsed
                .flags
                .get("query")
                .map(|v| clean_text(v, 240))
                .or_else(|| parsed.positional.get(1).map(|v| clean_text(v, 240)))
                .unwrap_or_default();
            if query.is_empty() {
                Err("query_required".to_string())
            } else {
                let limit = parse_usize_flag(&parsed.flags, "limit", 200, 1, 2000);
                Ok(run_memory_search(&control_runtime_root, &query, limit))
            }
        }
        "memory-summarize" => {
            let query = parsed
                .flags
                .get("query")
                .map(|v| clean_text(v, 240))
                .or_else(|| parsed.positional.get(1).map(|v| clean_text(v, 240)))
                .unwrap_or_default();
            if query.is_empty() {
                Err("query_required".to_string())
            } else {
                let limit = parse_usize_flag(&parsed.flags, "limit", 120, 1, 2000);
                Ok(run_memory_summarize(&control_runtime_root, &query, limit))
            }
        }
        "memory-last-change" => {
            let limit = parse_usize_flag(&parsed.flags, "limit", 25, 1, 500);
            Ok(run_memory_last_change(&control_runtime_root, limit))
        }
        "membrief" | "memory-brief" => {
            let query = parsed
                .flags
                .get("query")
                .map(|v| clean_text(v, 240))
                .or_else(|| parsed.positional.get(1).map(|v| clean_text(v, 240)))
                .unwrap_or_default();
            if query.is_empty() {
                Err("query_required".to_string())
            } else {
                let limit = parse_usize_flag(&parsed.flags, "limit", 120, 1, 2000);
                Ok(run_membrief(&control_runtime_root, &query, limit))
            }
        }
        "trace-find" => {
            let trace_id = parsed
                .flags
                .get("trace-id")
                .map(|v| clean_text(v, 160))
                .or_else(|| parsed.positional.get(1).map(|v| clean_text(v, 160)))
                .unwrap_or_default();
            if trace_id.is_empty() {
                Err("trace_id_required".to_string())
            } else {
                let limit = parse_usize_flag(&parsed.flags, "limit", 10, 1, 200);
                Ok(run_trace_find(&control_runtime_root, &trace_id, limit))
            }
        }
        "sync-allowed-models" | "sync-allowlist" => {
            run_sync_allowed_models(&control_runtime_root, &parsed)
        }
        "smoke-routing" => Ok(run_smoke_routing(&control_runtime_root, &parsed)),
        "spawn-safe" => {
            let require_plan = bool_flag(&parsed.flags, "require-plan", false);
            let strict_plan = bool_flag(&parsed.flags, "strict-plan", false);
            build_spawn_packet(&control_runtime_root, &parsed, &payload, require_plan, strict_plan)
        }
        "smart-spawn" => run_smart_spawn(&control_runtime_root, &parsed, &payload),
        "auto-spawn" => run_auto_spawn(&control_runtime_root, &parsed, &payload),
        "execute-handoff" => run_execute_handoff(&control_runtime_root, &parsed, &payload),
        "safe-run" | "control_runtime-safe" | "watch-exec" => run_safe_run(root, &control_runtime_root, &parsed),
        "control_runtime-health" | "safe-health" | "health" => {
            Ok(run_control_runtime_health(&control_runtime_root, &parsed))
        }
        "cron-drift" => {
            let workspace_root = workspace_root(root, &parsed);
            Ok(run_cron_drift(&control_runtime_root, &workspace_root))
        }
        "cron-sync" => {
            let workspace_root = workspace_root(root, &parsed);
            run_cron_sync(&control_runtime_root, &workspace_root)
        }
        "doctor" | "control_runtime-doctor" => {
            let workspace_root = workspace_root(root, &parsed);
            Ok(run_doctor(&control_runtime_root, &workspace_root, &parsed))
        }
        "audit-plane" | "control-plane-audit" => {
            let workspace_root = workspace_root(root, &parsed);
            Ok(run_audit_plane(&control_runtime_root, &workspace_root, &parsed))
        }
        "daily-brief" => {
            let workspace_root = workspace_root(root, &parsed);
            Ok(run_daily_brief(&control_runtime_root, &workspace_root, &parsed))
        }
        "fail-playbook" => {
            let workspace_root = workspace_root(root, &parsed);
            Ok(run_fail_playbook(&control_runtime_root, &workspace_root, &parsed))
        }
        "help" | "--help" | "-h" => {
            usage();
            Ok(with_receipt(json!({
                "ok": true,
                "type": "operator_tooling_usage"
            })))
        }
        _ => Err(format!("unsupported_command:{command}")),
    };

    match result {
        Ok(value) => {
            let pretty = bool_flag(&parsed.flags, "pretty", true);
            if pretty {
                print_json(&value);
            } else {
                println!(
                    "{}",
                    serde_json::to_string(&value).unwrap_or_else(|_| "{\"ok\":false}".to_string())
                );
            }
            0
        }
        Err(err) => {
            if matches!(command.as_str(), "help" | "--help" | "-h") {
                0
            } else {
                print_json(&error_receipt(command.as_str(), &err, 1));
                1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_model_prefers_non_default_rules() {
        let policy = json!({
            "tiers": {
                "tier1": ["openrouter/gpt-5"],
                "tier2": ["openrouter/qwen3.5"],
                "tier3": ["ollama/llama3.2"]
            },
            "rules": [
                {"if": {"tags_any": ["security"]}, "use": "tier1"},
                {"if": {"default": true}, "use": "tier2"}
            ]
        });
        let route = route_model_with_policy(&policy, &["security".to_string()], DEFAULT_MODEL);
        assert_eq!(
            route.get("model").and_then(Value::as_str),
            Some("openrouter/gpt-5")
        );
        assert_eq!(route.get("tier").and_then(Value::as_str), Some("tier1"));
    }

    #[test]
    fn escalate_chain_orders_from_base_tier_three() {
        let policy = json!({
            "tiers": {
                "tier1": ["openrouter/gpt-5"],
                "tier2": ["openrouter/qwen3.5"],
                "tier3": ["ollama/llama3.2"]
            },
            "rules": [{"if": {"default": true}, "use": "tier3"}]
        });
        let payload = json!({"tags": ["general"]});
        let out = run_escalate_model(&policy, &payload, Path::new("/tmp/policy.json"));
        let chain = out
            .get("modelChain")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert_eq!(
            chain,
            vec![
                "ollama/llama3.2".to_string(),
                "openrouter/qwen3.5".to_string(),
                "openrouter/gpt-5".to_string()
            ]
        );
    }

    #[test]
    fn plan_validate_requires_rollback_for_high_risk() {
        let plan = json!({
            "goal": "Deploy auth lane change",
            "assumptions": ["a", "b"],
            "risks": ["x", "y"],
            "steps": ["Identify files", "Apply patch", "Run checks"],
            "tags": ["security", "deployment"]
        });
        let result = run_plan_validate(&plan);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap_or_default(),
            "plan_high_risk_requires_rollback"
        );
    }

    #[test]
    fn state_write_updates_last_task_and_decisions() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("state.json");
        let mut state = json!({});
        let payload = json!({
            "task": "Sync routing policy",
            "tags": ["routing", "policy"],
            "model": "openrouter/qwen3.5",
            "result": "ok",
            "decision": "promote tier2 fallback",
            "context": {"reason": "latency"}
        });
        let out = run_state_write(&mut state, &payload, &path).expect("state write");
        assert!(out.get("ok").and_then(Value::as_bool).unwrap_or(false));
        assert_eq!(
            state
                .pointer("/last_task/task")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "Sync routing policy"
        );
        let decisions = state
            .get("decisions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(decisions.len(), 1);
    }

    #[test]
    fn spawn_safe_enforces_default_tag_range() {
        let temp = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["spawn-safe".to_string()]);
        let payload = json!({
            "task": "route memory sync",
            "tags": ["ops", "routing"]
        });
        let out = build_spawn_packet(temp.path(), &parsed, &payload, false, false);
        assert!(out.is_err());
        assert_eq!(out.err().unwrap_or_default(), "tags_len_out_of_range:3-6");
    }

    #[test]
    fn spawn_safe_requires_plan_for_default_high_risk_tags() {
        let temp = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["spawn-safe".to_string()]);
        let payload = json!({
            "task": "deploy auth controls",
            "tags": ["security", "prod", "auth"]
        });
        let out = build_spawn_packet(temp.path(), &parsed, &payload, false, false);
        assert!(out.is_err());
        assert_eq!(
            out.err().unwrap_or_default(),
            "plan_required_for_high_risk_tags"
        );
    }

    #[test]
    fn safe_apply_backup_paths_preserve_directory_structure() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let backup_dir = root.join("backups").join("snapshot");
        let a = root.join("dir-a/config.json");
        let b = root.join("dir-b/config.json");
        let pa = safe_apply_backup_path(root, &backup_dir, &a);
        let pb = safe_apply_backup_path(root, &backup_dir, &b);
        assert_ne!(pa, pb);
        assert!(pa.ends_with(Path::new("dir-a/config.json")));
        assert!(pb.ends_with(Path::new("dir-b/config.json")));
    }

    #[test]
    fn safe_run_command_key_tracks_multiple_args() {
        let domain = "models";
        let args = vec![
            "list".to_string(),
            "--provider=openrouter".to_string(),
            "--region=us".to_string(),
        ];
        let key = safe_run_command_key(domain, &args);
        assert!(key.starts_with("models list"));
        assert!(key.contains("--provider=openrouter"));
        assert!(key.contains("--region=us"));
    }
}
