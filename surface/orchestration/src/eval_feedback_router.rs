use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_REGRESSION_PATH: &str = "core/local/artifacts/eval_regression_guard_current.json";
const DEFAULT_ISSUE_DRAFTS_PATH: &str = "core/local/artifacts/eval_issue_drafts_current.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_feedback_router_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_feedback_router_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/EVAL_FEEDBACK_ROUTER_CURRENT.md";
const ISSUE_CANDIDATE_MIN_OCCURRENCES: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnforcementDestination {
    ControlPlaneRetry,
    GatewayQuarantine,
    KernelBlock,
}

impl EnforcementDestination {
    fn as_str(self) -> &'static str {
        match self {
            Self::ControlPlaneRetry => "control_plane_retry",
            Self::GatewayQuarantine => "gateway_quarantine",
            Self::KernelBlock => "kernel_block",
        }
    }
}

fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}

fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline_prefix = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline_prefix) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            return args.get(idx + 1).cloned();
        }
    }
    None
}

fn parse_bool_flag(args: &[String], key: &str, default: bool) -> bool {
    parse_flag(args, key)
        .map(|raw| matches!(raw.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(default)
}

fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn ensure_parent(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
        ),
    )
}

fn write_text(path: &str, value: &str) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(path, value)
}

fn str_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_str().map(str::trim).filter(|raw| !raw.is_empty())
}

fn bool_at(value: &Value, path: &[&str]) -> bool {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return false;
        };
        cursor = next;
    }
    cursor.as_bool().unwrap_or(false)
}

fn destination_for_issue_class(issue_class: &str) -> EnforcementDestination {
    match issue_class {
        "wrong_tool_selection"
        | "auto_tool_selection_claim"
        | "bad_workflow_selection"
        | "no_response"
        | "response_loop"
        | "policy_block_confusion" => EnforcementDestination::ControlPlaneRetry,
        "tool_output_misdirection"
        | "external_tool_failure"
        | "gateway_failure"
        | "invalid_schema_response"
        | "oversized_response"
        | "repeated_flapping" => EnforcementDestination::GatewayQuarantine,
        _ => EnforcementDestination::KernelBlock,
    }
}

fn route_for_destination(destination: EnforcementDestination) -> (&'static str, &'static str) {
    match destination {
        EnforcementDestination::ControlPlaneRetry => (
            "retry_with_trace_and_probe_context",
            "control_plane_retry_event",
        ),
        EnforcementDestination::GatewayQuarantine => (
            "quarantine_gateway_and_route_around",
            "gateway_quarantine_event",
        ),
        EnforcementDestination::KernelBlock => {
            ("block_release_or_runtime_escalation", "kernel_block_event")
        }
    }
}

fn push_issue_routes(routes: &mut Vec<Value>, issue_drafts: &Value) {
    let drafts = issue_drafts
        .get("issue_drafts")
        .or_else(|| issue_drafts.get("drafts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for draft in drafts {
        let issue_class = str_at(&draft, &["issue_class"]).unwrap_or("unknown");
        let destination = destination_for_issue_class(issue_class);
        let (action, receipt) = route_for_destination(destination);
        routes.push(json!({
            "source": "eval_issue_drafts",
            "failure_id": str_at(&draft, &["id"]).unwrap_or("eval_issue"),
            "failure_class": issue_class,
            "severity": str_at(&draft, &["severity"]).unwrap_or("medium"),
            "destination": destination.as_str(),
            "action": action,
            "receipt_type": receipt,
        }));
    }
}

fn push_regression_routes(routes: &mut Vec<Value>, regression: &Value) {
    if bool_at(regression, &["ok"]) {
        return;
    }
    let failures = regression
        .get("failures")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if failures.is_empty() {
        let (action, receipt) = route_for_destination(EnforcementDestination::KernelBlock);
        routes.push(json!({
            "source": "eval_regression_guard",
            "failure_id": "eval_regression_guard_not_ok",
            "failure_class": "eval_release_regression",
            "severity": "critical",
            "destination": EnforcementDestination::KernelBlock.as_str(),
            "action": action,
            "receipt_type": receipt,
        }));
        return;
    }
    for failure in failures {
        let (action, receipt) = route_for_destination(EnforcementDestination::KernelBlock);
        routes.push(json!({
            "source": "eval_regression_guard",
            "failure_id": str_at(&failure, &["artifact"]).or_else(|| str_at(&failure, &["id"])).unwrap_or("eval_regression_failure"),
            "failure_class": str_at(&failure, &["id"]).unwrap_or("eval_release_regression"),
            "severity": "critical",
            "destination": EnforcementDestination::KernelBlock.as_str(),
            "action": action,
            "receipt_type": receipt,
        }));
    }
}

fn route_fingerprint(route: &Value) -> String {
    [
        str_at(route, &["source"]).unwrap_or("unknown_source"),
        str_at(route, &["failure_class"]).unwrap_or("unknown_failure"),
        str_at(route, &["destination"]).unwrap_or("unknown_destination"),
        str_at(route, &["action"]).unwrap_or("unknown_action"),
    ]
    .join(":")
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" | "release_blocking" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn dedupe_routes(routes: Vec<Value>) -> Vec<Value> {
    let mut clustered: BTreeMap<String, Value> = BTreeMap::new();
    for mut route in routes {
        let fingerprint = route_fingerprint(&route);
        if let Some(existing) = clustered.get_mut(&fingerprint) {
            let current = existing
                .get("occurrence_count")
                .and_then(Value::as_u64)
                .unwrap_or(1);
            existing["occurrence_count"] = json!(current + 1);
            if let Some(id) = str_at(&route, &["failure_id"]) {
                let ids = existing
                    .get_mut("clustered_failure_ids")
                    .and_then(Value::as_array_mut);
                if let Some(ids) = ids {
                    if !ids.iter().any(|row| row.as_str() == Some(id)) {
                        ids.push(json!(id));
                    }
                }
            }
            let existing_severity = str_at(existing, &["severity"]).unwrap_or("unknown");
            let new_severity = str_at(&route, &["severity"]).unwrap_or("unknown");
            if severity_rank(new_severity) > severity_rank(existing_severity) {
                existing["severity"] = json!(new_severity);
            }
            continue;
        }
        route["route_fingerprint"] = json!(fingerprint.clone());
        route["occurrence_count"] = json!(1);
        route["clustered_failure_ids"] = json!([
            str_at(&route, &["failure_id"]).unwrap_or("eval_failure")
        ]);
        clustered.insert(fingerprint, route);
    }
    clustered.into_values().collect()
}

fn occurrence_total(routes: &[Value]) -> u64 {
    routes
        .iter()
        .map(|row| row.get("occurrence_count").and_then(Value::as_u64).unwrap_or(1))
        .sum()
}

fn route_issue_candidate_ready(route: &Value) -> bool {
    let occurrence_count = route
        .get("occurrence_count")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    occurrence_count >= ISSUE_CANDIDATE_MIN_OCCURRENCES
        && (matches!(
            str_at(route, &["destination"]),
            Some("kernel_block" | "gateway_quarantine" | "control_plane_retry")
        ) || matches!(
            str_at(route, &["severity"]),
            Some("critical" | "release_blocking" | "high" | "medium")
        ))
}

fn annotate_issue_readiness(routes: &mut [Value]) {
    for route in routes {
        let ready = route_issue_candidate_ready(route);
        route["issue_candidate_ready"] = json!(ready);
        route["issue_candidate_reason"] = json!(issue_candidate_reason(route));
    }
}

fn issue_candidate_reason(route: &Value) -> &'static str {
    let occurrence_count = route
        .get("occurrence_count")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    if occurrence_count < ISSUE_CANDIDATE_MIN_OCCURRENCES {
        return "awaiting_repeated_stable_signature";
    }
    match (str_at(route, &["destination"]), str_at(route, &["severity"])) {
        (Some("kernel_block"), _) => "repeated_kernel_block_route",
        (Some("gateway_quarantine"), _) => "repeated_gateway_quarantine_route",
        (_, Some("critical" | "release_blocking" | "high")) => "repeated_high_severity_route",
        _ => "repeated_failure_cluster",
    }
}

fn issue_candidate_routes(routes: &[Value]) -> Vec<Value> {
    let mut candidates = routes
        .iter()
        .filter(|row| route_issue_candidate_ready(row))
        .map(|row| {
            let failure_class = str_at(row, &["failure_class"]).unwrap_or("unknown");
            let destination = str_at(row, &["destination"]).unwrap_or("unknown");
            let occurrence_count = row.get("occurrence_count").and_then(Value::as_u64).unwrap_or(1);
            let severity = str_at(row, &["severity"]).unwrap_or("unknown");
            let priority_score = severity_rank(severity) as u64 * 100 + occurrence_count;
            json!({
                "issue_contract_version": 1,
                "source_report": "eval_feedback_router",
                "issue_lifecycle_state": "candidate_open",
                "route_fingerprint": str_at(row, &["route_fingerprint"]).unwrap_or("unknown"),
                "dedupe_key": format!("eval_feedback:{failure_class}:{destination}"),
                "source_artifacts": [issue_candidate_source_artifact(str_at(row, &["source"]).unwrap_or("unknown"))],
                "source_artifact_policy": "local_relative_paths_only",
                "stable_signature_occurrence_count": occurrence_count,
                "minimum_issue_candidate_occurrences": ISSUE_CANDIDATE_MIN_OCCURRENCES,
                "related_failure_ids": row.get("clustered_failure_ids").cloned().unwrap_or_else(|| json!([])),
                "title": format!("Eval feedback route needs action: {failure_class} -> {destination}"),
                "failure_class": failure_class,
                "destination": destination,
                "owner": issue_candidate_owner(destination),
                "target_layer": issue_candidate_target_layer(destination),
                "evidence_source": str_at(row, &["source"]).unwrap_or("unknown"),
                "release_gate_effect": issue_candidate_release_gate_effect(destination, severity),
                "severity": severity,
                "impact": issue_candidate_impact(destination),
                "occurrence_count": occurrence_count,
                "priority_score": priority_score,
                "priority_band": priority_band(priority_score),
                "escalation_tier": issue_candidate_escalation_tier(destination, severity, occurrence_count),
                "stability": issue_candidate_stability(occurrence_count),
                "reason": issue_candidate_reason(row),
                "recommended_action": str_at(row, &["action"]).unwrap_or("review_eval_failure_cluster"),
                "operator_next_step": issue_candidate_operator_next_step(destination),
                "triage_queue": issue_candidate_triage_queue(destination),
                "receipt_type": str_at(row, &["receipt_type"]).unwrap_or("eval_feedback_route_event"),
                "safe_to_auto_file_issue": true,
                "safe_to_auto_apply_patch": false,
                "human_review_required": true,
                "autonomous_mitigation_allowed": false,
                "requires_operator_ack": true,
                "reopen_policy": "reopen_if_route_fingerprint_recurs_above_threshold",
                "close_on_absence_window": "next_strict_eval_feedback_router_run",
                "closing_evidence_required": issue_candidate_closing_evidence(destination),
                "closure_verification_command": "cargo test --manifest-path surface/orchestration/Cargo.toml eval_feedback_router -- --nocapture",
                "acceptance_criteria": [
                    "failure cluster is reproduced or explicitly waived with evidence",
                    "destination action emits the declared receipt type",
                    "same route fingerprint does not recur above threshold after fix"
                ]
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .get("priority_score")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .cmp(&left.get("priority_score").and_then(Value::as_u64).unwrap_or(0))
    });
    candidates
}

fn issue_candidate_owner(destination: &str) -> &'static str {
    match destination {
        "kernel_block" => "core/kernel",
        "gateway_quarantine" => "adapters/gateways",
        "control_plane_retry" => "surface/orchestration",
        _ => "surface/orchestration",
    }
}

fn issue_candidate_actionability_ok(candidate: &Value) -> bool {
    [
        "source_report",
        "issue_lifecycle_state",
        "source_artifact_policy",
        "route_fingerprint",
        "dedupe_key",
        "owner",
        "target_layer",
        "triage_queue",
        "release_gate_effect",
        "closing_evidence_required",
        "operator_next_step",
        "receipt_type",
    ]
    .iter()
    .all(|field| str_at(candidate, &[*field]).is_some())
        && candidate
            .get("issue_contract_version")
            .and_then(Value::as_u64)
            == Some(1)
        && candidate
            .get("source_artifacts")
            .and_then(Value::as_array)
            .map(|rows| {
                !rows.is_empty()
                    && rows
                        .iter()
                        .all(|row| row.as_str().map(local_artifact_path_ok).unwrap_or(false))
            })
            .unwrap_or(false)
        && bool_at(candidate, &["safe_to_auto_file_issue"])
        && candidate
            .get("safe_to_auto_apply_patch")
            .and_then(Value::as_bool)
            == Some(false)
        && candidate.get("human_review_required").and_then(Value::as_bool) == Some(true)
        && candidate.get("requires_operator_ack").and_then(Value::as_bool) == Some(true)
        && candidate
            .get("autonomous_mitigation_allowed")
            .and_then(Value::as_bool)
            == Some(false)
}

fn local_artifact_path_ok(path: &str) -> bool {
    let trimmed = path.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('/')
        && !trimmed.starts_with("http://")
        && !trimmed.starts_with("https://")
        && !trimmed.contains("..")
}

fn issue_candidate_source_artifact(source: &str) -> &'static str {
    match source {
        "eval_issue_drafts" => DEFAULT_ISSUE_DRAFTS_PATH,
        "eval_regression_guard" => DEFAULT_REGRESSION_PATH,
        _ => DEFAULT_OUT_PATH,
    }
}

fn issue_candidate_escalation_tier(
    destination: &str,
    severity: &str,
    occurrence_count: u64,
) -> &'static str {
    if destination == "kernel_block" || matches!(severity, "critical" | "release_blocking") {
        "release_blocker"
    } else if destination == "gateway_quarantine" || occurrence_count >= 3 {
        "operator_attention"
    } else {
        "watchlist"
    }
}

fn issue_candidate_stability(occurrence_count: u64) -> &'static str {
    if occurrence_count >= 3 {
        "stable_cluster"
    } else if occurrence_count >= 2 {
        "repeated"
    } else {
        "single_observation"
    }
}

fn issue_candidate_operator_next_step(destination: &str) -> &'static str {
    match destination {
        "kernel_block" => "open or update a release-blocking kernel correctness issue",
        "gateway_quarantine" => "verify gateway quarantine/recovery receipts and route around the boundary",
        "control_plane_retry" => "inspect the workflow trace and tighten retry/probe selection",
        _ => "triage the eval feedback route",
    }
}

fn issue_candidate_target_layer(destination: &str) -> &'static str {
    match destination {
        "kernel_block" => "kernel",
        "gateway_quarantine" => "gateway",
        "control_plane_retry" => "orchestration",
        _ => "orchestration",
    }
}

fn issue_candidate_triage_queue(destination: &str) -> &'static str {
    match destination {
        "kernel_block" => "release_blockers",
        "gateway_quarantine" => "gateway_reliability",
        "control_plane_retry" => "orchestration_quality",
        _ => "eval_triage",
    }
}

fn issue_candidate_release_gate_effect(destination: &str, severity: &str) -> &'static str {
    if destination == "kernel_block" || matches!(severity, "critical" | "release_blocking") {
        "blocks_release_until_closed"
    } else if destination == "gateway_quarantine" {
        "requires_gateway_health_or_quarantine_receipt"
    } else {
        "requires_control_plane_regression_evidence"
    }
}

fn issue_candidate_closing_evidence(destination: &str) -> &'static str {
    match destination {
        "kernel_block" => "passing kernel/blocker receipt plus eval regression guard rerun",
        "gateway_quarantine" => "gateway quarantine or recovery receipt plus repeated-failure absence",
        "control_plane_retry" => "workflow retry trace plus stable issue-candidate count below threshold",
        _ => "operator review with matching receipt evidence",
    }
}

fn issue_candidate_impact(destination: &str) -> &'static str {
    match destination {
        "kernel_block" => "release/runtime correctness can be blocked until this failure class is closed",
        "gateway_quarantine" => "external-boundary reliability or fail-closed behavior is at risk",
        "control_plane_retry" => "operator-facing workflow quality may degrade or repeat retries",
        _ => "runtime quality needs triage before promotion",
    }
}

fn priority_band(score: u64) -> &'static str {
    if score >= 400 {
        "p0"
    } else if score >= 300 {
        "p1"
    } else if score >= 200 {
        "p2"
    } else {
        "p3"
    }
}

fn priority_band_count(candidates: &[Value], band: &str) -> usize {
    candidates
        .iter()
        .filter(|row| row.get("priority_band").and_then(Value::as_str) == Some(band))
        .count()
}

fn severity_count(routes: &[Value], severity: &str) -> usize {
    routes
        .iter()
        .filter(|row| str_at(row, &["severity"]) == Some(severity))
        .count()
}

fn severity_occurrence_total(routes: &[Value], severity: &str) -> u64 {
    routes
        .iter()
        .filter(|row| str_at(row, &["severity"]) == Some(severity))
        .map(|row| row.get("occurrence_count").and_then(Value::as_u64).unwrap_or(1))
        .sum()
}

fn destination_count(routes: &[Value], destination: EnforcementDestination) -> usize {
    routes
        .iter()
        .filter(|row| str_at(row, &["destination"]) == Some(destination.as_str()))
        .count()
}

fn route_is_well_formed(route: &Value) -> bool {
    [
        "source",
        "failure_id",
        "failure_class",
        "severity",
        "destination",
        "action",
        "receipt_type",
    ]
    .iter()
    .all(|field| str_at(route, &[*field]).is_some())
}

fn build_report(regression_path: &str, issue_drafts_path: &str) -> Value {
    let regression = read_json(regression_path);
    let issue_drafts = read_json(issue_drafts_path);
    let mut routes = Vec::new();
    push_issue_routes(&mut routes, &issue_drafts);
    push_regression_routes(&mut routes, &regression);
    let raw_route_count = routes.len();
    let mut routes = dedupe_routes(routes);
    annotate_issue_readiness(&mut routes);
    let deduped_route_count = routes.len();
    let issue_candidates = issue_candidate_routes(&routes);
    let malformed = routes
        .iter()
        .filter(|row| !route_is_well_formed(row))
        .count();
    let regression_failed = !bool_at(&regression, &["ok"]);
    let regression_blocked = !regression_failed
        || routes
            .iter()
            .any(|row| str_at(row, &["destination"]) == Some("kernel_block"));
    let checks = vec![
        json!({"id": "eval_feedback_route_coverage_contract", "ok": malformed == 0, "detail": format!("routes={};malformed={malformed}", routes.len())}),
        json!({"id": "eval_feedback_destination_set_contract", "ok": routes.iter().all(|row| matches!(str_at(row, &["destination"]), Some("control_plane_retry" | "gateway_quarantine" | "kernel_block"))), "detail": "destinations=control_plane_retry,gateway_quarantine,kernel_block"}),
        json!({"id": "eval_regression_release_block_contract", "ok": regression_blocked, "detail": format!("regression_failed={regression_failed};kernel_block_routes={}", destination_count(&routes, EnforcementDestination::KernelBlock))}),
        json!({"id": "eval_feedback_route_dedupe_contract", "ok": deduped_route_count <= raw_route_count, "detail": format!("raw_routes={raw_route_count};deduped_routes={deduped_route_count}")}),
        json!({"id": "eval_feedback_occurrence_accounting_contract", "ok": occurrence_total(&routes) as usize == raw_route_count, "detail": format!("raw_routes={raw_route_count};clustered_occurrences={}", occurrence_total(&routes))}),
        json!({"id": "eval_issue_candidate_actionability_contract", "ok": issue_candidates.iter().all(issue_candidate_actionability_ok), "detail": format!("issue_candidates={};malformed_actionability={}", issue_candidates.len(), issue_candidates.iter().filter(|row| !issue_candidate_actionability_ok(row)).count())}),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    json!({
        "type": "eval_feedback_router",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "summary": {
            "route_count": routes.len(),
            "raw_route_count": raw_route_count,
            "deduped_route_count": deduped_route_count,
            "clustered_occurrence_count": occurrence_total(&routes),
            "dedupe_compression_ratio": if raw_route_count == 0 { 1.0 } else { deduped_route_count as f64 / raw_route_count as f64 },
            "release_blocking_occurrence_count": severity_occurrence_total(&routes, "critical") + severity_occurrence_total(&routes, "release_blocking"),
            "issue_candidate_ready_count": issue_candidates.len(),
            "issue_candidate_ready": !issue_candidates.is_empty(),
            "issue_candidate_actionability_ok": issue_candidates.iter().all(issue_candidate_actionability_ok),
            "issue_candidate_actionability_failure_count": issue_candidates.iter().filter(|row| !issue_candidate_actionability_ok(row)).count(),
            "top_issue_candidate": issue_candidates.first().cloned().unwrap_or_else(|| json!({})),
            "top_issue_candidate_action": issue_candidates
                .first()
                .and_then(|row| str_at(row, &["recommended_action"]))
                .unwrap_or(""),
            "top_issue_candidate_fingerprint": issue_candidates
                .first()
                .and_then(|row| str_at(row, &["route_fingerprint"]))
                .unwrap_or(""),
            "top_issue_candidate_owner": issue_candidates
                .first()
                .and_then(|row| str_at(row, &["owner"]))
                .unwrap_or(""),
            "top_issue_candidate_release_gate_effect": issue_candidates
                .first()
                .and_then(|row| str_at(row, &["release_gate_effect"]))
                .unwrap_or(""),
            "top_issue_candidate_escalation_tier": issue_candidates
                .first()
                .and_then(|row| str_at(row, &["escalation_tier"]))
                .unwrap_or(""),
            "top_issue_candidate_closing_evidence": issue_candidates
                .first()
                .and_then(|row| str_at(row, &["closing_evidence_required"]))
                .unwrap_or(""),
            "top_issue_candidate_triage_queue": issue_candidates
                .first()
                .and_then(|row| str_at(row, &["triage_queue"]))
                .unwrap_or(""),
            "top_issue_candidate_lifecycle_state": issue_candidates
                .first()
                .and_then(|row| str_at(row, &["issue_lifecycle_state"]))
                .unwrap_or(""),
            "top_issue_candidate_source_artifact_count": issue_candidates
                .first()
                .and_then(|row| row.get("source_artifacts"))
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(0),
            "top_issue_candidate_closure_verification_command": issue_candidates
                .first()
                .and_then(|row| str_at(row, &["closure_verification_command"]))
                .unwrap_or(""),
            "p0_issue_candidate_count": priority_band_count(&issue_candidates, "p0"),
            "release_blocking_issue_candidate_count": issue_candidates
                .iter()
                .filter(|row| matches!(str_at(row, &["severity"]), Some("critical" | "release_blocking")))
                .count(),
            "issue_candidate_priority_bands": {
                "p0": priority_band_count(&issue_candidates, "p0"),
                "p1": priority_band_count(&issue_candidates, "p1"),
                "p2": priority_band_count(&issue_candidates, "p2"),
                "p3": priority_band_count(&issue_candidates, "p3"),
            },
            "severity_counts": {
                "critical": severity_count(&routes, "critical"),
                "release_blocking": severity_count(&routes, "release_blocking"),
                "high": severity_count(&routes, "high"),
                "medium": severity_count(&routes, "medium"),
                "low": severity_count(&routes, "low"),
            },
            "severity_occurrences": {
                "critical": severity_occurrence_total(&routes, "critical"),
                "release_blocking": severity_occurrence_total(&routes, "release_blocking"),
                "high": severity_occurrence_total(&routes, "high"),
                "medium": severity_occurrence_total(&routes, "medium"),
                "low": severity_occurrence_total(&routes, "low"),
            },
            "destinations": {
                "control_plane_retry": destination_count(&routes, EnforcementDestination::ControlPlaneRetry),
                "gateway_quarantine": destination_count(&routes, EnforcementDestination::GatewayQuarantine),
                "kernel_block": destination_count(&routes, EnforcementDestination::KernelBlock),
            },
            "control_plane_retry": destination_count(&routes, EnforcementDestination::ControlPlaneRetry),
            "gateway_quarantine": destination_count(&routes, EnforcementDestination::GatewayQuarantine),
            "kernel_block": destination_count(&routes, EnforcementDestination::KernelBlock),
            "eval_release_gate": if regression_failed { "blocked" } else { "not_blocked" },
        },
        "checks": checks,
        "routes": routes,
        "issue_candidates": issue_candidates,
        "issue_candidate_policy": {
            "ready_when": [
                "destination=kernel_block",
                "destination=gateway_quarantine",
                "severity=critical|release_blocking|high",
                "occurrence_count>=2"
            ],
            "dedupe_before_issue_synthesis": true,
            "auto_apply_allowed": false
        },
        "sources": {
            "eval_regression_guard": regression_path,
            "eval_issue_drafts": issue_drafts_path,
        }
    })
}

fn markdown(report: &Value) -> String {
    format!(
        "# Eval Feedback Router (Current)\n\n- generated_at: {}\n- ok: {}\n- route_count: {}\n- raw_route_count: {}\n- deduped_route_count: {}\n- release_blocking_occurrence_count: {}\n- issue_candidate_ready_count: {}\n- control_plane_retry: {}\n- gateway_quarantine: {}\n- kernel_block: {}\n- eval_release_gate: {}\n",
        str_at(report, &["generated_at"]).unwrap_or(""),
        bool_at(report, &["ok"]),
        report.pointer("/summary/route_count").and_then(Value::as_u64).unwrap_or(0),
        report.pointer("/summary/raw_route_count").and_then(Value::as_u64).unwrap_or(0),
        report.pointer("/summary/deduped_route_count").and_then(Value::as_u64).unwrap_or(0),
        report.pointer("/summary/release_blocking_occurrence_count").and_then(Value::as_u64).unwrap_or(0),
        report.pointer("/summary/issue_candidate_ready_count").and_then(Value::as_u64).unwrap_or(0),
        report.pointer("/summary/control_plane_retry").and_then(Value::as_u64).unwrap_or(0),
        report.pointer("/summary/gateway_quarantine").and_then(Value::as_u64).unwrap_or(0),
        report.pointer("/summary/kernel_block").and_then(Value::as_u64).unwrap_or(0),
        str_at(report, &["summary", "eval_release_gate"]).unwrap_or("unknown"),
    )
}

pub fn run_eval_feedback_router(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let regression_path =
        parse_flag(args, "regression").unwrap_or_else(|| DEFAULT_REGRESSION_PATH.to_string());
    let issue_drafts_path =
        parse_flag(args, "issues").unwrap_or_else(|| DEFAULT_ISSUE_DRAFTS_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let report = build_report(&regression_path, &issue_drafts_path);
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_text(&markdown_path, &markdown(&report)).is_ok();
    if !write_ok {
        eprintln!("eval_feedback_router: failed to write outputs");
        return 2;
    }
    let _ = writeln!(
        io::stdout(),
        "{}",
        serde_json::to_string(&report).unwrap_or_default()
    );
    if strict && !bool_at(&report, &["ok"]) {
        return 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_issue_classes_to_retry_quarantine_and_kernel_block() {
        assert_eq!(
            destination_for_issue_class("wrong_tool_selection"),
            EnforcementDestination::ControlPlaneRetry
        );
        assert_eq!(
            destination_for_issue_class("invalid_schema_response"),
            EnforcementDestination::GatewayQuarantine
        );
        assert_eq!(
            destination_for_issue_class("hallucination"),
            EnforcementDestination::KernelBlock
        );
    }

    #[test]
    fn failed_regression_artifact_routes_to_kernel_block() {
        let mut routes = Vec::new();
        push_regression_routes(
            &mut routes,
            &json!({"ok": false, "failures": [{"id": "eval_release_artifact_not_passing", "artifact": "eval_quality_gate_v1"}]}),
        );
        assert_eq!(routes.len(), 1);
        assert_eq!(str_at(&routes[0], &["destination"]), Some("kernel_block"));
        assert_eq!(
            str_at(&routes[0], &["action"]),
            Some("block_release_or_runtime_escalation")
        );
    }

    #[test]
    fn repeated_issue_classes_cluster_into_one_route() {
        let routes = dedupe_routes(vec![
            json!({
                "source": "eval_issue_drafts",
                "failure_id": "a",
                "failure_class": "wrong_tool_selection",
                "severity": "high",
                "destination": "control_plane_retry",
                "action": "retry_with_trace_and_probe_context",
                "receipt_type": "control_plane_retry_event"
            }),
            json!({
                "source": "eval_issue_drafts",
                "failure_id": "b",
                "failure_class": "wrong_tool_selection",
                "severity": "medium",
                "destination": "control_plane_retry",
                "action": "retry_with_trace_and_probe_context",
                "receipt_type": "control_plane_retry_event"
            }),
        ]);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0]["occurrence_count"].as_u64(), Some(2));
        assert_eq!(str_at(&routes[0], &["severity"]), Some("high"));
        assert!(route_issue_candidate_ready(&routes[0]));
    }

    #[test]
    fn single_high_severity_route_waits_for_repeated_stable_signature() {
        let route = json!({
            "source": "eval_issue_drafts",
            "failure_id": "single",
            "failure_class": "wrong_tool_selection",
            "severity": "high",
            "destination": "control_plane_retry",
            "action": "retry_with_trace_and_probe_context",
            "receipt_type": "control_plane_retry_event",
            "occurrence_count": 1
        });

        assert!(!route_issue_candidate_ready(&route));
        assert_eq!(
            issue_candidate_reason(&route),
            "awaiting_repeated_stable_signature"
        );
    }
}
