// Layer ownership: surface/orchestration (tool-routing authority evidence only).
mod report_rendering;

pub use report_rendering::render_markdown;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ToolRoutingAuthorityOperatorSummary {
    pub status: String,
    pub total_checks: u64,
    pub passing_checks: u64,
    pub failing_checks: u64,
    pub planner_payload_decision_audit_failures: u64,
    pub json_artifact: String,
    pub markdown_artifact: String,
    pub operator_next_step: String,
    pub top_failed_check: Option<String>,
    pub top_failed_missing_count: u64,
    pub authority_promotion_blocked: bool,
    pub release_blocking: bool,
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
    pub operator_summary: ToolRoutingAuthorityOperatorSummary,
    pub summary: BTreeMap<String, u64>,
    pub checks: Vec<ToolRoutingAuthorityCheck>,
}

pub fn build_tool_routing_authority_report(root: impl AsRef<Path>) -> ToolRoutingAuthorityReport {
    let root = root.as_ref();
    let contracts = read_text(root, "surface/orchestration/src/contracts.rs");
    let preconditions = read_text(root, "surface/orchestration/src/planner/preconditions.rs");
    let capability_registry = read_text(
        root,
        "surface/orchestration/src/planner/capability_registry.rs",
    );
    let planner_candidates =
        read_text(root, "surface/orchestration/src/planner/plan_candidates.rs");
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
    let sequencing = read_text(root, "surface/orchestration/src/sequencing.rs");
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
    let request_surface_policy = read_text(
        root,
        "surface/orchestration/config/request_surface_probe_authority_policy.json",
    );
    let result_packaging = read_text(root, "surface/orchestration/src/result_packaging.rs");
    let continuous_eval = read_text(root, "surface/orchestration/src/continuous_eval.rs");
    let eval_feedback_router = read_text(root, "surface/orchestration/src/eval_feedback_router.rs");
    let transient_context = read_text(root, "surface/orchestration/src/transient_context.rs");
    let release_proof_pack_assemble = read_text(
        root,
        "tests/tooling/scripts/ci/release_proof_pack_assemble.ts",
    );
    let self_maintenance_executor = read_text(
        root,
        "surface/orchestration/src/self_maintenance/executor.rs",
    );
    let self_maintenance_noise_policy_doc = read_text(
        root,
        "docs/workspace/policy/self_maintenance_noise_discipline_policy.md",
    );
    let self_maintenance_noise_policy_json = read_text(
        root,
        "surface/orchestration/config/self_maintenance_noise_discipline_policy.json",
    );
    let planner_payload_decision_audit = planner_payload_decision_audit(
        &preconditions,
        &planner_candidates,
        &planner_common,
        &planner_chain,
        &planner_strategy,
        &sequencing,
    );

    let checks = vec![
        required_probe_keys_declared(&contracts, &preconditions),
        typed_surfaces_fail_closed_on_missing_probes(&preconditions),
        heuristic_probe_fallbacks_compatibility_fenced(&preconditions),
        generic_execute_tool_not_authoritative(&contracts, &preconditions),
        specific_missing_probe_diagnostics_declared(&preconditions),
        decision_trace_shape_declared(&preconditions),
        decision_trace_regressions_declared(&probe_matrix),
        request_surface_probe_authority_policy_declared(&request_surface_policy),
        request_surface_probe_authority_policy_semantics_declared(&request_surface_policy),
        prepare_context_capability_contract_declared(
            &contracts,
            &preconditions,
            &capability_registry,
            &planner_candidates,
        ),
        planner_candidate_metadata_contract_declared(&planner_candidates),
        workflow_quality_scoped_metadata_declared(&contracts, &result_packaging),
        runtime_quality_schema_workflow_clean(&contracts),
        legacy_heuristic_sources_registered(&preconditions, &request_surface_policy),
        self_maintenance_noise_discipline_declared(
            &self_maintenance_executor,
            &self_maintenance_noise_policy_doc,
            &self_maintenance_noise_policy_json,
        ),
        eval_issue_stability_gate_declared(&continuous_eval, &eval_feedback_router),
        eval_issue_autonomy_safety_contract_declared(&continuous_eval, &eval_feedback_router),
        issue_candidate_lifecycle_and_provenance_contract_declared(
            &continuous_eval,
            &eval_feedback_router,
            &release_proof_pack_assemble,
        ),
        transient_observation_invariant_report_declared(&transient_context),
        authority_artifact_paths_are_local(),
        planner_payload_audit_scope_complete(&planner_payload_decision_audit),
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
    let operator_summary =
        build_operator_summary(&summary, &checks, &planner_payload_decision_audit);

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
        operator_summary,
        summary,
        checks,
    }
}

fn build_operator_summary(
    summary: &BTreeMap<String, u64>,
    checks: &[ToolRoutingAuthorityCheck],
    planner_payload_decision_audit: &[PlannerPayloadDecisionAuditRow],
) -> ToolRoutingAuthorityOperatorSummary {
    let failing_checks = summary.get("failing_checks").copied().unwrap_or_default();
    let audit_failures = summary
        .get("planner_payload_decision_audit_failures")
        .copied()
        .unwrap_or_default();
    let top_failed_check = checks.iter().find(|check| !check.ok);
    ToolRoutingAuthorityOperatorSummary {
        status: if failing_checks == 0 && audit_failures == 0 {
            "pass".to_string()
        } else {
            "fail".to_string()
        },
        total_checks: summary.get("total_checks").copied().unwrap_or_default(),
        passing_checks: summary.get("passing_checks").copied().unwrap_or_default(),
        failing_checks,
        planner_payload_decision_audit_failures: audit_failures,
        json_artifact: TOOL_ROUTING_AUTHORITY_ARTIFACT_JSON.to_string(),
        markdown_artifact: TOOL_ROUTING_AUTHORITY_ARTIFACT_MARKDOWN.to_string(),
        operator_next_step: if failing_checks == 0 && audit_failures == 0 {
            "keep guard in release evidence bundle".to_string()
        } else {
            "fix failed checks before release or planner/eval authority promotion".to_string()
        },
        top_failed_check: top_failed_check.map(|check| check.id.clone()).or_else(|| {
            planner_payload_decision_audit
                .iter()
                .find(|row| !row.ok)
                .map(|row| format!("planner_payload_decision_audit:{}", row.decision_scope))
        }),
        top_failed_missing_count: top_failed_check
            .map(|check| check.missing.len() as u64)
            .unwrap_or_default(),
        authority_promotion_blocked: failing_checks > 0 || audit_failures > 0,
        release_blocking: failing_checks > 0 || audit_failures > 0,
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
    sequencing: &str,
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
        non_legacy_payload_decision_row(
            "surface/orchestration/src/sequencing.rs",
            "sequencing_feedback_and_fallback",
            sequencing,
        ),
    ]
}

fn prepare_context_capability_contract_declared(
    contracts: &str,
    preconditions: &str,
    capability_registry: &str,
    planner_candidates: &str,
) -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    if !contracts.contains("Capability::PrepareContext")
        || !contracts.contains("Capability::PrepareContext => &[\"prepare_context\"]")
    {
        missing.push(
            "contracts must declare PrepareContext and map it to the prepare_context probe key"
                .to_string(),
        );
    }
    if !preconditions.contains("\"prepare_context\"") || !preconditions.contains("PrepareContext") {
        missing
            .push("preconditions must parse prepare_context as an explicit capability".to_string());
    }
    if !capability_registry.contains("Capability::PrepareContext")
        || !capability_registry.contains("ContextAtomAppend")
    {
        missing.push(
            "capability registry must model context preparation as a mutating ContextAtomAppend pre-step"
                .to_string(),
        );
    }
    if !planner_candidates.contains("Capability::PrepareContext")
        || !planner_candidates.contains("explicit_context_preparation_capability:selected")
    {
        missing.push(
            "planner candidates must report selected context preparation as an explicit capability"
                .to_string(),
        );
    }
    ToolRoutingAuthorityCheck::new(
        "prepare_context_capability_contract_declared",
        vec![
            "context preparation is a named capability, not an implicit read-memory side effect"
                .to_string(),
            "prepare_context has a typed probe key and mutating contract evidence".to_string(),
        ],
        missing,
    )
}

fn planner_candidate_metadata_contract_declared(
    planner_candidates: &str,
) -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    if !planner_candidates.contains("decomposition_signature(")
        || !planner_candidates.contains("decomposition_signature,")
    {
        missing.push(
            "plan candidates must populate decomposition_signature from family, contract, and capability graph"
                .to_string(),
        );
    }
    if !planner_candidates.contains("let reported_capabilities = capability_graph.clone();")
        || !planner_candidates.contains("capabilities: reported_capabilities")
    {
        missing.push(
            "reported candidate capabilities must mirror the final capability_graph after pre-step injection"
                .to_string(),
        );
    }
    if !planner_candidates.contains("fn capability_key(")
        || !planner_candidates.contains(".map(capability_key)")
    {
        missing.push(
            "decomposition signatures must use stable capability probe keys instead of debug formatting"
                .to_string(),
        );
    }
    ToolRoutingAuthorityCheck::new(
        "planner_candidate_metadata_contract_declared",
        vec![
            "alternative plans expose stable decomposition signatures".to_string(),
            "candidate capability metadata cannot drop injected mutating pre-step capabilities"
                .to_string(),
        ],
        missing,
    )
}

fn heuristic_probe_fallbacks_compatibility_fenced(
    preconditions: &str,
) -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    for token in [
        "allow_heuristic_probe_fallback(",
        "RequestSurface::Legacy",
        "heuristic.policy_scope_and_mutability",
        "heuristic.transport_hints_or_operation",
        "probe.required_for_typed_surface",
    ] {
        if !preconditions.contains(token) {
            missing.push(format!(
                "heuristic probe fallback fence missing token: {token}"
            ));
        }
    }
    ToolRoutingAuthorityCheck::new(
        "heuristic_probe_fallbacks_compatibility_fenced",
        vec![
            "heuristic probe fallbacks remain legacy compatibility only".to_string(),
            "typed surfaces receive missing-probe diagnostics instead of payload shortcuts"
                .to_string(),
        ],
        missing,
    )
}

fn request_surface_probe_authority_policy_declared(
    request_surface_policy: &str,
) -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    for token in [
        "\"request_surface_probe_authority_policy\"",
        "\"sdk\"",
        "\"gateway\"",
        "\"dashboard\"",
        "\"typed_cli\"",
        "\"legacy\"",
    ] {
        if !request_surface_policy.contains(token) {
            missing.push(format!("request-surface policy missing token: {token}"));
        }
    }
    ToolRoutingAuthorityCheck::new(
        "request_surface_probe_authority_policy_declared",
        vec![
            "request surface probe authority policy exists as an explicit config artifact"
                .to_string(),
            "adapted and legacy lanes are named separately".to_string(),
        ],
        missing,
    )
}

fn request_surface_probe_authority_policy_semantics_declared(
    request_surface_policy: &str,
) -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    for surface in ["sdk", "gateway", "dashboard", "typed_cli"] {
        if !request_surface_policy.contains(&format!("\"{surface}\"")) {
            missing.push(format!(
                "request-surface probe authority policy must cover adapted surface {surface}"
            ));
        }
    }
    for token in [
        "\"authority_source\": \"core_probe_envelope\"",
        "\"probe_authoritative\": true",
        "\"payload_probe_shortcuts_allowed\": false",
        "\"heuristic_probe_fallback_allowed\": false",
        "\"missing_transport_probe_behavior\": \"refuse_missing_transport_probe\"",
        "\"missing_policy_probe_behavior\": \"refuse_missing_policy_probe\"",
        "\"legacy\"",
        "\"payload_probe_shortcuts_allowed\": true",
        "\"heuristic_probe_fallback_allowed\": true",
        "\"required_marker\": \"RequestSurface::Legacy\"",
        "\"missing_probe: <capability>.<field>\"",
        "\"probe.required_for_typed_surface.<capability>.<field>\"",
    ] {
        if !request_surface_policy.contains(token) {
            missing.push(format!(
                "request-surface probe authority policy missing token: {token}"
            ));
        }
    }
    ToolRoutingAuthorityCheck::new(
        "request_surface_probe_authority_policy_semantics_declared",
        vec![
            "adapted surfaces are CoreProbeEnvelope-authoritative by policy".to_string(),
            "legacy is the only lane allowed to use payload shortcuts or heuristic fallback"
                .to_string(),
        ],
        missing,
    )
}

fn authority_artifact_paths_are_local() -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    if !TOOL_ROUTING_AUTHORITY_ARTIFACT_JSON.starts_with("core/local/artifacts/") {
        missing.push(
            "tool-routing authority JSON artifact must stay under core/local/artifacts/"
                .to_string(),
        );
    }
    if !TOOL_ROUTING_AUTHORITY_ARTIFACT_MARKDOWN.starts_with("local/workspace/reports/") {
        missing.push(
            "tool-routing authority markdown report must stay under local/workspace/reports/"
                .to_string(),
        );
    }
    if TOOL_ROUTING_AUTHORITY_ARTIFACT_JSON.contains("/../")
        || TOOL_ROUTING_AUTHORITY_ARTIFACT_MARKDOWN.contains("/../")
    {
        missing.push("tool-routing authority artifact paths must not traverse upward".to_string());
    }
    ToolRoutingAuthorityCheck::new(
        "authority_artifact_paths_are_local",
        vec![
            "authority guard artifacts are local/runtime evidence, not root churn".to_string(),
            "guard output cannot escape canonical local artifact/report directories".to_string(),
        ],
        missing,
    )
}

fn planner_payload_audit_scope_complete(
    rows: &[PlannerPayloadDecisionAuditRow],
) -> ToolRoutingAuthorityCheck {
    let required = [
        (
            "surface/orchestration/src/planner/preconditions.rs",
            "legacy_probe_shortcuts",
        ),
        (
            "surface/orchestration/src/planner/plan_candidates.rs",
            "candidate_generation",
        ),
        (
            "surface/orchestration/src/planner/plan_candidates/common.rs",
            "candidate_common_helpers",
        ),
        (
            "surface/orchestration/src/planner/plan_candidates/chain.rs",
            "candidate_chain_selection",
        ),
        (
            "surface/orchestration/src/planner/plan_candidates/strategy.rs",
            "strategy_capability_selection",
        ),
        (
            "surface/orchestration/src/sequencing.rs",
            "sequencing_feedback_and_fallback",
        ),
    ];
    let mut missing = Vec::new();
    for (path, scope) in required {
        if !rows
            .iter()
            .any(|row| row.path == path && row.decision_scope == scope)
        {
            missing.push(format!(
                "planner payload audit must include {path} [{scope}]"
            ));
        }
    }
    let non_legacy_shortcut_rows = rows
        .iter()
        .filter(|row| {
            row.legacy_only && row.path != "surface/orchestration/src/planner/preconditions.rs"
        })
        .map(|row| format!("{} [{}]", row.path, row.decision_scope))
        .collect::<Vec<_>>();
    missing.extend(non_legacy_shortcut_rows.into_iter().map(|row| {
        format!("legacy payload shortcut allowance must not appear outside preconditions: {row}")
    }));
    ToolRoutingAuthorityCheck::new(
        "planner_payload_audit_scope_complete",
        vec![
            "planner payload audit covers candidate, helper, chain, strategy, and sequencing paths"
                .to_string(),
            "legacy raw-payload shortcuts are scoped only to preconditions".to_string(),
        ],
        missing,
    )
}

fn workflow_quality_scoped_metadata_declared(
    contracts: &str,
    result_packaging: &str,
) -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    for field in [
        "workflow_decomposition_signature_count",
        "workflow_distinct_contract_family_count",
        "workflow_distinct_capability_graph_count",
        "selected_decomposition_signature",
        "alternative_decomposition_signatures",
    ] {
        if !contracts.contains(field) || !result_packaging.contains(field) {
            missing.push(format!(
                "workflow-scoped ForgeCode quality metadata must declare and populate {field}"
            ));
        }
    }
    if !contracts.contains("ForgeCode(ForgeCodeWorkflowQualitySignals)")
        || !result_packaging.contains("WorkflowQualitySignals::ForgeCode")
    {
        missing.push(
            "ForgeCode workflow quality must stay behind WorkflowQualitySignals::ForgeCode"
                .to_string(),
        );
    }
    ToolRoutingAuthorityCheck::new(
        "workflow_quality_scoped_metadata_declared",
        vec![
            "workflow-family doctrine stays out of generic runtime quality".to_string(),
            "ForgeCode decomposition evidence remains workflow-scoped and populated".to_string(),
        ],
        missing,
    )
}

/// Tokens forbidden inside the `RuntimeQualitySignals` struct body.
///
/// The generic runtime-quality tier must stay workflow-agnostic; per-family
/// doctrine (ForgeCode, OpenHands, Codex, etc.) belongs in a `WorkflowQualitySignals`
/// enum variant, never in the generic struct. See:
/// `docs/workspace/policy/workflow_quality_extension_boundary_policy.md`.
const RUNTIME_QUALITY_FORBIDDEN_TOKENS: [&str; 11] = [
    "forgecode",
    "openhands",
    "forge_",
    "sage_",
    "muse_",
    "mcp_",
    "subagent_",
    "codex_",
    "step_checkpoint",
    "completion_hygiene",
    "parallel_independent_tool_calls",
];

fn extract_runtime_quality_signals_body(contracts: &str) -> Option<&str> {
    let header = "pub struct RuntimeQualitySignals {";
    let start = contracts.find(header)?;
    let body_start = start + header.len();
    let remainder = &contracts[body_start..];
    let close = remainder.find('}')?;
    Some(&remainder[..close])
}

/// Closed registry of heuristic source labels emitted by `preconditions.rs`
/// when `allow_heuristic_probe_fallback` is true. Adding a new heuristic source
/// requires updating this list AND the
/// `legacy_heuristic_source_registry.sources` array in
/// `surface/orchestration/config/request_surface_probe_authority_policy.json`.
const LEGACY_HEURISTIC_SOURCE_REGISTRY: [&str; 7] = [
    "heuristic.tool_hints_or_operation",
    "heuristic.target_descriptors_present",
    "heuristic.target_descriptor_domain",
    "heuristic.target_refs_present",
    "heuristic.mutation_cross_boundary",
    "heuristic.policy_scope_and_mutability",
    "heuristic.transport_hints_or_operation",
];

fn legacy_heuristic_sources_registered(
    preconditions: &str,
    request_surface_policy: &str,
) -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    for source in LEGACY_HEURISTIC_SOURCE_REGISTRY {
        if !preconditions.contains(source) {
            missing.push(format!(
                "preconditions.rs is missing registered heuristic source: {source}"
            ));
        }
        if !request_surface_policy.contains(source) {
            missing.push(format!(
                "request_surface_probe_authority_policy.json must declare heuristic source: {source}"
            ));
        }
    }
    if !request_surface_policy.contains("legacy_heuristic_source_registry") {
        missing
            .push("request_surface_probe_authority_policy.json missing legacy_heuristic_source_registry section".to_string());
    }
    // Guard against unregistered heuristic source labels: scan preconditions
    // for any "heuristic.<token>" string that is not in the registry.
    let mut cursor = 0;
    let needle = "heuristic.";
    while let Some(idx) = preconditions[cursor..].find(needle) {
        let abs = cursor + idx;
        let tail = &preconditions[abs..];
        let mut end = needle.len();
        while end < tail.len() {
            let ch = tail.as_bytes()[end] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' {
                end += 1;
            } else {
                break;
            }
        }
        let candidate = &tail[..end];
        if !LEGACY_HEURISTIC_SOURCE_REGISTRY.contains(&candidate)
            && candidate != "heuristic."
            && !candidate.is_empty()
        {
            missing.push(format!(
                "preconditions.rs uses unregistered heuristic source: {candidate}"
            ));
        }
        cursor = abs + end;
    }
    ToolRoutingAuthorityCheck::new(
        "legacy_heuristic_sources_registered",
        vec![
            "every heuristic.* source label used in preconditions.rs is in the closed registry"
                .to_string(),
            "request_surface_probe_authority_policy.json declares the registry as a burn-down target"
                .to_string(),
        ],
        missing,
    )
}

fn self_maintenance_noise_discipline_declared(
    self_maintenance_executor: &str,
    noise_policy_doc: &str,
    noise_policy_json: &str,
) -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    if !self_maintenance_executor.contains("ObserveOnly") {
        missing.push(
            "self_maintenance executor must keep ObserveOnly emission mode declared".to_string(),
        );
    }
    for token in [
        "self_maintenance_noise_discipline_policy",
        "rate_limit_window_floor_ms",
        "1800000",
        "rate_limit_window_multiplier_vs_planner_clarification_window",
        "ObserveOnly",
    ] {
        if !noise_policy_json.contains(token) {
            missing.push(format!(
                "self_maintenance_noise_discipline_policy.json missing token: {token}"
            ));
        }
    }
    for token in [
        "Self-Maintenance Noise Discipline Policy",
        "ObserveOnly",
        "30 minutes",
        "self_maintenance_noise_discipline_declared",
    ] {
        if !noise_policy_doc.contains(token) {
            missing.push(format!(
                "self_maintenance_noise_discipline_policy.md missing token: {token}"
            ));
        }
    }
    ToolRoutingAuthorityCheck::new(
        "self_maintenance_noise_discipline_declared",
        vec![
            "self-maintenance recommendations stay in ObserveOnly mode".to_string(),
            "rate-limit window stays >=2x planner clarification window with a 30-minute floor"
                .to_string(),
        ],
        missing,
    )
}

fn runtime_quality_schema_workflow_clean(contracts: &str) -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    let body = match extract_runtime_quality_signals_body(contracts) {
        Some(body) => body,
        None => {
            missing.push(
                "RuntimeQualitySignals struct definition not found in contracts.rs".to_string(),
            );
            return ToolRoutingAuthorityCheck::new(
                "runtime_quality_schema_workflow_clean",
                vec![
                    "generic RuntimeQualitySignals must remain workflow-family-agnostic"
                        .to_string(),
                    "workflow-family doctrine extends WorkflowQualitySignals enum, not the generic tier"
                        .to_string(),
                ],
                missing,
            );
        }
    };
    let body_lower = body.to_ascii_lowercase();
    for token in RUNTIME_QUALITY_FORBIDDEN_TOKENS {
        if body_lower.contains(token) {
            missing.push(format!(
                "RuntimeQualitySignals contains workflow-family-specific token: {token}"
            ));
        }
    }
    ToolRoutingAuthorityCheck::new(
        "runtime_quality_schema_workflow_clean",
        vec![
            "generic RuntimeQualitySignals stays workflow-family-agnostic".to_string(),
            "new workflow doctrine lands in a WorkflowQualitySignals variant, not the generic tier"
                .to_string(),
        ],
        missing,
    )
}

fn eval_issue_stability_gate_declared(
    continuous_eval: &str,
    eval_feedback_router: &str,
) -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    for token in [
        "stable_signature_occurrence_count",
        "minimum_issue_candidate_occurrences",
        "issue_candidate_ready",
    ] {
        if !continuous_eval.contains(token) || !eval_feedback_router.contains(token) {
            missing.push(format!(
                "eval issue readiness must preserve repeated-signature token {token}"
            ));
        }
    }
    if !continuous_eval.contains("issue_candidate_waiting_for_recurrence")
        && !eval_feedback_router.contains("issue_candidate_waiting_for_recurrence")
    {
        missing.push(
            "single-occurrence eval failures must expose a waiting-for-recurrence reason"
                .to_string(),
        );
    }
    ToolRoutingAuthorityCheck::new(
        "eval_issue_stability_gate_declared",
        vec![
            "eval observations stay authoritative while issue filing waits for stable recurrence"
                .to_string(),
            "one-off degraded runs cannot become issue candidates without recurrence metadata"
                .to_string(),
        ],
        missing,
    )
}

fn eval_issue_autonomy_safety_contract_declared(
    continuous_eval: &str,
    eval_feedback_router: &str,
) -> ToolRoutingAuthorityCheck {
    let combined = format!("{continuous_eval}\n{eval_feedback_router}");
    token_check(
        "eval_issue_autonomy_safety_contract_declared",
        &combined,
        &[
            "safe_to_auto_file_issue".to_string(),
            "safe_to_auto_apply_patch".to_string(),
            "human_review_required".to_string(),
            "autonomous_mitigation_allowed".to_string(),
            "proposal_only".to_string(),
        ],
        vec![
            "eval may produce authoritative observations and issue candidates".to_string(),
            "eval issue candidates remain proposal-only and cannot auto-apply patches".to_string(),
        ],
    )
}

fn issue_candidate_lifecycle_and_provenance_contract_declared(
    continuous_eval: &str,
    eval_feedback_router: &str,
    release_proof_pack_assemble: &str,
) -> ToolRoutingAuthorityCheck {
    let surfaces = [
        ("continuous_eval", continuous_eval),
        ("eval_feedback_router", eval_feedback_router),
        ("release_proof_pack_assemble", release_proof_pack_assemble),
    ];
    let required_common = [
        "issue_contract_version",
        "source_report",
        "issue_lifecycle_state",
        "source_artifacts",
        "source_artifact_policy",
        "local_relative_paths_only",
        "triage_queue",
        "requires_operator_ack",
        "reopen_policy",
        "close_on_absence_window",
        "closing_evidence_required",
        "closure_verification_command",
        "safe_to_auto_file_issue",
        "safe_to_auto_apply_patch",
        "human_review_required",
        "autonomous_mitigation_allowed",
    ];
    let required_eval_like = ["closing_evidence_required"];
    let required_proof_pack = ["localArtifactPathOk", "issue_candidate_contract"];
    let mut missing = Vec::new();
    for (surface, source) in surfaces {
        for token in required_common {
            if !source.contains(token) {
                missing.push(format!(
                    "{surface} issue-candidate contract missing token: {token}"
                ));
            }
        }
        if surface == "continuous_eval" || surface == "eval_feedback_router" {
            for token in required_eval_like {
                if !source.contains(token) {
                    missing.push(format!(
                        "{surface} eval issue-candidate contract missing token: {token}"
                    ));
                }
            }
        }
        if surface == "release_proof_pack_assemble" {
            for token in required_proof_pack {
                if !source.contains(token) {
                    missing.push(format!(
                        "{surface} proof-pack issue-candidate contract missing token: {token}"
                    ));
                }
            }
        }
    }
    ToolRoutingAuthorityCheck::new(
        "issue_candidate_lifecycle_and_provenance_contract_declared",
        vec![
            "eval, live-eval, and proof-pack issue candidates share lifecycle/provenance fields"
                .to_string(),
            "issue candidates require local evidence, operator acknowledgement, closure proof, and no auto-patch"
                .to_string(),
        ],
        missing,
    )
}

fn transient_observation_invariant_report_declared(
    transient_context: &str,
) -> ToolRoutingAuthorityCheck {
    let mut missing = Vec::new();
    for token in [
        "TransientExecutionObservationInvariantReport",
        "execution_observation_invariant_report",
        "dangling_observation_rows",
        "retired_execution_observation_objects",
        "retired_observation_heap_still_active",
    ] {
        if !transient_context.contains(token) {
            missing.push(format!(
                "transient execution-observation invariant report must expose {token}"
            ));
        }
    }
    ToolRoutingAuthorityCheck::new(
        "transient_observation_invariant_report_declared",
        vec![
            "transient execution observations expose map/heap drift as a production report"
                .to_string(),
            "retired observation refs are tracked so cleanup/restart drift is observable"
                .to_string(),
        ],
        missing,
    )
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
            operator_summary: ToolRoutingAuthorityOperatorSummary::default(),
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
            "fn sequencing() {}",
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
            "fn sequencing() {}",
        );
        let check = planner_payload_decision_audit_enforced(&rows);

        assert!(!check.ok);
        assert!(check.missing[0].contains("plan_candidates.rs"));
    }

    #[test]
    fn runtime_quality_schema_workflow_clean_passes_for_generic_struct() {
        let contracts = "pub struct RuntimeQualitySignals {\n    pub candidate_count: u32,\n    pub used_heuristic_probe: bool,\n    pub blocked_precondition_count: u32,\n    pub typed_probe_contract_gap_count: u32,\n    pub fallback_action_count: u32,\n}";
        let check = runtime_quality_schema_workflow_clean(contracts);
        assert!(check.ok, "expected clean schema to pass: {:?}", check.missing);
        assert!(check.missing.is_empty());
    }

    #[test]
    fn runtime_quality_schema_workflow_clean_rejects_forgecode_field() {
        let contracts = "pub struct RuntimeQualitySignals {\n    pub candidate_count: u32,\n    pub forgecode_subagent_active: bool,\n}";
        let check = runtime_quality_schema_workflow_clean(contracts);
        assert!(!check.ok);
        assert!(check
            .missing
            .iter()
            .any(|row| row.contains("forgecode")));
    }

    #[test]
    fn runtime_quality_schema_workflow_clean_rejects_mcp_token() {
        let contracts =
            "pub struct RuntimeQualitySignals {\n    pub mcp_alias_route_required: bool,\n}";
        let check = runtime_quality_schema_workflow_clean(contracts);
        assert!(!check.ok);
        assert!(check.missing.iter().any(|row| row.contains("mcp_")));
    }

    #[test]
    fn runtime_quality_schema_workflow_clean_rejects_subagent_token() {
        let contracts = "pub struct RuntimeQualitySignals {\n    pub subagent_brief_required: bool,\n}";
        let check = runtime_quality_schema_workflow_clean(contracts);
        assert!(!check.ok);
        assert!(check.missing.iter().any(|row| row.contains("subagent_")));
    }

    #[test]
    fn runtime_quality_schema_workflow_clean_does_not_inspect_workflow_struct() {
        // Tokens forbidden in RuntimeQualitySignals are allowed in the
        // ForgeCodeWorkflowQualitySignals struct; the guard scopes its check
        // strictly to the generic tier body.
        let contracts = "pub struct RuntimeQualitySignals {\n    pub candidate_count: u32,\n}\n\npub struct ForgeCodeWorkflowQualitySignals {\n    pub mcp_alias_route_required: bool,\n    pub subagent_brief_required: bool,\n}";
        let check = runtime_quality_schema_workflow_clean(contracts);
        assert!(check.ok, "guard must not flag workflow-tier fields: {:?}", check.missing);
    }

    #[test]
    fn runtime_quality_schema_workflow_clean_reports_missing_struct() {
        let contracts = "pub struct UnrelatedThing { pub x: u32 }";
        let check = runtime_quality_schema_workflow_clean(contracts);
        assert!(!check.ok);
        assert!(check
            .missing
            .iter()
            .any(|row| row.contains("RuntimeQualitySignals struct definition not found")));
    }
}
