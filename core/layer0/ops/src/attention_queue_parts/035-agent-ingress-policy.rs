#[derive(Debug, Clone)]
struct AttentionIngressDecision {
    allowed: bool,
    reason: &'static str,
    category: &'static str,
}

fn event_bool_path(event: &Value, path: &[&str]) -> Option<bool> {
    let mut current = event;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_bool()
}

fn event_str_path<'a>(event: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = event;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

fn event_text(event: &Value, key: &str) -> String {
    event
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn text_has_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn agent_attention_explicitly_visible(event: &Value) -> bool {
    if event_bool_path(event, &["attention_ingress", "agent_visible"]) == Some(true)
        || event_bool_path(event, &["attention_ingress", "allow"]) == Some(true)
        || event_bool_path(event, &["raw_event", "attention_ingress", "agent_visible"]) == Some(true)
        || event_bool_path(event, &["raw_event", "attention_ingress", "allow"]) == Some(true)
    {
        return true;
    }
    let visible_modes = ["agent_visible", "owned_actionable"];
    event_str_path(event, &["attention_visibility"])
        .or_else(|| event_str_path(event, &["raw_event", "attention_visibility"]))
        .map(|mode| visible_modes.contains(&mode.trim().to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn agent_attention_default_category(event: &Value) -> Option<&'static str> {
    let source_type = event_text(event, "source_type");
    let severity = event_text(event, "severity");
    let summary = event_text(event, "summary");
    if matches!(
        source_type.as_str(),
        "passive_memory_turn" | "chat_turn" | "conversation_turn" | "memory_event"
    ) {
        return None;
    }
    if text_has_any(&source_type, &["eval", "feedback", "chat_monitor", "issue_feedback"]) {
        return Some("eval_feedback");
    }
    if text_has_any(
        &source_type,
        &["tool", "mcp", "command_result", "workspace_result"],
    ) || text_has_any(
        &summary,
        &["tool result", "mcp", "command result", "workspace result"],
    ) {
        return Some("tooling");
    }
    if text_has_any(
        &source_type,
        &["subagent", "child_agent", "descendant", "agent_status", "agent_update", "agent_result"],
    ) {
        return Some("agent_coordination");
    }
    if text_has_any(
        &source_type,
        &["cron", "heartbeat", "automation", "scheduled_job", "schedule"],
    ) {
        return Some("owned_schedule");
    }
    if text_has_any(&source_type, &["mention", "assignment", "assigned_task", "direct_task"]) {
        return Some("direct_assignment");
    }
    if severity == "critical"
        && (text_has_any(&source_type, &["runtime", "safety", "policy", "health", "failure", "error"])
            || text_has_any(&summary, &["runtime", "safety", "policy", "health", "failure", "error"]))
    {
        return Some("critical_runtime");
    }
    None
}

fn attention_ingress_decision(
    event: &Value,
    contract: &AttentionContract,
) -> AttentionIngressDecision {
    if !contract.agent_ingress_policy_enabled {
        return AttentionIngressDecision { allowed: true, reason: "policy_disabled", category: "legacy_allow" };
    }
    if contract.agent_ingress_allow_all {
        return AttentionIngressDecision { allowed: true, reason: "agent_scoped_allow_all", category: "policy_override" };
    }
    let source = event_text(event, "source");
    if !source.starts_with("agent:") {
        return AttentionIngressDecision { allowed: true, reason: "not_agent_scoped", category: "global_event" };
    }
    if agent_attention_explicitly_visible(event) {
        return AttentionIngressDecision { allowed: true, reason: "explicit_agent_visible_opt_in", category: "explicit_opt_in" };
    }
    if let Some(category) = agent_attention_default_category(event) {
        return AttentionIngressDecision { allowed: true, reason: "owned_actionable_default", category };
    }
    AttentionIngressDecision { allowed: false, reason: "agent_scoped_event_not_owned_actionable", category: "filtered_context_noise" }
}

fn attention_ingress_json(decision: &AttentionIngressDecision) -> Value {
    json!({
        "allowed": decision.allowed,
        "reason": decision.reason,
        "category": decision.category
    })
}

fn emit_attention_ingress_drop(
    contract: &AttentionContract,
    run_context: &str,
    event: &Value,
    decision: &AttentionIngressDecision,
) {
    let (active_rows, expired_pruned) = load_active_queue(contract);
    let queue_depth = active_rows.len();
    let latest = update_latest(
        contract,
        "dropped_ingress_policy",
        queue_depth,
        None,
        expired_pruned,
    );
    let mut receipt = json!({
        "ok": true,
        "type": "attention_queue_enqueue",
        "ts": now_iso(),
        "decision": "dropped_ingress_policy",
        "queued": false,
        "run_context": run_context,
        "queue_depth_before": queue_depth,
        "queue_depth_after": queue_depth,
        "expired_pruned": expired_pruned,
        "attention_ingress": attention_ingress_json(decision),
        "attention_contract": contract_snapshot(contract),
        "event": {
            "source": event.get("source").cloned().unwrap_or_else(|| json!("unknown_source")),
            "source_type": event.get("source_type").cloned().unwrap_or_else(|| json!("unknown_type")),
            "severity": event.get("severity").cloned().unwrap_or_else(|| json!("info")),
            "summary": event.get("summary").cloned().unwrap_or_else(|| json!("attention_event")),
            "attention_key": event.get("attention_key").cloned().unwrap_or_else(|| json!(""))
        },
        "latest": latest
    });
    receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));
    append_jsonl(
        &contract.receipts_path,
        &json!({
            "ts": now_iso(),
            "type": "attention_receipt",
            "decision": "dropped_ingress_policy",
            "queued": false,
            "queue_depth_before": queue_depth,
            "queue_depth_after": queue_depth,
            "expired_pruned": expired_pruned,
            "attention_ingress_reason": decision.reason,
            "attention_ingress_category": decision.category,
            "attention_key": event.get("attention_key").cloned().unwrap_or_else(|| json!("")),
            "run_context": run_context,
            "receipt_hash": receipt.get("receipt_hash").cloned().unwrap_or_else(|| json!(""))
        }),
    );
    emit(&receipt);
}
