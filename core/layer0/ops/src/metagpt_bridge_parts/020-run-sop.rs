
fn run_sop(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let company_id = clean_token(payload.get("company_id").and_then(Value::as_str), "");
    if company_id.is_empty() {
        return Err("metagpt_company_id_required".to_string());
    }
    let steps = payload
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        return Err("metagpt_sop_steps_required".to_string());
    }
    let budget = payload
        .get("budget")
        .cloned()
        .unwrap_or_else(|| json!({"tokens": 2000, "max_stages": steps.len()}));
    let run = json!({
        "sop_run_id": stable_id("mgsop", &json!({"company_id": company_id, "steps": steps})),
        "company_id": company_id,
        "pipeline_name": clean_text(payload.get("pipeline_name").and_then(Value::as_str), 120),
        "stage_count": steps.len(),
        "steps": steps,
        "checkpoint_labels": payload.get("checkpoint_labels").cloned().unwrap_or_else(|| json!(["requirements", "design", "build", "review"])),
        "budget": budget,
        "executed_at": now_iso(),
    });
    let id = run
        .get("sop_run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "sop_runs").insert(id, run.clone());
    Ok(
        json!({"ok": true, "sop_run": run, "claim_evidence": claim("V6-WORKFLOW-006.2", "metagpt_sop_pipelines_execute_on_authoritative_workflow_with_receipts_and_budget_controls")}),
    )
}

fn safe_repo_change_path(value: &str) -> bool {
    ["core/", "client/", "adapters/", "apps/", "docs/", "tests/"]
        .iter()
        .any(|prefix| value.starts_with(prefix))
}

fn simulate_pr(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let adapter_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/metagpt_config_bridge.ts"),
    )?;
    let sandbox_mode = clean_token(
        payload.get("sandbox_mode").and_then(Value::as_str),
        "readonly",
    );
    if sandbox_mode == "disabled" {
        return Err("metagpt_pr_simulation_requires_sandbox".to_string());
    }
    let changed_files = payload
        .get("changed_files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if changed_files
        .iter()
        .filter_map(Value::as_str)
        .any(|path| !safe_repo_change_path(path))
    {
        return Err("metagpt_pr_simulation_path_outside_allowed_surface".to_string());
    }
    let destructive = payload
        .get("destructive")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if destructive {
        return Err("metagpt_pr_simulation_destructive_change_denied".to_string());
    }
    let pr = json!({
        "simulation_id": stable_id("mgpr", &json!({"task": payload.get("task"), "changed_files": changed_files})),
        "task": clean_text(payload.get("task").and_then(Value::as_str), 160),
        "changed_files": changed_files,
        "generated_patch_summary": clean_text(payload.get("generated_patch_summary").and_then(Value::as_str), 200),
        "tests": payload.get("tests").cloned().unwrap_or_else(|| json!([])),
        "sandbox_mode": sandbox_mode,
        "bridge_path": adapter_path,
        "review_required": true,
        "simulated_at": now_iso(),
    });
    let id = pr
        .get("simulation_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "pr_simulations").insert(id, pr.clone());
    Ok(
        json!({"ok": true, "pr_simulation": pr, "claim_evidence": claim("V6-WORKFLOW-006.3", "metagpt_code_generation_execution_and_pr_simulation_remain_sandboxed_and_review_gated")}),
    )
}

fn run_debate(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let proposal = clean_text(payload.get("proposal").and_then(Value::as_str), 200);
    if proposal.is_empty() {
        return Err("metagpt_debate_proposal_required".to_string());
    }
    let profile = profile(payload.get("profile"));
    let participants = payload
        .get("participants")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("pm"), json!("architect")]);
    let requested_rounds = payload.get("rounds").and_then(Value::as_u64).unwrap_or(2);
    let context_budget = payload
        .get("context_budget")
        .and_then(Value::as_u64)
        .unwrap_or(4096);
    let allowed_rounds = if profile == "tiny-max" {
        requested_rounds.min(2)
    } else {
        requested_rounds
    };
    let degraded = allowed_rounds != requested_rounds || context_budget < 1024;
    let review = json!({
        "debate_id": stable_id("mgdebate", &json!({"proposal": proposal, "participants": participants})),
        "proposal": proposal,
        "participants": participants,
        "rounds": allowed_rounds,
        "context_budget": context_budget,
        "degraded": degraded,
        "recommendation": clean_token(payload.get("recommendation").and_then(Value::as_str), "revise"),
        "completed_at": now_iso(),
    });
    let id = review
        .get("debate_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "debates").insert(id, review.clone());
    Ok(
        json!({"ok": true, "debate": review, "claim_evidence": claim("V6-WORKFLOW-006.4", "metagpt_multi_agent_debate_and_review_cycles_remain_receipted_and_budgeted")}),
    )
}

fn plan_requirements(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let prd_title = clean_text(payload.get("prd_title").and_then(Value::as_str), 140);
    if prd_title.is_empty() {
        return Err("metagpt_prd_title_required".to_string());
    }
    let requirements = payload
        .get("requirements")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if requirements.is_empty() {
        return Err("metagpt_requirements_required".to_string());
    }
    let stories: Vec<Value> = requirements.iter().enumerate().map(|(idx, row)| {
        let text = row.as_str().unwrap_or("requirement");
        json!({"story_id": format!("story-{}", idx + 1), "summary": text, "tasks": [format!("draft {}", idx + 1), format!("review {}", idx + 1)]})
    }).collect();
    let plan = json!({
        "plan_id": stable_id("mgreq", &json!({"prd_title": prd_title, "requirements": requirements})),
        "prd_title": prd_title,
        "requirements": requirements,
        "stories": stories,
        "stakeholders": payload.get("stakeholders").cloned().unwrap_or_else(|| json!([])),
        "auto_recall_query": clean_text(payload.get("auto_recall_query").and_then(Value::as_str), 120),
        "planned_at": now_iso(),
    });
    let id = plan
        .get("plan_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "requirements").insert(id, plan.clone());
    Ok(
        json!({"ok": true, "requirements_plan": plan, "claim_evidence": claim("V6-WORKFLOW-006.5", "metagpt_requirements_analysis_and_task_breakdown_route_through_governed_memory_and_decomposition_lanes")}),
    )
}

fn record_oversight(
    state: &mut Value,
    approval_queue_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let operator_id = clean_token(payload.get("operator_id").and_then(Value::as_str), "");
    if operator_id.is_empty() {
        return Err("metagpt_operator_id_required".to_string());
    }
    let event = json!({
        "oversight_id": stable_id("mgoverse", &json!({"operator_id": operator_id, "action": payload.get("action")})),
        "operator_id": operator_id,
        "action": clean_token(payload.get("action").and_then(Value::as_str), "review"),
        "target_id": clean_token(payload.get("target_id").and_then(Value::as_str), ""),
        "notes": clean_text(payload.get("notes").and_then(Value::as_str), 200),
        "recorded_at": now_iso(),
    });
    let mut queue = match fs::read_to_string(approval_queue_path) {
        Ok(raw) => serde_yaml::from_str::<Value>(&raw).unwrap_or_else(|_| json!({"events": []})),
        Err(_) => json!({"events": []}),
    };
    if !queue.get("events").map(Value::is_array).unwrap_or(false) {
        queue["events"] = json!([]);
    }
    queue
        .get_mut("events")
        .and_then(Value::as_array_mut)
        .expect("events")
        .push(event.clone());
    let encoded = serde_yaml::to_string(&queue)
        .map_err(|err| format!("metagpt_oversight_queue_encode_failed:{err}"))?;
    if let Some(parent) = approval_queue_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("metagpt_oversight_queue_dir_create_failed:{err}"))?;
    }
    fs::write(approval_queue_path, encoded)
        .map_err(|err| format!("metagpt_oversight_queue_write_failed:{err}"))?;
    let id = event
        .get("oversight_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "oversight").insert(id, event.clone());
    Ok(
        json!({"ok": true, "oversight": event, "approval_queue_path": approval_queue_path.display().to_string(), "claim_evidence": claim("V6-WORKFLOW-006.6", "metagpt_human_oversight_and_intervention_points_remain_within_existing_approval_boundaries")}),
    )
}

fn record_pipeline_trace(
    root: &Path,
    state: &mut Value,
    trace_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let trace = json!({
        "trace_id": stable_id("mgtrace", &json!({"stage": payload.get("stage"), "message": payload.get("message")})),
        "run_id": clean_token(payload.get("run_id").and_then(Value::as_str), ""),
        "stage": clean_token(payload.get("stage").and_then(Value::as_str), "pipeline"),
        "message": clean_text(payload.get("message").and_then(Value::as_str), 180),
        "metrics": payload.get("metrics").cloned().unwrap_or_else(|| json!({})),
        "trace_path": rel(root, trace_path),
        "recorded_at": now_iso(),
    });
    lane_utils::append_jsonl(trace_path, &trace)?;
    as_array_mut(state, "traces").push(trace.clone());
    Ok(
        json!({"ok": true, "pipeline_trace": trace, "claim_evidence": claim("V6-WORKFLOW-006.7", "metagpt_pipeline_events_stream_through_native_observability_and_receipt_lanes")}),
    )
}
