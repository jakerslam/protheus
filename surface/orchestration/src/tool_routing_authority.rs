// Layer ownership: surface/orchestration (tool-routing authority evidence only).
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const TOOL_ROUTING_AUTHORITY_ARTIFACT_JSON: &str =
    "core/local/artifacts/tool_routing_authority_guard_current.json";
pub const TOOL_ROUTING_AUTHORITY_ARTIFACT_MARKDOWN: &str =
    "local/workspace/reports/TOOL_ROUTING_AUTHORITY_GUARD_CURRENT.md";

pub const REQUIRED_TOOL_PROBE_KEYS: [&str; 5] = [
    "workspace_read",
    "workspace_search",
    "web_search",
    "web_fetch",
    "tool_route",
];

pub const DECISION_TRACE_FIELDS: [&str; 4] = ["selected", "rejected", "reason", "confidence"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRoutingAuthorityCheck {
    pub id: String,
    pub ok: bool,
    pub evidence: Vec<String>,
    pub missing: Vec<String>,
}

impl ToolRoutingAuthorityCheck {
    fn new(id: &str, evidence: Vec<String>, missing: Vec<String>) -> Self {
        Self {
            id: id.to_string(),
            ok: missing.is_empty(),
            evidence,
            missing,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannerPayloadDecisionAuditRow {
    pub path: String,
    pub decision_scope: String,
    pub payload_read_count: u64,
    pub legacy_only: bool,
    pub ok: bool,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolRoutingAuthorityReport {
    pub schema_id: String,
    pub schema_version: u64,
    pub generated_at_unix_seconds: u64,
    pub ok: bool,
    pub required_tool_probe_keys: Vec<String>,
    pub decision_trace_fields: Vec<String>,
    pub planner_payload_decision_audit: Vec<PlannerPayloadDecisionAuditRow>,
    pub summary: BTreeMap<String, u64>,
    pub checks: Vec<ToolRoutingAuthorityCheck>,
}

pub fn build_tool_routing_authority_report(root: impl AsRef<Path>) -> ToolRoutingAuthorityReport {
    let root = root.as_ref();
    let contracts = read_text(root, "surface/orchestration/src/contracts.rs");
    let preconditions = read_text(root, "surface/orchestration/src/planner/preconditions.rs");
    let planner_candidates = read_text(root, "surface/orchestration/src/planner/plan_candidates.rs");
    let planner_common = read_text(
        root,
        "surface/orchestration/src/planner/plan_candidates/common.rs",
    );
    let planner_chain = read_text(
        root,
        "surface/orchestration/src/planner/plan_candidates/chain.rs",
    );
    let planner_strategy = read_text(
        root,
        "surface/orchestration/src/planner/plan_candidates/strategy.rs",
    );
    let probe_matrix = read_text(
        root,
        "surface/orchestration/tests/conformance/probe_matrix.rs",
    );
    let route_guard = read_text(
        root,
        "tests/tooling/scripts/ci/tool_route_misdirection_guard.ts",
    );
    let route_fixture = read_text(
        root,
        "tests/tooling/fixtures/tool_route_misdirection_matrix.json",
    );
    let package_json = read_text(root, "package.json");
    let planner_payload_decision_audit = planner_payload_decision_audit(
        &preconditions,
        &planner_candidates,
        &planner_common,
        &planner_chain,
        &planner_strategy,
    );

    let checks = vec![
        required_probe_keys_declared(&contracts, &preconditions),
        typed_surfaces_fail_closed_on_missing_probes(&preconditions),
        generic_execute_tool_not_authoritative(&contracts, &preconditions),
        specific_missing_probe_diagnostics_declared(&preconditions),
        decision_trace_shape_declared(&preconditions),
        decision_trace_regressions_declared(&probe_matrix),
        planner_payload_decision_audit_enforced(&planner_payload_decision_audit),
        route_misdirection_regression_declared(&route_guard, &route_fixture),
        package_scripts_registered(&package_json),
    ];

    let failing_checks = checks.iter().filter(|row| !row.ok).count() as u64;
    let mut summary = BTreeMap::new();
    summary.insert("total_checks".to_string(), checks.len() as u64);
    summary.insert(
        "passing_checks".to_string(),
        checks.len() as u64 - failing_checks,
    );
    summary.insert("failing_checks".to_string(), failing_checks);
    summary.insert(
        "required_tool_probe_key_count".to_string(),
        REQUIRED_TOOL_PROBE_KEYS.len() as u64,
    );
    summary.insert(
        "decision_trace_field_count".to_string(),
        DECISION_TRACE_FIELDS.len() as u64,
    );
    summary.insert(
        "planner_payload_decision_audit_rows".to_string(),
        planner_payload_decision_audit.len() as u64,
    );
    summary.insert(
        "planner_payload_decision_audit_failures".to_string(),
        planner_payload_decision_audit
            .iter()
            .filter(|row| !row.ok)
            .count() as u64,
    );

    ToolRoutingAuthorityReport {
        schema_id: "tool_routing_authority_guard".to_string(),
        schema_version: 1,
        generated_at_unix_seconds: generated_at_unix_seconds(),
        ok: failing_checks == 0,
        required_tool_probe_keys: REQUIRED_TOOL_PROBE_KEYS
            .iter()
            .map(|row| row.to_string())
            .collect(),
        decision_trace_fields: DECISION_TRACE_FIELDS
            .iter()
            .map(|row| row.to_string())
            .collect(),
        planner_payload_decision_audit,
        summary,
        checks,
    }
}

pub fn write_tool_routing_authority_artifacts(
    root: impl AsRef<Path>,
    json_path: &str,
    markdown_path: &str,
) -> Result<ToolRoutingAuthorityReport, String> {
    let root = root.as_ref();
    let report = build_tool_routing_authority_report(root);
    write_text(
        root,
        json_path,
        &serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?,
    )?;
    write_text(root, markdown_path, &render_markdown(&report))?;
    Ok(report)
}

fn required_probe_keys_declared(contracts: &str, preconditions: &str) -> ToolRoutingAuthorityCheck {
    let combined = format!("{contracts}\n{preconditions}");
    let required = REQUIRED_TOOL_PROBE_KEYS
        .iter()
        .map(|key| format!("\"{key}\""))
        .collect::<Vec<_>>();
    token_check(
        "required_tool_probe_keys_declared",
        &combined,
        &required,
        vec![
            "Capability::probe_keys declares one canonical key per tool family".to_string(),
            "planner preconditions route by capability-owned probe key".to_string(),
        ],
    )
}

fn typed_surfaces_fail_closed_on_missing_probes(preconditions: &str) -> ToolRoutingAuthorityCheck {
    token_check(
        "typed_surfaces_fail_closed_on_missing_probes",
        preconditions,
        &[
            "fail_closed_on_missing_probe_for_typed_surface".to_string(),
            "RequestSurface::Legacy".to_string(),
            "authoritative_probe_required".to_string(),
            "missing_probe_field_source".to_string(),
        ],
        vec![
            "non-legacy surfaces return missing_probe before payload shortcuts or heuristics"
                .to_string(),
        ],
    )
}

fn generic_execute_tool_not_authoritative(
    contracts: &str,
    preconditions: &str,
) -> ToolRoutingAuthorityCheck {
    let normalized_contracts = normalize_source(contracts);
    let missing = [
        "Capability::ExecuteTool=>&[\"tool_route\"]",
        "\"execute_tool\"",
    ]
    .iter()
    .filter_map(|token| {
        if *token == "\"execute_tool\"" {
            if preconditions.contains(token) {
                Some("typed routing must not require generic execute_tool".to_string())
            } else {
                None
            }
        } else if normalized_contracts.contains(token) {
            None
        } else {
            Some(format!("missing token: {token}"))
        }
    })
    .collect::<Vec<_>>();
    ToolRoutingAuthorityCheck::new(
        "generic_execute_tool_not_authoritative",
        vec![
            "legacy ExecuteTool compatibility maps to tool_route".to_string(),
            "typed preconditions avoid execute_tool as a required probe key".to_string(),
        ],
        missing,
    )
}

fn specific_missing_probe_diagnostics_declared(preconditions: &str) -> ToolRoutingAuthorityCheck {
    token_check(
        "specific_missing_probe_diagnostics_declared",
        preconditions,
        &[
            "missing_probe_source".to_string(),
            "missing_probe_field_source".to_string(),
            "missing_probe: {}".to_string(),
            "missing_probe: {}.{}".to_string(),
        ],
        vec!["failures name the exact missing probe key/field".to_string()],
    )
}

fn decision_trace_shape_declared(preconditions: &str) -> ToolRoutingAuthorityCheck {
    let required = DECISION_TRACE_FIELDS
        .iter()
        .map(|field| format!("\"{field}\""))
        .chain(["deterministic_routing_decision_trace".to_string()])
        .collect::<Vec<_>>();
    token_check(
        "decision_trace_shape_declared",
        preconditions,
        &required,
        vec![
            "tool routing emits selected path, rejected alternatives, reason, and confidence"
                .to_string(),
        ],
    )
}

fn decision_trace_regressions_declared(probe_matrix: &str) -> ToolRoutingAuthorityCheck {
    token_check(
        "decision_trace_regressions_declared",
        probe_matrix,
        &[
            "non_legacy_tool_routing_requires_authoritative_probe_even_when_unadapted".to_string(),
            "routing_decision_trace_records_selected_rejected_reason_and_confidence".to_string(),
            "missing_probe: web_search.tool_available".to_string(),
            "probe.core_probe_envelope.workspace_search.tool_available".to_string(),
        ],
        vec!["conformance tests lock missing-probe and trace behavior".to_string()],
    )
}

fn route_misdirection_regression_declared(
    route_guard: &str,
    route_fixture: &str,
) -> ToolRoutingAuthorityCheck {
    let combined = format!("{route_guard}\n{route_fixture}");
    token_check(
        "route_misdirection_regression_declared",
        &combined,
        &[
            "WEB_ROUTES".to_string(),
            "LOCAL_ROUTES".to_string(),
            "route_misdirected".to_string(),
            "REQUIRED_LOCAL_REJECTED_ROUTES".to_string(),
            "workspace_search".to_string(),
            "web_search".to_string(),
            "web_fetch".to_string(),
        ],
        vec!["route misclassification guard covers local-vs-web route separation".to_string()],
    )
}

fn package_scripts_registered(package_json: &str) -> ToolRoutingAuthorityCheck {
    token_check(
        "package_scripts_registered",
        package_json,
        &[
            "ops:typed-probe:matrix:guard".to_string(),
            "ops:tool-route:misdirection:guard".to_string(),
            "ops:tool-routing:authority:guard".to_string(),
        ],
        vec!["tool-routing authority evidence is runnable through package scripts".to_string()],
    )
}

fn planner_payload_decision_audit(
    preconditions: &str,
    planner_candidates: &str,
    planner_common: &str,
    planner_chain: &str,
    planner_strategy: &str,
) -> Vec<PlannerPayloadDecisionAuditRow> {
    vec![
        legacy_payload_decision_row(
            "surface/orchestration/src/planner/preconditions.rs",
            "legacy_probe_shortcuts",
            preconditions,
        ),
        non_legacy_payload_decision_row(
            "surface/orchestration/src/planner/plan_candidates.rs",
            "candidate_generation",
            planner_candidates,
        ),
        non_legacy_payload_decision_row(
            "surface/orchestration/src/planner/plan_candidates/common.rs",
            "candidate_common_helpers",
            planner_common,
        ),
        non_legacy_payload_decision_row(
            "surface/orchestration/src/planner/plan_candidates/chain.rs",
            "candidate_chain_selection",
            planner_chain,
        ),
        non_legacy_payload_decision_row(
            "surface/orchestration/src/planner/plan_candidates/strategy.rs",
            "strategy_capability_selection",
            planner_strategy,
        ),
    ]
}

fn legacy_payload_decision_row(
    path: &str,
    decision_scope: &str,
    source: &str,
) -> PlannerPayloadDecisionAuditRow {
    let payload_read_count = source.matches("request.payload").count() as u64;
    let legacy_only = source.contains("allow_payload_probe_shortcuts")
        && source.contains("RequestSurface::Legacy")
        && source.contains("if !allow_payload_probe_shortcuts(request)");
    PlannerPayloadDecisionAuditRow {
        path: path.to_string(),
        decision_scope: decision_scope.to_string(),
        payload_read_count,
        legacy_only,
        ok: payload_read_count == 0 || legacy_only,
        evidence: vec![
            "payload probe shortcuts are allowed only behind RequestSurface::Legacy".to_string(),
        ],
    }
}

fn non_legacy_payload_decision_row(
    path: &str,
    decision_scope: &str,
    source: &str,
) -> PlannerPayloadDecisionAuditRow {
    let payload_read_count = source.matches("request.payload").count() as u64;
    PlannerPayloadDecisionAuditRow {
        path: path.to_string(),
        decision_scope: decision_scope.to_string(),
        payload_read_count,
        legacy_only: false,
        ok: payload_read_count == 0,
        evidence: vec![
            "non-legacy planner candidate decisions must use typed fields, CoreProbeEnvelope, or execution observations".to_string(),
        ],
    }
}

fn planner_payload_decision_audit_enforced(
    rows: &[PlannerPayloadDecisionAuditRow],
) -> ToolRoutingAuthorityCheck {
    let missing = rows
        .iter()
        .filter(|row| !row.ok)
        .map(|row| {
            format!(
                "{} has {} non-legacy request.payload decision reads",
                row.path, row.payload_read_count
            )
        })
        .collect::<Vec<_>>();
    ToolRoutingAuthorityCheck::new(
        "planner_payload_decision_audit_enforced",
        vec![
            "planner candidate paths do not read raw request.payload for non-legacy decisions"
                .to_string(),
            "legacy precondition payload shortcuts remain explicitly gated".to_string(),
        ],
        missing,
    )
}

fn token_check(
    id: &str,
    source: &str,
    required: &[String],
    evidence: Vec<String>,
) -> ToolRoutingAuthorityCheck {
    let missing = required
        .iter()
        .filter(|token| !source.contains(token.as_str()))
        .map(|token| format!("missing token: {token}"))
        .collect::<Vec<_>>();
    ToolRoutingAuthorityCheck::new(id, evidence, missing)
}

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
    lines.push("## Planner Payload Decision Audit".to_string());
    for row in &report.planner_payload_decision_audit {
        lines.push(format!(
            "- {} [{}]: ok={} payload_read_count={} legacy_only={}",
            row.path, row.decision_scope, row.ok, row.payload_read_count, row.legacy_only
        ));
    }
    lines.push(String::new());
    lines.push("## Checks".to_string());
    for check in &report.checks {
        lines.push(format!(
            "- {}: ok={} missing={}",
            check.id,
            check.ok,
            if check.missing.is_empty() {
                "none".to_string()
            } else {
                check.missing.join("; ")
            }
        ));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn read_text(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).unwrap_or_default()
}

fn write_text(root: &Path, relative: &str, content: &str) -> Result<(), String> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(path, content).map_err(|err| err.to_string())
}

fn generated_at_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn normalize_source(source: &str) -> String {
    source.split_whitespace().collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_tool_probe_keys_are_distinct_and_do_not_use_execute_tool() {
        let mut keys = REQUIRED_TOOL_PROBE_KEYS.to_vec();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), REQUIRED_TOOL_PROBE_KEYS.len());
        assert!(!REQUIRED_TOOL_PROBE_KEYS.contains(&"execute_tool"));
    }

    #[test]
    fn token_check_reports_exact_missing_contract() {
        let check = token_check(
            "sample",
            "workspace_read web_search",
            &[
                "workspace_read".to_string(),
                "workspace_search".to_string(),
                "web_search".to_string(),
            ],
            vec![],
        );
        assert!(!check.ok);
        assert_eq!(check.missing, vec!["missing token: workspace_search"]);
    }

    #[test]
    fn markdown_surfaces_selected_rejected_and_confidence_fields() {
        let mut summary = BTreeMap::new();
        summary.insert("total_checks".to_string(), 0);
        let report = ToolRoutingAuthorityReport {
            schema_id: "tool_routing_authority_guard".to_string(),
            schema_version: 1,
            generated_at_unix_seconds: 0,
            ok: true,
            required_tool_probe_keys: REQUIRED_TOOL_PROBE_KEYS
                .iter()
                .map(|row| row.to_string())
                .collect(),
            decision_trace_fields: DECISION_TRACE_FIELDS
                .iter()
                .map(|row| row.to_string())
                .collect(),
            planner_payload_decision_audit: Vec::new(),
            summary,
            checks: Vec::new(),
        };
        let markdown = render_markdown(&report);
        assert!(markdown.contains("selected"));
        assert!(markdown.contains("rejected"));
        assert!(markdown.contains("confidence"));
    }

    #[test]
    fn planner_payload_audit_allows_only_legacy_shortcut_reads() {
        let rows = planner_payload_decision_audit(
            "fn allow_payload_probe_shortcuts(request: &TypedOrchestrationRequest) -> bool { matches!(request.surface, RequestSurface::Legacy) } fn probe_bool(request: &TypedOrchestrationRequest) { if !allow_payload_probe_shortcuts(request) { return; } let _ = request.payload.get(\"capability_probes\"); }",
            "fn build() {}",
            "fn common() {}",
            "fn chain() {}",
            "fn strategy() {}",
        );
        let check = planner_payload_decision_audit_enforced(&rows);

        assert!(check.ok);
        assert_eq!(rows[0].payload_read_count, 1);
        assert!(rows[0].legacy_only);
    }

    #[test]
    fn planner_payload_audit_rejects_candidate_payload_reads() {
        let rows = planner_payload_decision_audit(
            "fn allow_payload_probe_shortcuts(request: &TypedOrchestrationRequest) -> bool { matches!(request.surface, RequestSurface::Legacy) } fn probe_bool(request: &TypedOrchestrationRequest) { if !allow_payload_probe_shortcuts(request) { return; } let _ = request.payload.get(\"capability_probes\"); }",
            "fn build(request: &TypedOrchestrationRequest) { let _ = request.payload.get(\"transport_available\"); }",
            "fn common() {}",
            "fn chain() {}",
            "fn strategy() {}",
        );
        let check = planner_payload_decision_audit_enforced(&rows);

        assert!(!check.ok);
        assert!(check.missing[0].contains("plan_candidates.rs"));
    }
}
