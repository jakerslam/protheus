fn record_id(row: &Value) -> String {
    clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120)
}

pub fn handle(
    root: &Path,
    method: &str,
    path_only: &str,
    body: &[u8],
    _snapshot: &Value,
) -> Option<CompatApiResponse> {
    if let Some(segments) = workflow_path_segments(path_only) {
        let mut workflows = load_workflows(root);
        if method == "GET" && segments.is_empty() {
            return Some(CompatApiResponse {
                status: 200,
                payload: Value::Array(workflows),
            });
        }
        if !segments.is_empty() {
            let workflow_id = clean_id(&segments[0], 120);
            if workflow_id.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "workflow_id_required"}),
                });
            }
            if method == "GET" && segments.len() == 1 {
                if let Some(found) = workflows
                    .iter()
                    .find(|row| record_id(row) == workflow_id)
                    .cloned()
                {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: found,
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "workflow_not_found"}),
                });
            }
            if method == "GET" && segments.len() == 2 && segments[1] == "runs" {
                let runs_state = load_workflow_runs(root);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "workflow_id": workflow_id,
                        "runs": runs_for_workflow(&runs_state, &workflow_id)
                    }),
                });
            }
            if method == "GET" && segments.len() == 2 && segments[1] == "validate" {
                if let Some(found) = workflows
                    .iter()
                    .find(|row| record_id(row) == workflow_id)
                    .cloned()
                {
                    let validation = validate_workflow_graph(&found);
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({
                            "ok": true,
                            "workflow_id": workflow_id,
                            "validation": validation
                        }),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "workflow_not_found"}),
                });
            }
            if method == "POST" && segments.len() == 2 && segments[1] == "run" {
                if let Some(idx) = workflows
                    .iter()
                    .position(|row| record_id(row) == workflow_id)
                {
                    let validation = validate_workflow_graph(&workflows[idx]);
                    if !validation
                        .get("valid")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        return Some(CompatApiResponse {
                            status: 400,
                            payload: json!({
                                "ok": false,
                                "error": "workflow_validation_failed",
                                "workflow_id": workflow_id,
                                "validation": validation
                            }),
                        });
                    }
                    let request = parse_json(body);
                    let input = clean_text(
                        request.get("input").and_then(Value::as_str).unwrap_or(""),
                        10_000,
                    );
                    let started = Utc::now();
                    let (output, step_rows) = workflow_output(&input, &workflows[idx]);
                    let finished = Utc::now();
                    workflows[idx]["updated_at"] = Value::String(crate::now_iso());
                    workflows[idx]["last_run"] = Value::String(crate::now_iso());
                    workflows[idx]["validation"] = validation.clone();
                    save_workflows(root, &workflows);
                    let mut runs_state = load_workflow_runs(root);
                    let mut runs = runs_for_workflow(&runs_state, &workflow_id);
                    let run_id = make_id(
                        "run",
                        &json!({"workflow_id": workflow_id, "ts": crate::now_iso(), "input": input}),
                    );
                    let run = json!({
                        "run_id": run_id,
                        "workflow_id": workflow_id,
                        "status": "completed",
                        "input": input,
                        "output": output,
                        "steps": step_rows,
                        "started_at": started.to_rfc3339(),
                        "finished_at": finished.to_rfc3339(),
                        "duration_ms": (finished - started).num_milliseconds().max(1)
                    });
                    runs.push(run.clone());
                    set_runs_for_workflow(&mut runs_state, &workflow_id, runs);
                    save_workflow_runs(root, runs_state);
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({
                            "ok": true,
                            "status": "completed",
                            "workflow_id": workflow_id,
                            "run_id": run["run_id"].clone(),
                            "output": run["output"].clone(),
                            "run": run,
                            "validation": validation
                        }),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "workflow_not_found"}),
                });
            }
            if method == "PUT" && segments.len() == 1 {
                if let Some(idx) = workflows
                    .iter()
                    .position(|row| record_id(row) == workflow_id)
                {
                    let request = parse_json(body);
                    let mut merged = workflows[idx].clone();
                    if request.get("name").and_then(Value::as_str).is_some() {
                        merged["name"] = Value::String(clean_text(
                            request
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or("workflow"),
                            160,
                        ));
                    }
                    if request.get("description").and_then(Value::as_str).is_some() {
                        merged["description"] = Value::String(clean_text(
                            request
                                .get("description")
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                            1000,
                        ));
                    }
                    if request.get("steps").is_some() {
                        merged["steps"] = Value::Array(
                            array_from_value(request.get("steps").unwrap_or(&json!([])), "steps")
                                .iter()
                                .enumerate()
                                .map(|(step_idx, step)| normalize_workflow_step(step, step_idx))
                                .collect::<Vec<_>>(),
                        );
                    }
                    merged["id"] = Value::String(workflow_id.clone());
                    merged["updated_at"] = Value::String(crate::now_iso());
                    let mut normalized = normalize_workflow(&merged);
                    let validation = validate_workflow_graph(&normalized);
                    if !validation
                        .get("valid")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        return Some(CompatApiResponse {
                            status: 400,
                            payload: json!({
                                "ok": false,
                                "error": "workflow_validation_failed",
                                "workflow_id": workflow_id,
                                "validation": validation
                            }),
                        });
                    }
                    normalized["validation"] = validation;
                    workflows[idx] = normalized;
                    save_workflows(root, &workflows);
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({"ok": true, "workflow": workflows[idx].clone()}),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "workflow_not_found"}),
                });
            }
            if method == "DELETE" && segments.len() == 1 {
                let before = workflows.len();
                workflows.retain(|row| record_id(row) != workflow_id);
                if workflows.len() == before {
                    return Some(CompatApiResponse {
                        status: 404,
                        payload: json!({"ok": false, "error": "workflow_not_found"}),
                    });
                }
                save_workflows(root, &workflows);
                let mut runs_state = load_workflow_runs(root);
                set_runs_for_workflow(&mut runs_state, &workflow_id, Vec::new());
                save_workflow_runs(root, runs_state);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "deleted": true, "workflow_id": workflow_id}),
                });
            }
        }
        if method == "POST" && segments.is_empty() {
            let request = parse_json(body);
            let mut workflow = normalize_workflow(&request);
            let validation = validate_workflow_graph(&workflow);
            if !validation
                .get("valid")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({
                        "ok": false,
                        "error": "workflow_validation_failed",
                        "validation": validation
                    }),
                });
            }
            if workflow
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                workflow["id"] = Value::String(make_id(
                    "wf",
                    &json!({"name": workflow.get("name").cloned().unwrap_or(Value::Null), "ts": crate::now_iso()}),
                ));
            }
            workflow["created_at"] = Value::String(crate::now_iso());
            workflow["updated_at"] = Value::String(crate::now_iso());
            workflow["validation"] = validation;
            workflows.push(workflow.clone());
            save_workflows(root, &workflows);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "workflow": workflow}),
            });
        }
        return Some(CompatApiResponse {
            status: 405,
            payload: json!({"ok": false, "error": "method_not_allowed"}),
        });
    }

    if method == "GET" && path_only == "/api/cron/jobs" {
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "jobs": load_jobs(root)}),
        });
    }

    if method == "POST" && path_only == "/api/cron/jobs" {
        let request = parse_json(body);
        let mut jobs = load_jobs(root);
        let mut row = normalize_job(&request);
        if row
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        {
            row["id"] = Value::String(make_id(
                "cron",
                &json!({"name": row.get("name").cloned().unwrap_or(Value::Null), "ts": crate::now_iso()}),
            ));
        }
        row["created_at"] = Value::String(crate::now_iso());
        row["updated_at"] = Value::String(crate::now_iso());
        row["next_run"] = if as_bool(row.get("enabled"), true) {
            schedule_next_run(row.get("schedule").unwrap_or(&json!({})))
        } else {
            Value::Null
        };
        jobs.push(row.clone());
        save_jobs(root, &jobs);
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "job": row}),
        });
    }

    if path_only.starts_with("/api/cron/jobs/") {
        let tail = path_only.trim_start_matches("/api/cron/jobs/");
        let segments = tail
            .split('/')
            .filter_map(|v| {
                let cleaned = clean_text(v, 200);
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
            })
            .collect::<Vec<_>>();
        if !segments.is_empty() {
            let job_id = clean_id(&segments[0], 120);
            let mut jobs = load_jobs(root);
            if method == "PUT" && segments.len() == 2 && segments[1] == "enable" {
                let request = parse_json(body);
                if let Some(idx) = jobs.iter().position(|row| record_id(row) == job_id) {
                    let enabled = as_bool(request.get("enabled"), true);
                    jobs[idx]["enabled"] = Value::Bool(enabled);
                    jobs[idx]["updated_at"] = Value::String(crate::now_iso());
                    jobs[idx]["next_run"] = if enabled {
                        schedule_next_run(jobs[idx].get("schedule").unwrap_or(&json!({})))
                    } else {
                        Value::Null
                    };
                    save_jobs(root, &jobs);
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({"ok": true, "job": jobs[idx].clone()}),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "job_not_found"}),
                });
            }
            if method == "DELETE" && segments.len() == 1 {
                let before = jobs.len();
                jobs.retain(|row| record_id(row) != job_id);
                if before == jobs.len() {
                    return Some(CompatApiResponse {
                        status: 404,
                        payload: json!({"ok": false, "error": "job_not_found"}),
                    });
                }
                save_jobs(root, &jobs);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "deleted": true, "job_id": job_id}),
                });
            }
        }
        return Some(CompatApiResponse {
            status: 405,
            payload: json!({"ok": false, "error": "method_not_allowed"}),
        });
    }

    if path_only.starts_with("/api/schedules/") && method == "POST" && path_only.ends_with("/run") {
        let job_id = clean_id(
            path_only
                .trim_start_matches("/api/schedules/")
                .trim_end_matches("/run")
                .trim_matches('/'),
            120,
        );
        if job_id.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "job_id_required"}),
            });
        }
        let mut jobs = load_jobs(root);
        if let Some(idx) = jobs.iter().position(|row| record_id(row) == job_id) {
            let ran_at = crate::now_iso();
            jobs[idx]["last_run"] = Value::String(ran_at.clone());
            jobs[idx]["updated_at"] = Value::String(ran_at.clone());
            jobs[idx]["run_count"] = Value::from(as_i64(jobs[idx].get("run_count"), 0).max(0) + 1);
            jobs[idx]["next_run"] = if as_bool(jobs[idx].get("enabled"), true) {
                schedule_next_run(jobs[idx].get("schedule").unwrap_or(&json!({})))
            } else {
                Value::Null
            };
            let agent_id = clean_text(
                jobs[idx]
                    .get("agent_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                140,
            );
            if !agent_id.is_empty() {
                let user_text = clean_text(
                    jobs[idx]
                        .pointer("/action/message")
                        .and_then(Value::as_str)
                        .unwrap_or("Scheduled task executed."),
                    2000,
                );
                let assistant_text = "Scheduled execution logged by Rust core.";
                let _ = crate::dashboard_agent_state::append_turn(
                    root,
                    &agent_id,
                    &user_text,
                    assistant_text,
                );
            }
            save_jobs(root, &jobs);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "status": "completed",
                    "job_id": job_id,
                    "ran_at": ran_at
                }),
            });
        }
        return Some(CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "status": "failed", "error": "job_not_found"}),
        });
    }

    if path_only == "/api/triggers" && method == "GET" {
        return Some(CompatApiResponse {
            status: 200,
            payload: Value::Array(load_triggers(root)),
        });
    }

    if path_only.starts_with("/api/triggers/") {
        let trigger_id = clean_id(path_only.trim_start_matches("/api/triggers/"), 120);
        let mut triggers = load_triggers(root);
        if method == "PUT" {
            let request = parse_json(body);
            if let Some(idx) = triggers.iter().position(|row| record_id(row) == trigger_id) {
                if request.get("enabled").is_some() {
                    triggers[idx]["enabled"] = Value::Bool(as_bool(request.get("enabled"), true));
                }
                triggers[idx]["updated_at"] = Value::String(crate::now_iso());
                save_triggers(root, &triggers);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "trigger": triggers[idx].clone()}),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "trigger_not_found"}),
            });
        }
        if method == "DELETE" {
            let before = triggers.len();
            triggers.retain(|row| record_id(row) != trigger_id);
            if before == triggers.len() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "trigger_not_found"}),
                });
            }
            save_triggers(root, &triggers);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "deleted": true, "trigger_id": trigger_id}),
            });
        }
        return Some(CompatApiResponse {
            status: 405,
            payload: json!({"ok": false, "error": "method_not_allowed"}),
        });
    }

    if path_only == "/api/approvals" && method == "GET" {
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "approvals": load_approvals(root)}),
        });
    }

    if path_only.starts_with("/api/approvals/") && method == "POST" {
        let tail = path_only.trim_start_matches("/api/approvals/");
        let segments = tail
            .split('/')
            .filter_map(|v| {
                let cleaned = clean_text(v, 200);
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
            })
            .collect::<Vec<_>>();
        if segments.len() == 2 {
            let approval_id = clean_id(&segments[0], 120);
            let action = clean_id(&segments[1], 40);
            if action == "approve" || action == "reject" {
                let mut approvals = load_approvals(root);
                if let Some(idx) = approvals
                    .iter()
                    .position(|row| record_id(row) == approval_id)
                {
                    approvals[idx]["status"] = Value::String(if action == "approve" {
                        "approved".to_string()
                    } else {
                        "rejected".to_string()
                    });
                    approvals[idx]["updated_at"] = Value::String(crate::now_iso());
                    save_approvals(root, &approvals);
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({"ok": true, "approval": approvals[idx].clone()}),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "approval_not_found"}),
                });
            }
        }
        return Some(CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "invalid_approval_route"}),
        });
    }

    if path_only == "/api/eyes" && method == "GET" {
        let eyes = load_eyes(root);
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": true,
                "eyes": eyes,
                "catalog": {"eyes": eyes}
            }),
        });
    }

    if path_only == "/api/eyes" && method == "POST" {
        let request = parse_json(body);
        let name = clean_text(
            request.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        let url = clean_text(
            request.get("url").and_then(Value::as_str).unwrap_or(""),
            500,
        );
        let api_key = clean_text(
            request.get("api_key").and_then(Value::as_str).unwrap_or(""),
            4000,
        );
        if name.is_empty() && url.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "name_or_url_required"}),
            });
        }
        if url.is_empty() && api_key.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "url_or_api_key_required"}),
            });
        }
        let mut eyes = load_eyes(root);
        let canonical_name = if name.is_empty() {
            host_from_url(&url)
        } else {
            name
        };
        let mut id = clean_id(request.get("id").and_then(Value::as_str).unwrap_or(""), 120);
        if id.is_empty() {
            id = clean_id(&canonical_name, 120);
        }
        if id.is_empty() {
            id = make_id(
                "eye",
                &json!({"name": canonical_name, "url": url, "ts": crate::now_iso()}),
            );
        }
        let now = crate::now_iso();
        let topics = clean_text(
            request.get("topics").and_then(Value::as_str).unwrap_or(""),
            600,
        );
        let topic_rows = topics
            .split(',')
            .filter_map(|v| {
                let t = clean_text(v, 80);
                if t.is_empty() {
                    None
                } else {
                    Some(Value::String(t))
                }
            })
            .collect::<Vec<_>>();
        let status = {
            let raw = clean_text(
                request
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("active"),
                24,
            )
            .to_ascii_lowercase();
            match raw.as_str() {
                "active" | "paused" | "dormant" | "disabled" => raw,
                _ => "active".to_string(),
            }
        };
        let cadence = as_i64(request.get("cadence_hours"), 4).clamp(1, 168);
        let mut row = json!({
            "uid": make_id("eyeuid", &json!({"id": id, "ts": now})),
            "id": id,
            "name": if canonical_name.is_empty() { "eye" } else { &canonical_name },
            "status": status,
            "endpoint_url": url,
            "endpoint_host": host_from_url(request.get("url").and_then(Value::as_str).unwrap_or("")),
            "api_key_present": !api_key.is_empty(),
            "api_key_hash": if api_key.is_empty() {
                Value::Null
            } else {
                Value::String(crate::deterministic_receipt_hash(&json!({"id": id, "api_key": api_key})))
            },
            "cadence_hours": cadence,
            "topics": topic_rows,
            "updated_ts": now,
            "source": "manual"
        });
        row = normalize_eye(&row);
        let existing_idx = eyes.iter().position(|eye| {
            clean_id(eye.get("id").and_then(Value::as_str).unwrap_or(""), 120) == id
                || clean_text(eye.get("name").and_then(Value::as_str).unwrap_or(""), 120)
                    .eq_ignore_ascii_case(row.get("name").and_then(Value::as_str).unwrap_or(""))
        });
        let created = if let Some(idx) = existing_idx {
            let mut merged = eyes[idx].clone();
            merged["name"] = row["name"].clone();
            merged["status"] = row["status"].clone();
            if !row["endpoint_url"].as_str().unwrap_or("").is_empty() {
                merged["endpoint_url"] = row["endpoint_url"].clone();
                merged["endpoint_host"] = row["endpoint_host"].clone();
            }
            if row.get("api_key_present").and_then(Value::as_bool) == Some(true) {
                merged["api_key_present"] = Value::Bool(true);
                merged["api_key_hash"] = row["api_key_hash"].clone();
            }
            merged["cadence_hours"] = row["cadence_hours"].clone();
            merged["topics"] = row["topics"].clone();
            merged["updated_ts"] = Value::String(crate::now_iso());
            eyes[idx] = normalize_eye(&merged);
            false
        } else {
            eyes.push(row.clone());
            true
        };
        save_eyes(root, &eyes);
        let eye = eyes
            .iter()
            .find(|eye| clean_id(eye.get("id").and_then(Value::as_str).unwrap_or(""), 120) == id)
            .cloned()
            .unwrap_or(row);
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "created": created, "eye": eye}),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_snapshot() -> Value {
        json!({"ok": true})
    }

    #[test]
    fn workflow_create_rejects_unknown_target_edges() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let payload = json!({
            "name": "invalid-workflow",
            "steps": [
                {"id":"step-1","name":"step-1","mode":"sequential","next":"step-404","prompt":"{{input}}"},
                {"id":"step-2","name":"step-2","mode":"sequential","prompt":"{{input}}"}
            ]
        });
        let response = handle(
            tmp.path(),
            "POST",
            "/api/workflows",
            serde_json::to_vec(&payload).expect("encode").as_slice(),
            &empty_snapshot(),
        )
        .expect("response");
        assert_eq!(response.status, 400);
        assert_eq!(
            response.payload.get("error").and_then(Value::as_str),
            Some("workflow_validation_failed")
        );
    }

    #[test]
    fn workflow_create_and_validate_route_succeeds_for_valid_graph() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let payload = json!({
            "name": "valid-workflow",
            "steps": [
                {"id":"step-1","name":"step-1","mode":"sequential","next":"step-2","prompt":"{{input}}"},
                {"id":"step-2","name":"step-2","mode":"sequential","prompt":"{{input}}"}
            ]
        });
        let create = handle(
            tmp.path(),
            "POST",
            "/api/workflows",
            serde_json::to_vec(&payload).expect("encode").as_slice(),
            &empty_snapshot(),
        )
        .expect("create");
        assert_eq!(create.status, 200);
        let workflow_id = create
            .payload
            .pointer("/workflow/id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(!workflow_id.is_empty());

        let validate = handle(
            tmp.path(),
            "GET",
            &format!("/api/workflows/{workflow_id}/validate"),
            b"",
            &empty_snapshot(),
        )
        .expect("validate");
        assert_eq!(validate.status, 200);
        assert_eq!(
            validate
                .payload
                .pointer("/validation/valid")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
}
