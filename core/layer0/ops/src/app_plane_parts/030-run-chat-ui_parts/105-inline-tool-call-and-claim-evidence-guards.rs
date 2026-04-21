fn chat_ui_repair_inline_tool_call_payload(raw_payload: &str) -> Option<String> {
    let trimmed = clean(raw_payload, 4_000).trim().to_string();
    if trimmed.is_empty() {
        return None;
    }
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        return None;
    }
    let open_curly = trimmed.chars().filter(|ch| *ch == '{').count() as i64;
    let close_curly = trimmed.chars().filter(|ch| *ch == '}').count() as i64;
    let open_square = trimmed.chars().filter(|ch| *ch == '[').count() as i64;
    let close_square = trimmed.chars().filter(|ch| *ch == ']').count() as i64;
    if close_curly > open_curly || close_square > open_square {
        return None;
    }
    let mut repaired = trimmed.clone();
    for _ in 0..(open_square - close_square) {
        repaired.push(']');
    }
    for _ in 0..(open_curly - close_curly) {
        repaired.push('}');
    }
    if repaired == trimmed {
        None
    } else {
        Some(repaired)
    }
}

fn chat_ui_inline_tool_call_schema(raw_response: &str) -> Value {
    let cleaned = clean(raw_response, 16_000);
    let lowered = cleaned.to_ascii_lowercase();
    let marker = "<function=";
    let Some(marker_idx) = lowered.find(marker) else {
        return json!({
            "detected": false
        });
    };
    let remainder = &cleaned[(marker_idx + marker.len())..];
    let split_idx = remainder.find('>').unwrap_or(remainder.len());
    let raw_tool = clean(&remainder[..split_idx], 120);
    let tool = clean(
        raw_tool.trim_matches(|ch: char| ch == '"' || ch == '\'' || ch.is_ascii_whitespace()),
        80,
    );
    let payload_segment = if split_idx < remainder.len() {
        &remainder[(split_idx + 1)..]
    } else {
        ""
    };
    let mut payload = clean(payload_segment, 4_000);
    let payload_lowered = payload.to_ascii_lowercase();
    if let Some(close_idx) = payload_lowered.find("</function>") {
        payload = payload[..close_idx].to_string();
    }
    let payload = clean(payload.trim(), 4_000);
    let mut normalized_payload = payload.clone();
    let mut schema_valid = false;
    let mut schema_repaired = false;
    let mut args = Value::Null;
    if !payload.is_empty() {
        if let Ok(parsed) = serde_json::from_str::<Value>(&payload) {
            schema_valid = true;
            args = parsed;
        } else if let Some(repaired_payload) = chat_ui_repair_inline_tool_call_payload(&payload) {
            if let Ok(parsed) = serde_json::from_str::<Value>(&repaired_payload) {
                schema_valid = true;
                schema_repaired = true;
                normalized_payload = repaired_payload;
                args = parsed;
            }
        }
    }
    json!({
        "detected": true,
        "tool": if tool.is_empty() { Value::Null } else { json!(tool) },
        "payload": if normalized_payload.is_empty() { Value::Null } else { json!(normalized_payload) },
        "schema_valid": schema_valid,
        "schema_repaired": schema_repaired,
        "args": args
    })
}

fn chat_ui_contains_unverified_routing_root_cause_claim(response_text: &str) -> bool {
    let lowered = clean(response_text, 8_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    [
        "system is automatically triggering",
        "automatically triggering web searches",
        "backend automation that bypasses",
        "bypasses my conscious tool selection",
        "external systems fire tools independently",
        "automatic tool calls without my conscious selection",
        "fundamental design flaw",
    ]
    .iter()
    .any(|marker| lowered.contains(*marker))
}

fn chat_ui_has_structured_routing_claim_evidence(rows: &[Value]) -> bool {
    rows.iter().any(|row| {
        if row
            .get("auto_triggered")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return true;
        }
        let source = clean(
            row.get("source")
                .or_else(|| row.pointer("/diagnostics/source"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if source.contains("auto_trigger") || source.contains("automation") {
            return true;
        }
        let selection_authority = clean(
            row.get("selection_authority")
                .or_else(|| row.pointer("/gate/tool_selection_authority"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if selection_authority == "system_auto"
            || selection_authority == "automatic"
            || selection_authority == "backend_auto"
        {
            return true;
        }
        let error = clean(
            row.get("error")
                .or_else(|| row.pointer("/result/error"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        )
        .to_ascii_lowercase();
        error.contains("tool_auto_triggered")
            || error.contains("automatic_tool_trigger")
            || error.contains("backend_routing_override")
    })
}
