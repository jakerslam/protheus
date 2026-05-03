
fn infer_subagent_count_from_message(text: &str) -> usize {
    let lowered = text.to_ascii_lowercase();
    for token in lowered
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|token| !token.is_empty())
    {
        if let Ok(value) = token.parse::<usize>() {
            if value > 0 {
                return value.clamp(1, 8);
            }
        }
    }
    if lowered.contains("dozen") || lowered.contains("many") || lowered.contains("all") {
        return 5;
    }
    if lowered.contains("comprehensive") || lowered.contains("across") || lowered.contains("stress")
    {
        return 4;
    }
    if lowered.contains("parallel") || lowered.contains("swarm") || lowered.contains("subagent") {
        return 3;
    }
    2
}

fn user_requested_internal_runtime_details(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("conduit")
        || lowered.contains("cockpit")
        || lowered.contains("attention queue")
        || lowered.contains("memory lane")
        || lowered.contains("runtime lane")
        || lowered.contains("internal mechanics")
        || lowered.contains("system internals")
}

fn abstract_runtime_mechanics_terms(text: &str) -> String {
    // Workflow stabilization mode: preserve the LLM-authored visible text
    // verbatim except for mechanical transport cleanup done at call sites.
    text.to_string()
}

fn runtime_access_summary_text(runtime_summary: &Value) -> String {
    let queue_depth = parse_non_negative_i64(runtime_summary.get("queue_depth"), 0);
    let cockpit_blocks = parse_non_negative_i64(runtime_summary.get("cockpit_blocks"), 0);
    let cockpit_total_blocks =
        parse_non_negative_i64(runtime_summary.get("cockpit_total_blocks"), 0);
    let conduit_signals = parse_non_negative_i64(runtime_summary.get("conduit_signals"), 0);
    format!(
        "Current queue depth: {queue_depth}, active workers: {cockpit_blocks} ({cockpit_total_blocks} total), live signals: {conduit_signals}. Runtime status, persistent memory, and command surfaces are available."
    )
}

#[cfg(test)]
include!("../010-clean-text-tests.rs");
