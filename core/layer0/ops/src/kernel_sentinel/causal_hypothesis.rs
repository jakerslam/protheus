// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    kernel_sentinel_failure_level_for_finding, kernel_sentinel_root_frame_for_finding,
    KernelSentinelFinding,
};
use serde_json::{json, Value};

const CAUSAL_HYPOTHESIS_SCHEMA_VERSION: u32 = 1;
const DEFAULT_CAUSAL_HYPOTHESIS_LIMIT: usize = 8;

struct CausalPattern {
    id: &'static str,
    mechanism: &'static str,
    root_cause: &'static str,
    systemic_cause: &'static str,
    falsification_probe: &'static str,
    expected_if_true: &'static str,
    expected_if_false: &'static str,
    confidence_base: u8,
    causal_power: u8,
    tags: &'static [&'static str],
    missing_evidence: &'static [&'static str],
}

pub fn kernel_sentinel_causal_hypothesis_model() -> Value {
    json!({
        "type": "kernel_sentinel_causal_hypothesis_model",
        "schema_version": CAUSAL_HYPOTHESIS_SCHEMA_VERSION,
        "purpose": "rank Sentinel root-cause hypotheses as testable causal arguments, not prose summaries",
        "causal_ladder": ["symptom", "immediate_mechanism", "violated_invariant", "likely_root_cause", "systemic_or_process_cause"],
        "required_quality_fields": ["support_evidence", "counter_evidence", "missing_evidence", "confidence_percent", "falsification_probe", "causal_power_score"],
        "promotion_policy": "hypotheses need evidence, freshness/recurrence support, falsification probes, and owner/actionability before issue/TODO promotion"
    })
}

pub(super) fn root_cause_hypothesis_text(
    finding: &KernelSentinelFinding,
    evidence_refs: &[String],
    invariant: &str,
    semantic_frame: &Value,
) -> String {
    let pattern = classify_pattern(finding, evidence_refs);
    let fallback_root_frame = kernel_sentinel_root_frame_for_finding(finding);
    let root_frame = semantic_frame["root_frame"]
        .as_str()
        .unwrap_or(&fallback_root_frame);
    format!(
        "{}: `{}` likely arises because {}. It violates `{}` under `{}` and should be tested with `{}`.",
        pattern.id,
        finding.fingerprint,
        pattern.root_cause,
        invariant,
        root_frame,
        pattern.falsification_probe
    )
}

pub(super) fn build_kernel_sentinel_causal_hypotheses(
    findings: &[KernelSentinelFinding],
    architectural_incident_report: &Value,
    args: &[String],
) -> Value {
    let limit = option_usize(args, "--causal-hypothesis-limit", DEFAULT_CAUSAL_HYPOTHESIS_LIMIT);
    let mut hypotheses = findings
        .iter()
        .filter(|finding| finding.status == "open")
        .filter(|finding| !finding.evidence.is_empty())
        .map(|finding| hypothesis_for_finding(finding, architectural_incident_report))
        .collect::<Vec<_>>();
    hypotheses.sort_by(|a, b| {
        score_for_sort(b)
            .cmp(&score_for_sort(a))
            .then_with(|| id_for_sort(a).cmp(&id_for_sort(b)))
    });
    let candidate_count = hypotheses.len();
    hypotheses.truncate(limit);
    let quality_failures = quality_failures(&hypotheses);
    json!({
        "ok": quality_failures.is_empty(),
        "type": "kernel_sentinel_causal_hypothesis_synthesis",
        "schema_version": CAUSAL_HYPOTHESIS_SCHEMA_VERSION,
        "model": kernel_sentinel_causal_hypothesis_model(),
        "hypothesis_limit": limit,
        "candidate_hypothesis_count": candidate_count,
        "hypothesis_count": hypotheses.len(),
        "quality_gate": {
            "ok": quality_failures.is_empty(),
            "failure_count": quality_failures.len(),
            "failures": quality_failures,
            "policy": "do_not_promote_root_cause_hypotheses_without_support_counter_evidence_missing_evidence_and_falsification_probe"
        },
        "top_hypotheses": hypotheses,
        "pattern_catalog": pattern_catalog(),
    })
}

fn hypothesis_for_finding(
    finding: &KernelSentinelFinding,
    architectural_incident_report: &Value,
) -> Value {
    let pattern = classify_pattern(finding, &finding.evidence);
    let failure_level = kernel_sentinel_failure_level_for_finding(finding);
    let root_frame = kernel_sentinel_root_frame_for_finding(finding);
    let violated_invariant = violated_invariant_guess(finding);
    let support_evidence = support_evidence(finding, &pattern);
    let counter_evidence = counter_evidence(finding);
    let missing_evidence = missing_evidence(&pattern, finding);
    let confidence = confidence_percent(finding, &pattern, support_evidence.len(), missing_evidence.len());
    let causal_power = causal_power_score(&pattern, finding, architectural_incident_report);
    json!({
        "id": format!("causal_hypothesis:{}", finding.fingerprint),
        "schema_version": CAUSAL_HYPOTHESIS_SCHEMA_VERSION,
        "finding_id": finding.id,
        "finding_fingerprint": finding.fingerprint,
        "pattern": pattern.id,
        "pattern_tags": pattern.tags,
        "failure_level": failure_level.code(),
        "root_frame": root_frame,
        "causal_ladder": {
            "symptom": finding.summary,
            "immediate_mechanism": pattern.mechanism,
            "violated_invariant": violated_invariant,
            "likely_root_cause": pattern.root_cause,
            "systemic_or_process_cause": pattern.systemic_cause,
        },
        "support_evidence": support_evidence,
        "counter_evidence": counter_evidence,
        "missing_evidence": missing_evidence,
        "confidence_percent": confidence,
        "causal_power_score": causal_power,
        "falsification_probe": {
            "probe": pattern.falsification_probe,
            "expected_if_true": pattern.expected_if_true,
            "expected_if_false": pattern.expected_if_false,
        },
        "promotion_ready": confidence >= 70 && causal_power >= 60,
        "next_action": format!("Run `{}` before promoting this as a TODO or issue root cause.", pattern.falsification_probe),
    })
}

fn classify_pattern(finding: &KernelSentinelFinding, evidence_refs: &[String]) -> CausalPattern {
    let text = joined_text(finding, evidence_refs);
    if any(&text, &["os_reason_codesigning", "codesign", "exit 137", "code-signing", "codesigning"]) {
        return pattern(
            "installed_runtime_identity_invalid",
            "the operating system kills the installed runtime before Kernel ops can emit a receipt",
            "the installed binary identity/signature no longer matches macOS launch policy",
            "install/update does not force a current executable identity before launchd handoff",
            "codesign --verify --verbose=2 /Users/jay/.local/bin/infring-ops-new && /Users/jay/.local/bin/infring-ops --help",
            "codesign reports invalid or the command exits 137 / launchd reports OS_REASON_CODESIGNING",
            "the command returns a deterministic CLI receipt and launchd no longer reports code-signing kills",
            82,
            90,
            &["runtime_identity", "launchd", "gateway_startup"],
            &["installer_update_event", "launchd_last_exit_reason"],
        );
    }
    if any(&text, &["authority ghost", "authority_ghost", "truth leak", "removed_authority_syntax", "authority residue"]) {
        return pattern(
            "authority_shape_residue",
            "old authority behavior survives after the visible syntax or route was removed",
            "authority was removed cosmetically while data shape, fallback affordances, or compatibility paths still encode it",
            "deauthority work did not include shape-level, behavior-level, and evidence-level enforcement",
            "run the boundary/ownership guard for the affected route and inspect whether fallback data still carries authority-shaped fields",
            "guards or runtime traces show authority-shaped fields/actions outside the owning layer",
            "the syntax, data shape, fallback path, and runtime trace all agree authority is gone",
            78,
            92,
            &["authority_residue", "policy_truth", "architecture"],
            &["shape_guard_output", "fallback_route_trace", "owner_policy_ref"],
        );
    }
    if any(&text, &["healthz", "listener_absent", "stale_duplicate", "dashboard", "gateway"]) {
        return pattern(
            "gateway_lifecycle_truth_contradiction",
            "configured gateway health disagrees with process, listener, or alternate-route observations",
            "gateway lifecycle truth is fragmented across healthz, PID files, watchdog state, and fallback ports",
            "startup success is not gated on one durable listener/health receipt for the configured route",
            "infring gateway status --dashboard-open=0 && lsof -nP -iTCP:4173 -sTCP:LISTEN",
            "status, listener, PID, or watchdog facts disagree for the configured dashboard route",
            "all configured-route facts agree and stale alternate listeners are either absent or declared non-authoritative",
            76,
            86,
            &["gateway", "lifecycle", "fragmented_observability"],
            &["configured_listener_probe", "watchdog_state", "pid_file_state"],
        );
    }
    if any(&text, &["mini os", "shell", "full_state", "mirror", "projection", "browser heap"]) {
        return pattern(
            "projection_surface_became_runtime_owner",
            "a projection surface retains or reconstructs runtime-owned state",
            "presentation code still owns enough runtime shape to behave like a second control plane",
            "Gateway/Shell projection budgets are not enforced at the live route boundary",
            "run shell projection and payload-budget guards against the affected route, then inspect returned default fields",
            "default payloads include raw/full runtime state or browser storage grows with complete histories",
            "default payloads are bounded projections with detail refs and no runtime authority fields",
            74,
            88,
            &["projection", "shell_boundary", "bounded_payload"],
            &["live_payload_sample", "heap_budget_probe", "detail_route_receipt"],
        );
    }
    if any(&text, &["oversized", "report size", "noise", "raw evidence", "bounded report"]) {
        return pattern(
            "observability_noise_release",
            "raw evidence volume escapes into operator-facing reports",
            "the report release path lacks a strict quality/budget filter before writing current artifacts",
            "Observability has not separated raw streams from compact operator summaries at the source",
            "run kernel-sentinel report with a small byte budget and inspect report_budget.within_budget",
            "operator report embeds raw evidence or exceeds budget",
            "operator report remains a bounded index with refs to raw streams",
            72,
            80,
            &["observability", "report_budget", "quality_filter"],
            &["report_budget_artifact", "raw_stream_refs"],
        );
    }
    if any(&text, &["empty reply", "empty direct reply", "final llm", "finalization", "no response"]) {
        return pattern(
            "response_finalization_gap",
            "workflow/tool progress does not synthesize into a user-visible final response",
            "the finalization contract is not preserving enough phase evidence to distinguish tool wait, LLM empty output, and fallback suppression",
            "workflow response provenance is not fully carried through the final visible-output gate",
            "run the workflow empty-reply regression and inspect final_llm_response plus visible_response provenance",
            "trace shows pending tool or empty LLM phase without a synthesized final response",
            "trace shows a finalized response or a non-chat diagnostic channel with no silent drop",
            70,
            76,
            &["workflow_finalization", "visible_response", "trace"],
            &["phase_trace", "final_llm_status", "visible_response_origin"],
        );
    }
    if any(&text, &["receipt", "missing_receipt", "receipt_integrity"]) {
        return pattern(
            "receipt_integrity_gap",
            "runtime mutation or release evidence lacks a matching authoritative receipt",
            "the action path can complete without durable proof material in the Kernel receipt stream",
            "receipt completeness is not a hard precondition for the affected action family",
            "run Kernel Sentinel receipt-completeness checks against the action and receipt streams",
            "the action exists without matching receipt_for/action_for linkage",
            "every action has a matching receipt and the Sentinel finding family disappears",
            76,
            78,
            &["receipts", "kernel_truth", "release_gate"],
            &["action_stream_ref", "receipt_stream_ref"],
        );
    }
    if any(&text, &["boundedness", "rss", "queue", "backpressure", "memory"]) {
        return pattern(
            "boundedness_budget_regression",
            "resource or queue metrics exceed declared budget",
            "the runtime lacks a current budget comparison or mitigation for this workload profile",
            "boundedness evidence is not yet enforced as an operator-facing hard budget for this path",
            "run the boundedness profile replay and compare current RSS/queue/recovery values to budget",
            "current metrics exceed budget or lack baseline comparison",
            "metrics stay within budget and regressions fail closed before release",
            68,
            72,
            &["boundedness", "queue", "resource_budget"],
            &["profile_budget", "current_metric_sample", "baseline_delta"],
        );
    }
    pattern(
        "semantic_frame_default",
        "the finding indicates a Sentinel-class failure but lacks a stronger matched root-cause pattern",
        "the failure should be framed by its category, failure level, and violated invariant until more evidence arrives",
        "the evidence stream lacks enough mechanism-specific probes to choose a stronger causal pattern",
        "collect a focused diagnostic run for the finding fingerprint and rerun Kernel Sentinel report",
        "new diagnostic evidence selects a stronger pattern or contradicts this default frame",
        "no stronger mechanism appears and the finding resolves via category-level remediation",
        54,
        50,
        &["default", "needs_diagnostic_probe"],
        &["focused_diagnostic_run", "counter_evidence_probe"],
    )
}

fn pattern(
    id: &'static str,
    mechanism: &'static str,
    root_cause: &'static str,
    systemic_cause: &'static str,
    falsification_probe: &'static str,
    expected_if_true: &'static str,
    expected_if_false: &'static str,
    confidence_base: u8,
    causal_power: u8,
    tags: &'static [&'static str],
    missing_evidence: &'static [&'static str],
) -> CausalPattern {
    CausalPattern {
        id,
        mechanism,
        root_cause,
        systemic_cause,
        falsification_probe,
        expected_if_true,
        expected_if_false,
        confidence_base,
        causal_power,
        tags,
        missing_evidence,
    }
}

fn joined_text(finding: &KernelSentinelFinding, evidence_refs: &[String]) -> String {
    format!(
        "{} {} {} {} {:?} {}",
        finding.fingerprint,
        finding.summary,
        finding.recommended_action,
        evidence_refs.join(" "),
        finding.category,
        finding.status
    )
    .to_ascii_lowercase()
}

fn any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn support_evidence(finding: &KernelSentinelFinding, pattern: &CausalPattern) -> Vec<String> {
    let mut rows = Vec::<String>::new();
    for reference in &finding.evidence {
        if rows.len() >= 4 {
            break;
        }
        rows.push(reference.clone());
    }
    if rows.len() < 4 {
        rows.push(format!("summary://{}", compact(&finding.summary, 96)));
    }
    if rows.len() < 4 {
        rows.push(format!("pattern://{}", pattern.id));
    }
    rows
}

fn counter_evidence(finding: &KernelSentinelFinding) -> Vec<String> {
    let text = joined_text(finding, &finding.evidence);
    let mut rows = Vec::new();
    if any(&text, &["alternate_healthz_ready", "5173", "backend healthy"]) {
        rows.push("alternate route is healthy, so the root is scoped to configured route ownership rather than total runtime death".to_string());
    }
    if any(&text, &["waived", "expected degradation", "controlled violation"]) {
        rows.push("finding may be expected or waived; verify waiver freshness before promotion".to_string());
    }
    if rows.is_empty() {
        rows.push("no direct contradiction observed yet; keep falsification probe mandatory".to_string());
    }
    rows
}

fn missing_evidence(pattern: &CausalPattern, finding: &KernelSentinelFinding) -> Vec<String> {
    let text = joined_text(finding, &finding.evidence);
    pattern
        .missing_evidence
        .iter()
        .filter(|row| !text.contains(&row.replace('_', " ")))
        .map(|row| row.to_string())
        .collect()
}

fn violated_invariant_guess(finding: &KernelSentinelFinding) -> String {
    for reference in &finding.evidence {
        if let Some((_, tail)) = reference.split_once("invariant://") {
            return tail
                .split(['/', ';', '?', '#'])
                .next()
                .unwrap_or("unknown_invariant")
                .to_string();
        }
    }
    let text = joined_text(finding, &finding.evidence);
    if text.contains("healthz") || text.contains("gateway") {
        "gateway_success_requires_durable_listener".to_string()
    } else if text.contains("authority") || text.contains("truth") {
        "authority_removed_at_behavior_and_shape_level".to_string()
    } else if text.contains("receipt") {
        "every_mutation_has_authoritative_receipt".to_string()
    } else {
        format!("{:?}_invariant", finding.category).to_ascii_lowercase()
    }
}

fn confidence_percent(
    finding: &KernelSentinelFinding,
    pattern: &CausalPattern,
    support_count: usize,
    missing_count: usize,
) -> u8 {
    let severity_bonus = match finding.severity {
        super::KernelSentinelSeverity::Critical => 8,
        super::KernelSentinelSeverity::High => 5,
        super::KernelSentinelSeverity::Medium => 2,
        super::KernelSentinelSeverity::Low => 0,
    };
    pattern
        .confidence_base
        .saturating_add((support_count.min(4) as u8) * 3)
        .saturating_add(severity_bonus)
        .saturating_sub((missing_count.min(5) as u8) * 4)
        .min(96)
}

fn causal_power_score(
    pattern: &CausalPattern,
    finding: &KernelSentinelFinding,
    architectural_incident_report: &Value,
) -> u8 {
    let incident_bonus = architectural_incident_report
        .get("incident_count")
        .and_then(Value::as_u64)
        .or_else(|| {
            architectural_incident_report
                .get("incidents")
                .and_then(Value::as_array)
                .map(|rows| rows.len() as u64)
        })
        .unwrap_or(0)
        .min(3) as u8
        * 4;
    let level_bonus = kernel_sentinel_failure_level_for_finding(finding).priority() * 3;
    pattern
        .causal_power
        .saturating_add(incident_bonus)
        .saturating_add(level_bonus)
        .min(100)
}

fn quality_failures(hypotheses: &[Value]) -> Vec<Value> {
    hypotheses
        .iter()
        .filter_map(|hypothesis| {
            let mut reasons = Vec::new();
            if hypothesis["support_evidence"].as_array().map(Vec::len).unwrap_or(0) == 0 {
                reasons.push("missing_support_evidence");
            }
            if hypothesis["counter_evidence"].as_array().map(Vec::len).unwrap_or(0) == 0 {
                reasons.push("missing_counter_evidence");
            }
            if hypothesis["confidence_percent"].as_u64().unwrap_or(0) < 60 {
                reasons.push("low_confidence");
            }
            if hypothesis["causal_power_score"].as_u64().unwrap_or(0) < 50 {
                reasons.push("low_causal_power");
            }
            if hypothesis["falsification_probe"]["probe"]
                .as_str()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                reasons.push("missing_falsification_probe");
            }
            (!reasons.is_empty()).then(|| {
                json!({
                    "hypothesis_id": hypothesis["id"].clone(),
                    "reasons": reasons,
                })
            })
        })
        .collect()
}

fn score_for_sort(row: &Value) -> u64 {
    row["causal_power_score"].as_u64().unwrap_or(0) * 1000
        + row["confidence_percent"].as_u64().unwrap_or(0)
}

fn id_for_sort(row: &Value) -> String {
    row["id"].as_str().unwrap_or("").to_string()
}

fn option_usize(args: &[String], name: &str, fallback: usize) -> usize {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).and_then(|raw| raw.parse::<usize>().ok()))
        .unwrap_or(fallback)
}

fn compact(value: &str, max: usize) -> String {
    let mut out = value.trim().replace('\n', " ");
    if out.len() > max {
        out.truncate(max);
    }
    out
}

fn pattern_catalog() -> Vec<Value> {
    ["installed_runtime_identity_invalid", "gateway_lifecycle_truth_contradiction", "authority_shape_residue", "projection_surface_became_runtime_owner", "observability_noise_release", "response_finalization_gap", "receipt_integrity_gap", "boundedness_budget_regression", "semantic_frame_default"].iter().map(|id| json!({"id": id, "kind": "causal_pattern"})).collect()
}

#[cfg(test)]
#[path = "causal_hypothesis_tests.rs"]
mod causal_hypothesis_tests;
