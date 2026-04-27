// Layer ownership: surface/orchestration (tool-routing authority report rendering only).
use super::ToolRoutingAuthorityReport;

pub fn render_markdown(report: &ToolRoutingAuthorityReport) -> String {
    let mut lines = Vec::new();
    lines.push("# Tool Routing Authority Guard (Current)".to_string());
    lines.push(String::new());
    lines.push(format!("- pass: {}", report.ok));
    lines.push(format!(
        "- generated_at_unix_seconds: {}",
        report.generated_at_unix_seconds
    ));
    lines.push(format!(
        "- required_tool_probe_keys: {}",
        report.required_tool_probe_keys.join(", ")
    ));
    lines.push(format!(
        "- decision_trace_fields: {}",
        report.decision_trace_fields.join(", ")
    ));
    lines.push(String::new());
    render_operator_summary(report, &mut lines);
    render_failed_checks(report, &mut lines);
    render_payload_audit_failures(report, &mut lines);
    render_payload_audit(report, &mut lines);
    render_checks(report, &mut lines);
    lines.push(String::new());
    lines.join("\n")
}

fn render_operator_summary(report: &ToolRoutingAuthorityReport, lines: &mut Vec<String>) {
    let summary = &report.operator_summary;
    lines.push("## Operator Summary".to_string());
    lines.push(format!("- status: {}", summary.status));
    lines.push(format!("- total_checks: {}", summary.total_checks));
    lines.push(format!("- passing_checks: {}", summary.passing_checks));
    lines.push(format!("- failing_checks: {}", summary.failing_checks));
    lines.push(format!(
        "- planner_payload_decision_audit_failures: {}",
        summary.planner_payload_decision_audit_failures
    ));
    lines.push(format!("- json_artifact: {}", summary.json_artifact));
    lines.push(format!(
        "- markdown_artifact: {}",
        summary.markdown_artifact
    ));
    lines.push(format!(
        "- operator_next_step: {}",
        summary.operator_next_step
    ));
    lines.push(format!(
        "- top_failed_check: {}",
        summary.top_failed_check.as_deref().unwrap_or("none")
    ));
    lines.push(format!(
        "- top_failed_missing_count: {}",
        summary.top_failed_missing_count
    ));
    lines.push(format!(
        "- authority_promotion_blocked: {}",
        summary.authority_promotion_blocked
    ));
    lines.push(format!("- release_blocking: {}", summary.release_blocking));
    lines.push(String::new());
}

fn render_failed_checks(report: &ToolRoutingAuthorityReport, lines: &mut Vec<String>) {
    let failed_checks = report
        .checks
        .iter()
        .filter(|check| !check.ok)
        .collect::<Vec<_>>();
    if failed_checks.is_empty() {
        return;
    }
    lines.push("## Failed Checks (Actionable)".to_string());
    for check in &failed_checks {
        lines.push(format!(
            "- {}: missing_count={} missing={}",
            check.id,
            check.missing.len(),
            check.missing.join("; ")
        ));
    }
    lines.push(String::new());
}

fn render_payload_audit_failures(report: &ToolRoutingAuthorityReport, lines: &mut Vec<String>) {
    let failed_payload_audits = report
        .planner_payload_decision_audit
        .iter()
        .filter(|row| !row.ok)
        .collect::<Vec<_>>();
    if failed_payload_audits.is_empty() {
        return;
    }
    lines.push("## Planner Payload Audit Failures".to_string());
    for row in &failed_payload_audits {
        lines.push(format!(
            "- {} [{}]: payload_read_count={} legacy_only={}",
            row.path, row.decision_scope, row.payload_read_count, row.legacy_only
        ));
    }
    lines.push(String::new());
}

fn render_payload_audit(report: &ToolRoutingAuthorityReport, lines: &mut Vec<String>) {
    lines.push("## Planner Payload Decision Audit".to_string());
    for row in &report.planner_payload_decision_audit {
        lines.push(format!(
            "- {} [{}]: ok={} payload_read_count={} legacy_only={}",
            row.path, row.decision_scope, row.ok, row.payload_read_count, row.legacy_only
        ));
    }
    lines.push(String::new());
}

fn render_checks(report: &ToolRoutingAuthorityReport, lines: &mut Vec<String>) {
    lines.push("## Checks".to_string());
    for check in &report.checks {
        lines.push(format!(
            "- {}: ok={} evidence={} missing_count={} missing={}",
            check.id,
            check.ok,
            check.evidence.join("; "),
            check.missing.len(),
            if check.missing.is_empty() {
                "none".to_string()
            } else {
                check.missing.join("; ")
            }
        ));
    }
}
