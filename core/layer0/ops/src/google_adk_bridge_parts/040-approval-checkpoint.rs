fn approval_checkpoint(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let queue_path = approval_queue_path(root, argv, payload);
    let action_id = clean_token(payload.get("action_id").and_then(Value::as_str), "");
    let decision = clean_token(payload.get("decision").and_then(Value::as_str), "pending");
    let result = if action_id.is_empty() || decision == "pending" {
        let new_action_id = if action_id.is_empty() {
            stable_id(
                "gadkapproval",
                &json!({
                    "tool_id": payload.get("tool_id"),
                    "summary": payload.get("summary"),
                    "risk": payload.get("risk")
                }),
            )
        } else {
            action_id.clone()
        };
        let action_envelope = json!({
            "action_id": new_action_id,
            "directive_id": "google-adk-bridge",
            "type": "tool_invocation",
            "summary": clean_text(payload.get("summary").and_then(Value::as_str), 200),
            "payload_pointer": clean_text(payload.get("tool_id").and_then(Value::as_str), 160),
        });
        let queue_payload = json!({
            "action_envelope": action_envelope,
            "reason": clean_text(payload.get("reason").and_then(Value::as_str), 200),
        });
        let encoded = BASE64_STANDARD.encode(encode_json_arg(&queue_payload)?.as_bytes());
        let exit = crate::approval_gate_kernel::run(
            root,
            &[
                "queue".to_string(),
                format!("--payload-base64={encoded}"),
                format!("--queue-path={}", queue_path.display()),
            ],
        );
        if exit != 0 {
            return Err("google_adk_approval_queue_failed".to_string());
        }
        json!({
            "action_id": new_action_id,
            "decision": "pending",
            "status": approval_status_from_queue(&queue_path, &new_action_id),
        })
    } else {
        let args = if decision == "approve" {
            vec![
                "approve".to_string(),
                format!("--action-id={action_id}"),
                format!("--queue-path={}", queue_path.display()),
            ]
        } else {
            vec![
                "deny".to_string(),
                format!("--action-id={action_id}"),
                format!(
                    "--reason={}",
                    clean_text(payload.get("reason").and_then(Value::as_str), 120)
                ),
                format!("--queue-path={}", queue_path.display()),
            ]
        };
        let exit = crate::approval_gate_kernel::run(root, &args);
        if exit != 0 {
            return Err(format!("google_adk_approval_{}_failed", decision));
        }
        json!({
            "action_id": action_id,
            "decision": decision,
            "status": approval_status_from_queue(&queue_path, &action_id),
        })
    };
    let action_id = result
        .get("action_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "approval_records").insert(
        action_id.clone(),
        json!({
            "action_id": action_id,
            "tool_id": payload.get("tool_id").cloned().unwrap_or(Value::Null),
            "queue_path": rel(root, &queue_path),
            "status": result.get("status").cloned().unwrap_or(Value::Null),
            "updated_at": now_iso(),
        }),
    );
    Ok(json!({
        "ok": true,
        "approval": result,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-010.5", adk_claim("V6-WORKFLOW-010.5")),
    }))
}

fn rewind_session(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let session_id = clean_token(payload.get("session_id").and_then(Value::as_str), "");
    if session_id.is_empty() {
        return Err("google_adk_rewind_session_id_required".to_string());
    }
    let snapshot = state
        .get("session_snapshots")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&session_id))
        .cloned()
        .ok_or_else(|| format!("google_adk_snapshot_missing:{session_id}"))?;
    let context_payload = snapshot
        .get("context_payload")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let context_json = encode_json_arg(&context_payload)?;
    let exit = crate::swarm_runtime::run(
        root,
        &[
            "sessions".to_string(),
            "context-put".to_string(),
            format!("--session-id={session_id}"),
            format!("--context-json={context_json}"),
            "--merge=0".to_string(),
            format!("--state-path={}", swarm_state_path.display()),
        ],
    );
    if exit != 0 {
        return Err("google_adk_rewind_context_restore_failed".to_string());
    }
    emit_native_trace(
        root,
        &clean_token(
            snapshot.get("snapshot_id").and_then(Value::as_str),
            "google-adk-rewind",
        ),
        "google_adk_rewind",
        &format!("rewound session {session_id}"),
    )?;
    Ok(json!({
        "ok": true,
        "restored": {
            "session_id": session_id,
            "snapshot": snapshot,
            "swarm_state_path": rel(root, &swarm_state_path),
        },
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-010.6", adk_claim("V6-WORKFLOW-010.6")),
    }))
}

fn record_evaluation(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let session_id = clean_token(
        payload.get("session_id").and_then(Value::as_str),
        "google-adk-session",
    );
    let metrics = payload.get("metrics").cloned().unwrap_or_else(|| json!({}));
    let evaluation = json!({
        "evaluation_id": stable_id("gadkeval", &json!({"session_id": session_id, "metrics": metrics})),
        "session_id": session_id,
        "metrics": metrics,
        "score": parse_f64_value(payload.get("score"), 0.0, 0.0, 1.0),
        "profile": clean_token(payload.get("profile").and_then(Value::as_str), "rich"),
        "evaluated_at": now_iso(),
    });
    let evaluation_id = evaluation
        .get("evaluation_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "evaluations").insert(evaluation_id.clone(), evaluation.clone());
    emit_native_trace(
        root,
        &evaluation_id,
        "google_adk_eval",
        &format!(
            "session_id={} score={:.2}",
            session_id,
            evaluation
                .get("score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
    )?;
    Ok(json!({
        "ok": true,
        "evaluation": evaluation,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-010.6", adk_claim("V6-WORKFLOW-010.6")),
    }))
}

fn sandbox_execute(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let language = clean_token(payload.get("language").and_then(Value::as_str), "python");
    if !allowed_language(&language) {
        return Err("google_adk_sandbox_language_invalid".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let cloud = clean_token(payload.get("cloud").and_then(Value::as_str), "gcp");
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/polyglot/google_adk_runtime_bridge.ts"),
    )?;
    if !bridge_path.starts_with("adapters/") {
        return Err("google_adk_sandbox_bridge_must_be_adapter_owned".to_string());
    }
    let degraded = matches!(profile.as_str(), "pure" | "tiny-max") && !cloud.is_empty();
    let reason_code = if degraded {
        "cloud_integration_requires_rich_profile"
    } else {
        let tier = clean_token(payload.get("tier").and_then(Value::as_str), "wasm");
        let escape_attempt = parse_bool_value(payload.get("escape_attempt"), false);
        let exit = crate::canyon_plane::run(
            root,
            &[
                "sandbox".to_string(),
                "--op=run".to_string(),
                format!("--tier={tier}"),
                format!("--language={language}"),
                format!(
                    "--fuel={}",
                    parse_u64_value(payload.get("fuel"), 2000, 200, 50000)
                ),
                format!(
                    "--epoch={}",
                    parse_u64_value(payload.get("epoch"), 4, 1, 64)
                ),
                format!("--escape-attempt={}", if escape_attempt { 1 } else { 0 }),
                "--strict=1".to_string(),
            ],
        );
        if exit != 0 {
            return Err("google_adk_sandbox_execution_failed".to_string());
        }
        "sandbox_ok"
    };
    let record = json!({
        "sandbox_id": stable_id("gadksbx", &json!({"language": language, "profile": profile, "cloud": cloud})),
        "language": language,
        "profile": profile,
        "cloud": cloud,
        "bridge_path": bridge_path,
        "degraded": degraded,
        "reason_code": reason_code,
        "executed_at": now_iso(),
    });
    let sandbox_id = record
        .get("sandbox_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "sandbox_runs").insert(sandbox_id, record.clone());
    Ok(json!({
        "ok": true,
        "sandbox": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-010.7", adk_claim("V6-WORKFLOW-010.7")),
    }))
}

fn deploy_shell(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let shell_path = normalize_shell_path(
        root,
        payload
            .get("shell_path")
            .and_then(Value::as_str)
            .unwrap_or("client/runtime/systems/workflow/google_adk_bridge.ts"),
    )?;
    let target = clean_token(payload.get("target").and_then(Value::as_str), "local");
    let record = json!({
        "deployment_id": stable_id("gadkdep", &json!({"shell_path": shell_path, "target": target})),
        "shell_name": clean_token(payload.get("shell_name").and_then(Value::as_str), "google-adk-shell"),
        "shell_path": shell_path,
        "target": target,
        "deletable": true,
        "authority_delegate": "core://google-adk-bridge",
        "artifact_path": clean_text(payload.get("artifact_path").and_then(Value::as_str), 240),
        "deployed_at": now_iso(),
    });
    let deployment_id = record
        .get("deployment_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "deployments").insert(deployment_id, record.clone());
    Ok(json!({
        "ok": true,
        "deployment": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-010.8", adk_claim("V6-WORKFLOW-010.8")),
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].as_str();
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("google_adk_bridge_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let state_path = state_path(root, argv, input);
    let history_path = history_path(root, argv, input);
    let mut state = load_state(&state_path);

    let result = match command {
        "status" => Ok(json!({
            "ok": true,
            "state_path": rel(root, &state_path),
            "history_path": rel(root, &history_path),
            "a2a_agents": as_object_mut(&mut state, "a2a_agents").len(),
            "llm_agents": as_object_mut(&mut state, "llm_agents").len(),
            "tool_manifests": as_object_mut(&mut state, "tool_manifests").len(),
            "hierarchies": as_object_mut(&mut state, "hierarchies").len(),
            "approval_records": as_object_mut(&mut state, "approval_records").len(),
            "session_snapshots": as_object_mut(&mut state, "session_snapshots").len(),
            "evaluations": as_object_mut(&mut state, "evaluations").len(),
            "sandbox_runs": as_object_mut(&mut state, "sandbox_runs").len(),
            "deployments": as_object_mut(&mut state, "deployments").len(),
            "runtime_bridges": as_object_mut(&mut state, "runtime_bridges").len(),
            "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
        })),
        "register-a2a-agent" => register_a2a_agent(root, &mut state, input),
        "send-a2a-message" => send_a2a_message(root, argv, &mut state, input),
        "register-runtime-bridge" => register_runtime_bridge(root, &mut state, input),
        "route-model" => route_model(&state, input),
        "run-llm-agent" => run_llm_agent(root, argv, &mut state, input),
        "register-tool-manifest" => register_tool_manifest(root, &mut state, input),
        "invoke-tool-manifest" => invoke_tool_manifest(root, argv, &mut state, input),
        "coordinate-hierarchy" => coordinate_hierarchy(root, argv, &mut state, input),
        "approval-checkpoint" => approval_checkpoint(root, argv, &mut state, input),
        "rewind-session" => rewind_session(root, argv, &mut state, input),
        "record-evaluation" => record_evaluation(root, &mut state, input),
        "sandbox-execute" => sandbox_execute(root, &mut state, input),
        "deploy-shell" => deploy_shell(root, &mut state, input),
        _ => Err(format!("unknown_google_adk_bridge_command:{command}")),
    };

    match result {
        Ok(payload) => {
            let receipt = cli_receipt(
                &format!("google_adk_bridge_{}", command.replace('-', "_")),
                payload,
            );
            state["last_receipt"] = receipt.clone();
            if let Err(err) = save_state(&state_path, &state)
                .and_then(|_| append_history(&history_path, &receipt))
            {
                print_json_line(&cli_error("google_adk_bridge_error", &err));
                return 1;
            }
            print_json_line(&receipt);
            0
        }
        Err(err) => {
            print_json_line(&cli_error("google_adk_bridge_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_bridge_route_degrades_polyglot_in_pure_mode() {
        let mut state = default_state();
        let payload = json!({
            "name": "python-gateway",
            "language": "python",
            "provider": "google",
            "bridge_path": "adapters/polyglot/google_adk_runtime_bridge.ts",
            "supported_profiles": ["rich", "pure"]
        });
        let _ = register_runtime_bridge(Path::new("."), &mut state, payload.as_object().unwrap())
            .expect("register");
        let out = route_model(
            &state,
            &Map::from_iter([
                ("language".to_string(), json!("python")),
                ("provider".to_string(), json!("google")),
                ("model".to_string(), json!("gemini-2.0-flash")),
                ("profile".to_string(), json!("pure")),
            ]),
        )
        .expect("route");
        assert_eq!(out["route"]["degraded"].as_bool(), Some(true));
    }
}

