
fn inspect_state(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let checkpoint_id = clean_token(payload.get("checkpoint_id").and_then(Value::as_str), "");
    let checkpoint = if checkpoint_id.is_empty() {
        None
    } else {
        state
            .get("checkpoints")
            .and_then(Value::as_object)
            .and_then(|rows| rows.get(&checkpoint_id))
            .cloned()
    };
    let graph_id = clean_token(
        payload.get("graph_id").and_then(Value::as_str).or_else(|| {
            checkpoint
                .as_ref()
                .and_then(|row| row.get("graph_id"))
                .and_then(Value::as_str)
        }),
        "",
    );
    if graph_id.is_empty() {
        return Err("workflow_graph_inspection_graph_or_checkpoint_required".to_string());
    }
    let state_view = checkpoint
        .as_ref()
        .and_then(|row| row.get("snapshot"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let intervention = payload
        .get("intervention_patch")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let inspection = json!({

        "inspection_id": stable_id("lginspect", &json!({"graph_id": graph_id, "checkpoint_id": checkpoint_id, "state": state_view})),
        "graph_id": graph_id,
        "checkpoint_id": if checkpoint_id.is_empty() { json!(null) } else { json!(checkpoint_id) },
        "operator_id": clean_token(payload.get("operator_id").and_then(Value::as_str), "operator"),
        "inspection_mode": if intervention.as_object().map(|row| !row.is_empty()).unwrap_or(false) { json!("intervened") } else { json!("inspect_only") },
        "view_fields": payload.get("view_fields").cloned().unwrap_or_else(|| json!([])),
        "state_view": state_view,
        "intervention_patch": intervention,
        "change_applied": payload.get("intervention_patch").and_then(Value::as_object).map(|row| !row.is_empty()).unwrap_or(false),
        "inspected_at": now_iso(),
    });
    let inspection_id = inspection
        .get("inspection_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "inspections").insert(inspection_id, inspection.clone());
    Ok(json!({
        "ok": true,
        "inspection": inspection,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.3", semantic_claim("V6-WORKFLOW-002.3")),
    }))
}

fn interrupt_run(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let checkpoint_id = clean_token(payload.get("checkpoint_id").and_then(Value::as_str), "");
    if checkpoint_id.is_empty() {
        return Err("workflow_graph_interrupt_checkpoint_id_required".to_string());
    }
    let checkpoint = state
        .get("checkpoints")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&checkpoint_id))
        .cloned()
        .ok_or_else(|| format!("unknown_workflow_graph_checkpoint:{checkpoint_id}"))?;
    let graph_id = clean_token(
        checkpoint.get("graph_id").and_then(Value::as_str),
        "workflow_graph-graph",
    );
    let reason = clean_text(payload.get("reason").and_then(Value::as_str), 160);
    let interrupt = json!({
        "interrupt_id": stable_id("lginterrupt", &json!({"checkpoint_id": checkpoint_id, "reason": reason})),
        "graph_id": graph_id,
        "checkpoint_id": checkpoint_id,
        "thread_id": checkpoint.get("thread_id").cloned().unwrap_or_else(|| json!(null)),
        "resume_token": stable_id("lgresume", &json!({"checkpoint_id": checkpoint_id, "reason": reason})),
        "requested_by": clean_token(payload.get("requested_by").and_then(Value::as_str), "operator"),
        "reason": reason,
        "snapshot": checkpoint.get("snapshot").cloned().unwrap_or_else(|| json!({})),
        "status": "paused",
        "created_at": now_iso(),
    });
    let interrupt_id = interrupt
        .get("interrupt_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "interrupts").insert(interrupt_id, interrupt.clone());
    Ok(json!({
        "ok": true,
        "interrupt": interrupt,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.7", semantic_claim("V6-WORKFLOW-002.7")),
    }))
}

fn find_interrupt_key(
    interrupts: &Map<String, Value>,
    interrupt_id: &str,
    resume_token: &str,
) -> Option<String> {
    if !interrupt_id.is_empty() && interrupts.contains_key(interrupt_id) {
        return Some(interrupt_id.to_string());
    }
    if resume_token.is_empty() {
        return None;
    }
    interrupts.iter().find_map(|(id, row)| {
        (row.get("resume_token").and_then(Value::as_str) == Some(resume_token))
            .then(|| id.to_string())
    })
}

fn resume_run(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let interrupt_id = clean_token(payload.get("interrupt_id").and_then(Value::as_str), "");
    let resume_token = clean_token(payload.get("resume_token").and_then(Value::as_str), "");
    if interrupt_id.is_empty() && resume_token.is_empty() {
        return Err("workflow_graph_resume_interrupt_or_token_required".to_string());
    }
    let key = {
        let interrupts = state
            .get("interrupts")
            .and_then(Value::as_object)
            .ok_or_else(|| "workflow_graph_interrupt_store_missing".to_string())?;
        find_interrupt_key(interrupts, &interrupt_id, &resume_token)
            .ok_or_else(|| "workflow_graph_interrupt_not_found".to_string())?
    };
    let updated = {
        let interrupts = as_object_mut(state, "interrupts");
        let row = interrupts
            .get_mut(&key)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "workflow_graph_interrupt_record_invalid".to_string())?;
        if row
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| status != "paused")
        {
            return Err("workflow_graph_interrupt_not_paused".to_string());
        }
        row.insert("status".to_string(), json!("resumed"));
        row.insert("resumed_at".to_string(), json!(now_iso()));
        row.insert(
            "resume_mode".to_string(),
            json!(clean_token(
                payload.get("resume_mode").and_then(Value::as_str),
                "continue",
            )),
        );
        row.insert(
            "resume_context".to_string(),
            payload
                .get("resume_context")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        Value::Object(row.clone())
    };
    Ok(json!({
        "ok": true,
        "interrupt": updated,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.7", semantic_claim("V6-WORKFLOW-002.7")),
    }))
}
