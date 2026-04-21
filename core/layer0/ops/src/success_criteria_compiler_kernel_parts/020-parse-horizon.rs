
fn parse_horizon(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    if let Some(captures) = horizon_re().captures(&lower) {
        if let Some(m) = captures.get(1) {
            return normalize_spaces_str(m.as_str());
        }
    }
    if lower.contains("next run") {
        return "next run".to_string();
    }
    if lower.contains("next 2 runs") || lower.contains("next 2 run") {
        return "2 runs".to_string();
    }
    if lower.contains("24h") {
        return "24h".to_string();
    }
    if lower.contains("48h") {
        return "48h".to_string();
    }
    if lower.contains("7d") {
        return "7d".to_string();
    }
    String::new()
}

fn comparator_symbol(comparator: &str) -> &'static str {
    if comparator == "lte" {
        "<="
    } else {
        ">="
    }
}

fn format_count_target(text: &str, fallback: &str, suffix: &str) -> String {
    let comparator = parse_comparator(text, fallback);
    let threshold = parse_first_int(text, 1);
    format!("{}{} {suffix}", comparator_symbol(comparator), threshold)
}

fn normalize_target(metric: &str, target_text: &str, horizon_text: &str) -> String {
    let text = normalize_spaces_str(&format!("{target_text} {horizon_text}").to_ascii_lowercase());
    match metric {
        "execution_success" => "execution success".to_string(),
        "postconditions_ok" => "postconditions pass".to_string(),
        "queue_outcome_logged" => "outcome receipt logged".to_string(),
        "artifact_count" => format_count_target(&text, "gte", "artifact"),
        "outreach_artifact" => format_count_target(&text, "gte", "outreach artifact"),
        "reply_or_interview_count" => format_count_target(&text, "gte", "reply/interview signal"),
        "entries_count" => format_count_target(&text, "gte", "entries"),
        "revenue_actions_count" => format_count_target(&text, "gte", "revenue actions"),
        "token_usage" => {
            let comparator = parse_comparator(&text, "lte");
            let limit = parse_token_limit(&text).unwrap_or(1200);
            format!("tokens {}{}", comparator_symbol(comparator), limit)
        }
        "duration_ms" => {
            let comparator = parse_comparator(&text, "lte");
            let limit = parse_duration_limit_ms(&text).unwrap_or(15000);
            format!("duration {}{}ms", comparator_symbol(comparator), limit)
        }
        _ => {
            let normalized = normalize_spaces_str(target_text);
            if normalized.is_empty() {
                "execution success".to_string()
            } else {
                normalized
            }
        }
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|token| text.contains(token))
}

fn has_outreach_artifact_hint(text: &str) -> bool {
    text.contains("outreach") && contains_any(text, &["artifact", "draft", "offer", "proposal"])
}

fn classify_metric(metric_text: &str, target_text: &str, source_text: &str) -> String {
    let metric = normalize_spaces_str(metric_text).to_ascii_lowercase();
    let text = normalize_spaces_str(&format!("{metric_text} {target_text} {source_text}"))
        .to_ascii_lowercase();

    if metric.is_empty() && (text.contains("reply") || text.contains("interview")) {
        return "reply_or_interview_count".to_string();
    }
    if metric.is_empty() && has_outreach_artifact_hint(&text) {
        return "outreach_artifact".to_string();
    }

    match metric.as_str() {
        "validation_metric" | "validation_check" | "verification_metric" | "verification_check" => {
            "postconditions_ok".to_string()
        }
        "outreach_artifact" => "outreach_artifact".to_string(),
        "reply_or_interview_count"
        | "reply_count"
        | "interview_count"
        | "outreach_reply_count"
        | "outreach_interview_count" => "reply_or_interview_count".to_string(),
        "artifact_count"
        | "experiment_artifact"
        | "collector_success_runs"
        | "hypothesis_signal_lift"
        | "outreach_artifact_count"
        | "offer_draft_count"
        | "proposal_draft_count" => "artifact_count".to_string(),
        "verification_checks_passed" | "postconditions_ok" => "postconditions_ok".to_string(),
        "collector_failure_streak" | "queue_outcome_logged" => "queue_outcome_logged".to_string(),
        "entries_count" => "entries_count".to_string(),
        "revenue_actions_count" => "revenue_actions_count".to_string(),
        "latency" | "duration" | "time" | "elapsed_ms" | "elapsed" => "duration_ms".to_string(),
        "token_usage" => "token_usage".to_string(),
        "duration_ms" => "duration_ms".to_string(),
        "execution_success" => "execution_success".to_string(),
        _ => {
            if text.contains("reply") || text.contains("interview") {
                "reply_or_interview_count".to_string()
            } else if has_outreach_artifact_hint(&text) {
                "outreach_artifact".to_string()
            } else if contains_any(
                &text,
                &["artifact", "draft", "experiment", "patch", "plan", "deliverable"],
            ) {
                "artifact_count".to_string()
            } else if contains_any(
                &text,
                &["postcondition", "contract", "verify", "verification", "check pass"],
            ) {
                "postconditions_ok".to_string()
            } else if contains_any(&text, &["receipt", "evidence", "queue outcome", "logged"]) {
                "queue_outcome_logged".to_string()
            } else if text.contains("revenue") {
                "revenue_actions_count".to_string()
            } else if contains_any(&text, &["entries", "entry", "notes"]) {
                "entries_count".to_string()
            } else if text.contains("token") {
                "token_usage".to_string()
            } else if contains_any(
                &text,
                &[
                    "latency",
                    "duration",
                    "time",
                    "ms",
                    "msec",
                    "millisecond",
                    "second",
                    "sec",
                    "min",
                    "minute",
                ],
            ) {
                "duration_ms".to_string()
            } else {
                "execution_success".to_string()
            }
        }
    }
}
