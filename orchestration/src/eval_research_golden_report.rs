use super::eval_research_golden_utils::*;
use serde_json::{json, Value};

pub(super) fn append_failure_events(
    events: &mut Vec<Value>,
    case_id: &str,
    prompt: &str,
    agent_id: &str,
    live: bool,
    response_text: &str,
    failures: &[String],
    setup_failures: &[String],
) {
    for reason in failures.iter().chain(setup_failures.iter()) {
        events.push(json!({
            "type": "research_golden_failure",
            "case_id": case_id,
            "reason": reason,
            "prompt_preview": clean_text(prompt, 240),
            "response_preview": clean_text(response_text, 320),
            "agent_id": agent_id,
            "live": live,
            "generated_at": now_iso_like(),
        }));
    }
}

pub(super) fn markdown_report(report: &Value) -> String {
    let summary = report.get("summary").unwrap_or(&Value::Null);
    let mut out = format!(
        "# Research Golden Eval (Current)\n\n- generated_at: {}\n- ok: {}\n- mode: {}\n- cases: {}\n- average_score: {:.3}\n- research_success_rate: {:.3}\n- gate_path_ok: {}\n- gate_transition_path_ok: {}\n- safety_ok: {}\n- failure_count: {}\n\n",
        str_at(report, &["generated_at"], ""),
        bool_at(report, &["ok"], false),
        str_at(report, &["mode"], ""),
        u64_at(summary, &["cases"], 0),
        f64_at(summary, &["average_score"], 0.0),
        f64_at(summary, &["research_success_rate"], 0.0),
        bool_at(summary, &["gate_path_ok"], false),
        bool_at(summary, &["gate_transition_path_ok"], false),
        bool_at(summary, &["safety_ok"], false),
        u64_at(summary, &["failure_count"], 0),
    );
    out.push_str("## Gate Pass Rates\n\n");
    if let Some(rows) = report
        .get("workflow_gate_pass_rates")
        .and_then(Value::as_array)
    {
        for row in rows {
            out.push_str(&format!(
                "- {}: {}/{} ({:.3}) ok={}\n",
                str_at(row, &["gate"], "unknown"),
                u64_at(row, &["passed"], 0),
                u64_at(row, &["total"], 0),
                f64_at(row, &["pass_rate"], 0.0),
                bool_at(row, &["ok"], false)
            ));
        }
    }
    out.push_str("\n## Gate Transition Diagnostics\n\n");
    if let Some(rows) = report
        .get("gate_transition_pass_rates")
        .and_then(Value::as_array)
    {
        for row in rows {
            out.push_str(&format!(
                "- {}: {}/{} ({:.3})\n",
                str_at(row, &["checkpoint"], "unknown"),
                u64_at(row, &["passed"], 0),
                u64_at(row, &["total"], 0),
                f64_at(row, &["pass_rate"], 0.0),
            ));
        }
    }
    out.push_str("\n## Category Pass Rates\n\n");
    if let Some(rows) = report.get("category_pass_rates").and_then(Value::as_array) {
        for row in rows {
            out.push_str(&format!(
                "- {}: {}/{} ({:.3}) excellent={}/{}\n",
                str_at(row, &["category"], "unknown"),
                u64_at(row, &["passed"], 0),
                u64_at(row, &["total"], 0),
                f64_at(row, &["pass_rate"], 0.0),
                u64_at(row, &["excellent"], 0),
                u64_at(row, &["total"], 0),
            ));
        }
    }
    out.push_str("\n## Tag Pass Rates\n\n");
    if let Some(rows) = report.get("tag_pass_rates").and_then(Value::as_array) {
        for row in rows {
            out.push_str(&format!(
                "- {}: {}/{} ({:.3}) excellent={}/{}\n",
                str_at(row, &["tag"], "unknown"),
                u64_at(row, &["passed"], 0),
                u64_at(row, &["total"], 0),
                f64_at(row, &["pass_rate"], 0.0),
                u64_at(row, &["excellent"], 0),
                u64_at(row, &["total"], 0),
            ));
        }
    }
    let split = report.get("measurement_split").unwrap_or(&Value::Null);
    out.push_str("\n## Measurement Split\n\n");
    out.push_str(&format!(
        "- deterministic_workflow_path: ok={} hard_failure_cases={}\n",
        bool_at(split, &["deterministic_workflow_path", "ok"], false),
        u64_at(
            split,
            &["deterministic_workflow_path", "hard_failure_cases"],
            0
        )
    ));
    out.push_str(&format!(
        "- live_retrieval_health: status={} tool_execution_rate={:.3} evidence_context_rate={:.3}\n",
        str_at(split, &["live_retrieval_health", "status"], "unknown"),
        f64_at(
            split,
            &["live_retrieval_health", "tool_execution_rate"],
            0.0
        ),
        f64_at(
            split,
            &["live_retrieval_health", "evidence_context_rate"],
            0.0
        )
    ));
    out.push_str(&format!(
        "- end_to_end_golden: mode={} success_rate={:.3} soft_failure_cases={}\n",
        str_at(split, &["end_to_end_golden", "mode"], "unknown"),
        f64_at(split, &["end_to_end_golden", "research_success_rate"], 0.0),
        u64_at(split, &["failure_classification", "soft_failure_cases"], 0)
    ));
    let lifecycle = report.get("observation_lifecycle").unwrap_or(&Value::Null);
    out.push_str("\n## Observation Lifecycle\n\n");
    out.push_str(&format!(
        "- enabled={} ok={} events_recorded={}\n",
        bool_at(lifecycle, &["enabled"], false),
        bool_at(lifecycle, &["ok"], false),
        u64_at(lifecycle, &["events_recorded"], 0)
    ));
    out.push_str(&format!(
        "- archive: open_subjects={} archived_subjects={} reemerged_subjects={}\n",
        u64_at(lifecycle, &["summary", "archive", "open_subjects"], 0),
        u64_at(lifecycle, &["summary", "archive", "archived_subjects"], 0),
        u64_at(lifecycle, &["summary", "archive", "reemerged_subjects"], 0)
    ));
    out.push_str("\n## Lowest Cases\n\n");
    let mut case_rows = report
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    case_rows.sort_by_key(|row| u64_at(row, &["score"], 0));
    for row in case_rows.iter().take(8) {
        out.push_str(&format!(
            "- {}: score={} score_pass={} lifecycle_complete={} pass={} failures={}\n",
            str_at(row, &["case_id"], "unknown"),
            u64_at(row, &["score"], 0),
            bool_at(row, &["score_pass"], false),
            bool_at(row, &["lifecycle_gate_path_complete"], false),
            bool_at(row, &["pass"], false),
            row.get("failures")
                .and_then(Value::as_array)
                .map(|failures| failures
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", "))
                .unwrap_or_default()
        ));
    }
    out
}
