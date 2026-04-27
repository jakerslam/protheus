fn continuity_command_message(payload: &Value) -> String {
    let pending_total = payload
        .get("pending_total")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let rows = payload
        .pointer("/active_agents/rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if rows.is_empty() {
        return format!("Continuity: no active agent markers. Pending total: {pending_total}.");
    }
    let mut lines = vec![format!("Continuity: pending total {pending_total}. Top active agents:")];
    for row in rows.iter().take(3) {
        let name = clean_text(
            row.get("name")
                .or_else(|| row.get("agent_id"))
                .and_then(Value::as_str)
                .unwrap_or("Agent"),
            80,
        );
        let completion = row
            .get("completion_percent")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .clamp(0, 100);
        let objective = clean_text(
            row.get("objective")
                .and_then(Value::as_str)
                .unwrap_or("No active objective."),
            160,
        );
        lines.push(format!("- {name}: {completion}% - {objective}"));
    }
    lines.join("\n")
}

fn handle_agent_scope_continuity_command_route(
    root: &Path,
    method: &str,
    segments: &[String],
    body: &[u8],
    snapshot: &Value,
    agent_id: &str,
) -> Option<CompatApiResponse> {
    if method != "POST" || segments.len() != 1 || segments[0] != "command" {
        return None;
    }
    let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
    let command = clean_text(
        request.get("command").and_then(Value::as_str).unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    if command != "continuity" {
        return None;
    }
    let silent = request
        .get("silent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let continuity = continuity_pending_payload(root, snapshot);
    let message = continuity_command_message(&continuity);
    let top_active_agents = continuity
        .pointer("/active_agents/rows")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().take(3).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    Some(CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "type": "continuity_command",
            "agent_id": agent_id,
            "command": command,
            "silent": silent,
            "message": message,
            "top_active_agents": top_active_agents,
            "continuity": continuity
        }),
    })
}
