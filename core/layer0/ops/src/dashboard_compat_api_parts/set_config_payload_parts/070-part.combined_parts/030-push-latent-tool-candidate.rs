
fn push_latent_tool_candidate(
    out: &mut Vec<Value>,
    seen: &mut HashSet<String>,
    lowered_message: &str,
    tool: &str,
    label: &str,
    reason: &str,
    proposed_input: Value,
) {
    let normalized = normalize_tool_name(tool);
    if normalized.is_empty() || seen.contains(&normalized) {
        return;
    }
    seen.insert(normalized.clone());
    let receipt = crate::deterministic_receipt_hash(&json!({
        "tool": normalized,
        "label": label,
        "reason": reason,
        "message": lowered_message,
        "input": proposed_input.clone()
    }));
    out.push(json!({
        "tool": normalized,
        "label": clean_text(label, 80),
        "reason": clean_text(reason, 240),
        "requires_confirmation": true,
        "proposed_input": proposed_input,
        "discovery_receipt": receipt
    }));
}
