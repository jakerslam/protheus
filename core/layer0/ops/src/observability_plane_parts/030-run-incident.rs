fn run_incident(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        INCIDENT_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "observability_incident_response_contract",
            "allowed_ops": ["trigger", "status", "resolve"],
            "default_response_actions": ["snapshot", "log-capture", "recovery"],
            "allowed_response_actions": ["snapshot", "log-capture", "recovery", "quarantine", "rollback", "page-oncall"]
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allowed_ops = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if strict
        && !allowed_ops
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == op)
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "observability_plane_incident",
            "errors": ["observability_incident_op_invalid"]
        });
    }

    let path = incidents_state_path(root);
    let mut state = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "incidents": {},
            "last_updated_at": crate::now_iso()
        })
    });
    if !state
        .get("incidents")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["incidents"] = Value::Object(serde_json::Map::new());
    }

    if op == "status" {
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "observability_plane_incident",
            "lane": "core/layer0/ops",
            "op": "status",
            "state": state,
            "claim_evidence": [
                {
                    "id": "V6-OBSERVABILITY-001.3",
                    "claim": "incident_response_orchestrator_surfaces_active_incident_state",
                    "evidence": {
                        "incident_count": state
                            .get("incidents")
                            .and_then(Value::as_object)
                            .map(|m| m.len())
                            .unwrap_or(0)
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    let incident_id = clean_id(
        parsed
            .flags
            .get("incident-id")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("id").map(String::as_str)),
        "incident-default",
    );

    if op == "trigger" {
        let runbook = clean(
            parsed
                .flags
                .get("runbook")
                .cloned()
                .unwrap_or_else(|| "default-runbook".to_string()),
            120,
        );
        let action = clean(
            parsed.flags.get("action").cloned().unwrap_or_else(|| {
                contract
                    .get("default_response_actions")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("+")
            }),
            160,
        );
        let requested_actions = {
            let rows = split_actions(&action);
            if rows.is_empty() {
                contract
                    .get("default_response_actions")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|row| row.to_ascii_lowercase())
                    .collect::<Vec<_>>()
            } else {
                rows
            }
        };
        let mut allowed_actions = contract
            .get("allowed_response_actions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(|row| row.to_ascii_lowercase())
            .collect::<Vec<_>>();
        if allowed_actions.is_empty() {
            allowed_actions = contract
                .get("default_response_actions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
                .map(|row| row.to_ascii_lowercase())
                .collect::<Vec<_>>();
        }
        if allowed_actions.is_empty() {
            allowed_actions = vec![
                "snapshot".to_string(),
                "log-capture".to_string(),
                "recovery".to_string(),
                "quarantine".to_string(),
                "rollback".to_string(),
                "page-oncall".to_string(),
            ];
        }
        if strict
            && requested_actions
                .iter()
                .any(|row| !allowed_actions.iter().any(|allowed| allowed == row))
        {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "observability_plane_incident",
                "errors": ["observability_incident_response_action_invalid"]
            });
        }
        let mut response_receipts = Vec::<Value>::new();
        let artifacts_dir = incident_artifacts_dir(root, &incident_id);
        let _ = std::fs::create_dir_all(&artifacts_dir);
        for (idx, step) in requested_actions.iter().enumerate() {
            let artifact_path = artifacts_dir.join(format!("{:02}_{}.json", idx + 1, step));
            let artifact = match step.as_str() {
                "snapshot" => json!({
                    "step": step,
                    "context_snapshot": intelligent_context(root),
                    "ts": crate::now_iso()
                }),
                "log-capture" => json!({
                    "step": step,
                    "log_sources": [
                        alerts_state_path(root).display().to_string(),
                        workflows_state_path(root).display().to_string(),
                        incidents_state_path(root).display().to_string()
                    ],
                    "ts": crate::now_iso()
                }),
                "recovery" => json!({
                    "step": step,
                    "recovery_plan": {
                        "runbook": runbook,
                        "strategy": "bounded_rollback_then_verify"
                    },
                    "ts": crate::now_iso()
                }),
                _ => json!({
                    "step": step,
                    "runbook": runbook,
                    "policy_bounded": true,
                    "ts": crate::now_iso()
                }),
            };
            let _ = write_json(&artifact_path, &artifact);
            response_receipts.push(json!({
                "index": idx + 1,
                "step": step,
                "artifact_path": artifact_path.display().to_string(),
                "artifact_sha256": sha256_hex_str(&artifact.to_string())
            }));
        }
        let context = intelligent_context(root);
        let external_dispatch =
            run_incident_external_dispatch(parsed, &incident_id, &runbook, &requested_actions);
        if strict && external_dispatch.hard_fail {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "observability_plane_incident",
                "errors": ["observability_incident_external_dispatch_required_failed"],
                "incident_id": incident_id,
                "external_dispatch": {
                    "requested": external_dispatch.requested,
                    "mode": external_dispatch.mode,
                    "receipts": external_dispatch.receipts
                }
            });
        }
        let incident = json!({
            "incident_id": incident_id,
            "runbook": runbook,
            "action": action,
            "response_actions": requested_actions,
            "response_receipts": response_receipts,
            "external_dispatch": {
                "requested": external_dispatch.requested,
                "mode": external_dispatch.mode,
                "receipts": external_dispatch.receipts
            },
            "status": "active",
            "context": context,
            "triggered_at": crate::now_iso()
        });
        state["incidents"][&incident_id] = incident.clone();
        state["last_updated_at"] = Value::String(crate::now_iso());
        let _ = write_json(&path, &state);
        let _ = append_jsonl(
            &state_root(root).join("incidents").join("history.jsonl"),
            &json!({"op": "trigger", "incident_id": incident_id, "ts": crate::now_iso()}),
        );
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "observability_plane_incident",
            "lane": "core/layer0/ops",
            "op": "trigger",
            "incident": incident,
            "artifact": {
                "path": path.display().to_string(),
                "sha256": sha256_hex_str(&state.to_string())
            },
            "claim_evidence": [
                {
                    "id": "V6-OBSERVABILITY-001.3",
                    "claim": "incident_triggers_invoke_policy_bounded_response_actions_with_receipts",
                    "evidence": {
                    "incident_id": incident_id,
                    "external_dispatch_count": incident
                        .get("external_dispatch")
                        .and_then(|v| v.get("receipts"))
                        .and_then(Value::as_array)
                        .map(|rows| rows.len())
                        .unwrap_or(0),
                    "response_action_count": incident
                        .get("response_actions")
                        .and_then(Value::as_array)
                            .map(|rows| rows.len())
                            .unwrap_or(0)
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if strict && state["incidents"].get(&incident_id).is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "observability_plane_incident",
            "errors": ["observability_incident_not_found"]
        });
    }
    state["incidents"][&incident_id]["status"] = Value::String("resolved".to_string());
    state["incidents"][&incident_id]["resolved_at"] = Value::String(crate::now_iso());
    state["last_updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&path, &state);
    let _ = append_jsonl(
        &state_root(root).join("incidents").join("history.jsonl"),
        &json!({"op": "resolve", "incident_id": incident_id, "ts": crate::now_iso()}),
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "observability_plane_incident",
        "lane": "core/layer0/ops",
        "op": "resolve",
        "incident": state["incidents"][&incident_id].clone(),
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-OBSERVABILITY-001.3",
                "claim": "incident_resolution_generates_deterministic_orchestration_receipts",
                "evidence": {
                    "incident_id": incident_id
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_selfhost(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        SELFHOST_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "observability_self_hosted_deploy_contract",
            "allowed_profiles": ["docker-local", "k8s-local"],
            "telemetry_mandatory": false
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        20,
    )
    .to_ascii_lowercase();
    if op == "status" {
        let latest = read_json(&selfhost_state_path(root)).unwrap_or_else(|| Value::Null);
        let health = read_json(&selfhost_health_path(root)).unwrap_or_else(|| Value::Null);
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "observability_plane_selfhost",
            "lane": "core/layer0/ops",
            "op": "status",
            "latest": latest,
            "deployment_health": health,
            "claim_evidence": [
                {
                    "id": "V6-OBSERVABILITY-001.4",
                    "claim": "self_hosted_observability_profile_status_is_available_without_mandatory_telemetry",
                    "evidence": {
                        "has_latest": !latest.is_null(),
                        "has_health": !health.is_null()
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }
    if op != "deploy" {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "observability_plane_selfhost",
            "errors": ["observability_selfhost_op_invalid"]
        });
    }

    let profile = clean(
        parsed
            .flags
            .get("profile")
            .cloned()
            .unwrap_or_else(|| "docker-local".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let profile_allowed = contract
        .get("allowed_profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .any(|row| row == profile);
    if strict && !profile_allowed {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "observability_plane_selfhost",
            "errors": ["observability_selfhost_profile_invalid"]
        });
    }
    let telemetry_opt_in = parse_bool(parsed.flags.get("telemetry-opt-in"), false);
    let telemetry_mandatory = contract
        .get("telemetry_mandatory")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if strict && telemetry_mandatory && !telemetry_opt_in {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "observability_plane_selfhost",
            "errors": ["observability_selfhost_telemetry_required_by_contract"]
        });
    }
    let deployment = json!({
        "version": "v1",
        "profile": profile,
        "telemetry_opt_in": telemetry_opt_in,
        "command": if profile == "k8s-local" { "kubectl apply -f observability-stack.yaml" } else { "docker compose -f observability-stack.yml up -d" },
        "deployed_at": crate::now_iso()
    });
    let path = selfhost_state_path(root);
    let _ = write_json(&path, &deployment);
    let _ = append_jsonl(
        &state_root(root).join("deploy").join("history.jsonl"),
        &deployment,
    );
    let deployment_health = {
        let components = json!({
            "alerts_store_ready": alerts_state_path(root).parent().map(|p| p.exists()).unwrap_or(false),
            "workflow_registry_ready": workflows_state_path(root).parent().map(|p| p.exists()).unwrap_or(false),
            "incident_store_ready": incidents_state_path(root).parent().map(|p| p.exists()).unwrap_or(false)
        });
        let healthy = components
            .as_object()
            .map(|rows| rows.values().all(|v| v.as_bool().unwrap_or(false)))
            .unwrap_or(false);
        json!({
            "profile": profile,
            "healthy": healthy,
            "components": components,
            "checked_at": crate::now_iso()
        })
    };
    let health_path = selfhost_health_path(root);
    let _ = write_json(&health_path, &deployment_health);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "observability_plane_selfhost",
        "lane": "core/layer0/ops",
        "op": "deploy",
        "deployment": deployment,
        "deployment_health": deployment_health,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&read_json(&path).unwrap_or_else(|| json!({})).to_string()),
            "health_path": health_path.display().to_string(),
            "health_sha256": sha256_hex_str(&read_json(&health_path).unwrap_or_else(|| json!({})).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-OBSERVABILITY-001.4",
                "claim": "single_command_self_hosted_observability_profile_is_deployable_without_mandatory_telemetry",
                "evidence": {
                    "profile": profile,
                    "telemetry_opt_in": telemetry_opt_in,
                    "health_checked": true
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
