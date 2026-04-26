use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_POLICY_PATH: &str = "surface/orchestration/config/live_eval_policy.json";
const DEFAULT_STREAM_PATH: &str = "local/state/ops/eval/live_eval_stream.jsonl";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/live_eval_current.json";
const DEFAULT_LATEST_PATH: &str = "artifacts/live_eval_latest.json";
const DEFAULT_REPORT_PATH: &str = "local/workspace/reports/LIVE_EVAL_CURRENT.md";
const SOURCE_HEALTH_ISSUE_MIN_OCCURRENCES: u64 = 2;

#[derive(Debug, Clone, Deserialize)]
struct LiveEvalPolicy {
    schema_version: u32,
    sample_sources: Vec<SampleSourcePolicy>,
    drift_thresholds: DriftThresholds,
    mitigations: Vec<MitigationPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
struct SampleSourcePolicy {
    id: String,
    path: String,
    required: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct DriftThresholds {
    kernel_block_routes_max: u64,
    gateway_quarantine_routes_max: u64,
    control_plane_retry_routes_max: u64,
    missing_required_sources_max: u64,
    #[serde(default)]
    failed_required_sources_max: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct MitigationPolicy {
    id: String,
    trigger: String,
    action: String,
}

#[derive(Debug, Clone, Serialize)]
struct SourceObservation {
    id: String,
    path: String,
    present: bool,
    required: bool,
    ok: Option<bool>,
    summary: Value,
}

#[derive(Debug, Clone, Serialize)]
struct DriftFinding {
    id: String,
    severity: String,
    threshold: u64,
    observed: u64,
    mitigation_action: String,
}

#[derive(Debug, Clone, Serialize)]
struct CheckRow {
    id: String,
    ok: bool,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct LiveEvalReport {
    ok: bool,
    r#type: String,
    schema_version: u32,
    generated_unix_seconds: u64,
    stream_path: String,
    sources: Vec<SourceObservation>,
    source_health: Value,
    metrics: BTreeMap<String, u64>,
    drift_findings: Vec<DriftFinding>,
    mitigations: Vec<MitigationPolicy>,
    checks: Vec<CheckRow>,
}

pub fn run_continuous_eval(args: &[String]) -> i32 {
    let strict = flag_value(args, "--strict").unwrap_or_else(|| "0".to_string()) == "1";
    let policy_path =
        flag_value(args, "--policy").unwrap_or_else(|| DEFAULT_POLICY_PATH.to_string());
    let stream_path =
        flag_value(args, "--stream").unwrap_or_else(|| DEFAULT_STREAM_PATH.to_string());
    let out_path = flag_value(args, "--out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let latest_path =
        flag_value(args, "--latest").unwrap_or_else(|| DEFAULT_LATEST_PATH.to_string());
    let report_path =
        flag_value(args, "--report").unwrap_or_else(|| DEFAULT_REPORT_PATH.to_string());

    let policy = match load_policy(&policy_path) {
        Ok(policy) => policy,
        Err(err) => {
            eprintln!("continuous_eval: failed to read policy {policy_path}: {err}");
            return 1;
        }
    };

    let report = build_live_eval_report(&policy, &stream_path);
    let wrote_all = write_json(&out_path, &report)
        && write_json(&latest_path, &report)
        && append_jsonl(&stream_path, &report)
        && write_markdown(&report_path, &report);

    if !wrote_all {
        eprintln!("continuous_eval: failed to write one or more outputs");
        return 1;
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );

    if strict && !report.ok {
        return 1;
    }
    0
}

fn load_policy(path: &str) -> Result<LiveEvalPolicy, String> {
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| err.to_string())
}

fn read_jsonl(path: &str) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn build_live_eval_report(policy: &LiveEvalPolicy, stream_path: &str) -> LiveEvalReport {
    let generated_unix_seconds = now_unix_seconds();
    let sources: Vec<SourceObservation> =
        policy.sample_sources.iter().map(observe_source).collect();
    let previous_reports = read_jsonl(stream_path);
    let source_health = source_health(&sources, &previous_reports);
    let metrics = derive_metrics(&sources);
    let drift_findings = drift_findings(policy, &metrics);
    let mitigations = policy
        .mitigations
        .iter()
        .filter(|mitigation| {
            drift_findings
                .iter()
                .any(|finding| finding.id == mitigation.trigger)
        })
        .cloned()
        .collect::<Vec<_>>();
    let checks = vec![
        CheckRow {
            id: "live_eval_policy_schema_contract".to_string(),
            ok: policy.schema_version == 1,
            detail: format!("schema_version={}", policy.schema_version),
        },
        CheckRow {
            id: "live_eval_sources_present_contract".to_string(),
            ok: metrics
                .get("missing_required_sources")
                .copied()
                .unwrap_or(0)
                <= policy.drift_thresholds.missing_required_sources_max,
            detail: format!(
                "missing_required_sources={}",
                metrics
                    .get("missing_required_sources")
                    .copied()
                    .unwrap_or(0)
            ),
        },
        CheckRow {
            id: "live_eval_required_sources_passing_contract".to_string(),
            ok: metrics
                .get("failed_required_sources")
                .copied()
                .unwrap_or(0)
                <= policy.drift_thresholds.failed_required_sources_max.unwrap_or(0),
            detail: format!(
                "failed_required_sources={}",
                metrics
                    .get("failed_required_sources")
                    .copied()
                    .unwrap_or(0)
            ),
        },
        CheckRow {
            id: "live_eval_source_health_not_critical_contract".to_string(),
            ok: source_health
                .get("state")
                .and_then(Value::as_str)
                != Some("critical"),
            detail: format!("source_health={}", source_health),
        },
        CheckRow {
            id: "live_eval_source_issue_candidate_actionability_contract".to_string(),
            ok: source_health_issue_candidate_ok(&source_health),
            detail: format!("source_health={}", source_health),
        },
        CheckRow {
            id: "live_eval_stream_append_only_contract".to_string(),
            ok: stream_path.ends_with(".jsonl") && stream_path.starts_with("local/state/ops/eval/"),
            detail: stream_path.to_string(),
        },
        CheckRow {
            id: "live_eval_drift_mitigation_contract".to_string(),
            ok: drift_findings.is_empty() || !mitigations.is_empty(),
            detail: format!(
                "drift_findings={};mitigations={}",
                drift_findings.len(),
                mitigations.len()
            ),
        },
    ];
    let ok = checks.iter().all(|check| check.ok) && drift_findings.is_empty();

    LiveEvalReport {
        ok,
        r#type: "continuous_live_eval".to_string(),
        schema_version: 1,
        generated_unix_seconds,
        stream_path: stream_path.to_string(),
        source_health: source_health.clone(),
        sources,
        metrics,
        drift_findings,
        mitigations,
        checks,
    }
}

fn source_health(sources: &[SourceObservation], previous_reports: &[Value]) -> Value {
    let required_total = sources.iter().filter(|source| source.required).count();
    let required_present = sources
        .iter()
        .filter(|source| source.required && source.present)
        .count();
    let required_passing = sources
        .iter()
        .filter(|source| source.required && source.ok != Some(false) && source.present)
        .count();
    let optional_total = sources.iter().filter(|source| !source.required).count();
    let optional_present = sources
        .iter()
        .filter(|source| !source.required && source.present)
        .count();
    let optional_failed = sources
        .iter()
        .filter(|source| !source.required && source.ok == Some(false))
        .count();
    let missing_required_sources = sources
        .iter()
        .filter(|source| source.required && !source.present)
        .map(|source| source.id.clone())
        .collect::<Vec<_>>();
    let failed_required_sources = sources
        .iter()
        .filter(|source| source.required && source.ok == Some(false))
        .map(|source| source.id.clone())
        .collect::<Vec<_>>();
    let failed_optional_sources = sources
        .iter()
        .filter(|source| !source.required && source.ok == Some(false))
        .map(|source| source.id.clone())
        .collect::<Vec<_>>();
    let health_state = if required_present < required_total || required_passing < required_total {
        "critical"
    } else if optional_failed > 0 {
        "degraded"
    } else {
        "healthy"
    };
    let primary_blocker =
        source_health_blocker(required_total, required_present, required_passing, optional_failed);
    let blocking_source_count = missing_required_sources.len() + failed_required_sources.len();
    let dedupe_key = format!("live_eval_source_health:{primary_blocker}:{blocking_source_count}");
    let stable_signature_occurrence_count = if health_state == "critical" {
        source_health_signature_occurrences(previous_reports, dedupe_key.as_str()).saturating_add(1)
    } else {
        0
    };
    let issue_candidate_ready = health_state == "critical"
        && stable_signature_occurrence_count >= SOURCE_HEALTH_ISSUE_MIN_OCCURRENCES;
    json!({
        "state": health_state,
        "primary_blocker": primary_blocker,
        "recovery_action": source_health_recovery_action(required_total, required_present, required_passing, optional_failed),
        "recommended_action": source_health_recommended_action(primary_blocker),
        "operator_next_step": source_health_operator_next_step(primary_blocker),
        "release_gate_effect": source_health_release_gate_effect(health_state),
        "triage_queue": source_health_triage_queue(primary_blocker),
        "blocking_source_count": blocking_source_count,
        "dedupe_key": dedupe_key,
        "stable_signature_occurrence_count": stable_signature_occurrence_count,
        "minimum_issue_candidate_occurrences": SOURCE_HEALTH_ISSUE_MIN_OCCURRENCES,
        "issue_candidate_ready": issue_candidate_ready,
        "issue_candidate": source_health_issue_candidate(
            health_state,
            primary_blocker,
            blocking_source_count,
            stable_signature_occurrence_count,
            issue_candidate_ready,
        ),
        "required_total": required_total,
        "required_present": required_present,
        "required_passing": required_passing,
        "required_presence_ratio": ratio(required_present, required_total),
        "required_passing_ratio": ratio(required_passing, required_total),
        "missing_required_sources": missing_required_sources,
        "failed_required_sources": failed_required_sources,
        "optional_total": optional_total,
        "optional_present": optional_present,
        "optional_failed": optional_failed,
        "failed_optional_sources": failed_optional_sources,
        "optional_presence_ratio": ratio(optional_present, optional_total)
    })
}

fn source_health_signature_occurrences(previous_reports: &[Value], dedupe_key: &str) -> u64 {
    previous_reports
        .iter()
        .filter(|report| {
            report
                .pointer("/source_health/dedupe_key")
                .or_else(|| report.pointer("/source_health/issue_candidate/dedupe_key"))
                .and_then(Value::as_str)
                == Some(dedupe_key)
        })
        .count() as u64
}

fn source_health_blocker(
    required_total: usize,
    required_present: usize,
    required_passing: usize,
    optional_failed: usize,
) -> &'static str {
    if required_present < required_total {
        "missing_required_source"
    } else if required_passing < required_total {
        "failed_required_source"
    } else if optional_failed > 0 {
        "failed_optional_source"
    } else {
        "none"
    }
}

fn source_health_issue_candidate_ok(health: &Value) -> bool {
    if health.get("state").and_then(Value::as_str) != Some("critical") {
        return true;
    }
    let Some(candidate) = health.get("issue_candidate") else {
        return false;
    };
    [
        "source_report",
        "issue_lifecycle_state",
        "source_artifact_policy",
        "type",
        "owner",
        "target_layer",
        "route_to",
        "dedupe_key",
        "triage_queue",
        "release_gate_effect",
        "operator_next_step",
        "closing_evidence_required",
    ]
    .iter()
    .all(|field| candidate.get(*field).and_then(Value::as_str).is_some())
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
        && candidate
            .get("safe_to_auto_file_issue")
            .and_then(Value::as_bool)
            == Some(true)
        && candidate
            .get("safe_to_auto_apply_patch")
            .and_then(Value::as_bool)
            == Some(false)
        && candidate
            .get("human_review_required")
            .and_then(Value::as_bool)
            == Some(true)
        && candidate
            .get("requires_operator_ack")
            .and_then(Value::as_bool)
            == Some(true)
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

fn source_health_recovery_action(
    required_total: usize,
    required_present: usize,
    required_passing: usize,
    optional_failed: usize,
) -> &'static str {
    match source_health_blocker(required_total, required_present, required_passing, optional_failed) {
        "missing_required_source" => "restore_required_eval_artifact_or_collector",
        "failed_required_source" => "repair_required_eval_artifact_before_release",
        "failed_optional_source" => "review_optional_eval_source_for_degradation",
        _ => "none",
    }
}

fn source_health_issue_candidate(
    health_state: &str,
    primary_blocker: &str,
    blocking_source_count: usize,
    stable_signature_occurrence_count: u64,
    issue_candidate_ready: bool,
) -> Value {
    if health_state != "critical" {
        return json!({});
    }
    json!({
        "issue_contract_version": 1,
        "type": "live_eval_source_health_issue_candidate",
        "source_report": "continuous_live_eval",
        "issue_lifecycle_state": "candidate_open",
        "source_artifacts": [
            DEFAULT_OUT_PATH,
            DEFAULT_LATEST_PATH
        ],
        "source_artifact_policy": "local_relative_paths_only",
        "severity": "release_blocking",
        "owner": "surface/orchestration",
        "target_layer": "orchestration",
        "route_to": "eval_issue_synthesis",
        "dedupe_key": format!("live_eval_source_health:{primary_blocker}:{blocking_source_count}"),
        "failure_class": primary_blocker,
        "blocking_source_count": blocking_source_count,
        "stable_signature_occurrence_count": stable_signature_occurrence_count,
        "minimum_issue_candidate_occurrences": SOURCE_HEALTH_ISSUE_MIN_OCCURRENCES,
        "issue_candidate_ready": issue_candidate_ready,
        "issue_candidate_reason": if issue_candidate_ready { "repeated_live_eval_source_health_signature" } else { "awaiting_repeated_stable_signature" },
        "safe_to_auto_file_issue": true,
        "safe_to_auto_apply_patch": false,
        "human_review_required": true,
        "autonomous_mitigation_allowed": false,
        "requires_operator_ack": true,
        "reopen_policy": "reopen_if_required_source_health_regresses",
        "close_on_absence_window": "next_strict_continuous_eval_run",
        "release_gate_effect": "blocks_release_until_required_eval_sources_recover",
        "escalation_tier": "release_blocker",
        "stability": "live_source_health_current",
        "labels": [
            "live-eval",
            "release-gate",
            primary_blocker
        ],
        "recommended_action": source_health_recommended_action(primary_blocker),
        "operator_next_step": source_health_operator_next_step(primary_blocker),
        "triage_queue": source_health_triage_queue(primary_blocker),
        "closing_evidence_required": "continuous eval strict run reports ok=true and all required sources are present/passing",
        "closure_verification_command": "cargo test --manifest-path surface/orchestration/Cargo.toml continuous_eval -- --nocapture",
        "acceptance_criteria": [
            "all required live-eval sources are present",
            "all required live-eval sources report passing status",
            "continuous eval strict mode returns ok=true"
        ]
    })
}

fn source_health_triage_queue(primary_blocker: &str) -> &'static str {
    match primary_blocker {
        "missing_required_source" | "failed_required_source" => "eval_release_blockers",
        "failed_optional_source" => "eval_degradation_watchlist",
        _ => "eval_monitoring",
    }
}

fn source_health_release_gate_effect(health_state: &str) -> &'static str {
    match health_state {
        "critical" => "blocks_release_until_required_eval_sources_recover",
        "degraded" => "allows_release_with_operator_review",
        _ => "none",
    }
}

fn source_health_operator_next_step(primary_blocker: &str) -> &'static str {
    match primary_blocker {
        "missing_required_source" => "restore the collector or artifact path named in missing_required_sources",
        "failed_required_source" => "inspect failed_required_sources and route the failing artifact through eval issue synthesis",
        "failed_optional_source" => "triage failed_optional_sources without blocking release promotion",
        _ => "continue monitoring live eval sources",
    }
}

fn source_health_recommended_action(primary_blocker: &str) -> &'static str {
    match primary_blocker {
        "missing_required_source" => {
            "restore the missing required eval artifact or repair its collector before release"
        }
        "failed_required_source" => {
            "route the failing required eval artifact into issue synthesis and block release promotion"
        }
        "failed_optional_source" => {
            "file a non-blocking degradation issue and keep sampling the optional source"
        }
        _ => "continue continuous sampling",
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        1.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn observe_source(source: &SampleSourcePolicy) -> SourceObservation {
    let value = fs::read_to_string(&source.path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
    let present = value.is_some();
    let ok = value
        .as_ref()
        .and_then(|payload| payload.get("ok"))
        .and_then(Value::as_bool);
    let summary = value
        .as_ref()
        .and_then(|payload| payload.get("summary"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    SourceObservation {
        id: source.id.clone(),
        path: source.path.clone(),
        present,
        required: source.required,
        ok,
        summary,
    }
}

fn derive_metrics(sources: &[SourceObservation]) -> BTreeMap<String, u64> {
    let mut metrics = BTreeMap::new();
    metrics.insert(
        "missing_required_sources".to_string(),
        sources
            .iter()
            .filter(|source| source.required && !source.present)
            .count() as u64,
    );
    metrics.insert(
        "failed_required_sources".to_string(),
        sources
            .iter()
            .filter(|source| source.required && source.ok == Some(false))
            .count() as u64,
    );
    metrics.insert(
        "required_sources_present".to_string(),
        sources
            .iter()
            .filter(|source| source.required && source.present)
            .count() as u64,
    );
    metrics.insert(
        "required_sources_total".to_string(),
        sources.iter().filter(|source| source.required).count() as u64,
    );
    metrics.insert(
        "optional_sources_present".to_string(),
        sources
            .iter()
            .filter(|source| !source.required && source.present)
            .count() as u64,
    );
    metrics.insert(
        "optional_sources_failed".to_string(),
        sources
            .iter()
            .filter(|source| !source.required && source.ok == Some(false))
            .count() as u64,
    );

    for source in sources {
        if source.id == "eval_feedback_router" {
            copy_summary_count(source, &mut metrics, "kernel_block_routes", "kernel_block");
            copy_summary_count(
                source,
                &mut metrics,
                "gateway_quarantine_routes",
                "gateway_quarantine",
            );
            copy_summary_count(
                source,
                &mut metrics,
                "control_plane_retry_routes",
                "control_plane_retry",
            );
        }
    }

    for key in [
        "kernel_block_routes",
        "gateway_quarantine_routes",
        "control_plane_retry_routes",
    ] {
        metrics.entry(key.to_string()).or_insert(0);
    }
    metrics
}

fn copy_summary_count(
    source: &SourceObservation,
    metrics: &mut BTreeMap<String, u64>,
    metric_key: &str,
    summary_key: &str,
) {
    let count = source
        .summary
        .get("destinations")
        .and_then(|destinations| destinations.get(summary_key))
        .and_then(Value::as_u64)
        .or_else(|| source.summary.get(summary_key).and_then(Value::as_u64))
        .unwrap_or(0);
    metrics.insert(metric_key.to_string(), count);
}

fn drift_findings(policy: &LiveEvalPolicy, metrics: &BTreeMap<String, u64>) -> Vec<DriftFinding> {
    let thresholds = &policy.drift_thresholds;
    let checks = [
        (
            "live_eval_kernel_block_drift",
            "critical",
            "kernel_block_routes",
            thresholds.kernel_block_routes_max,
            "block_release_and_open_kernel_truth_issue",
        ),
        (
            "live_eval_gateway_quarantine_drift",
            "high",
            "gateway_quarantine_routes",
            thresholds.gateway_quarantine_routes_max,
            "quarantine_gateway_and_route_around",
        ),
        (
            "live_eval_control_plane_retry_drift",
            "medium",
            "control_plane_retry_routes",
            thresholds.control_plane_retry_routes_max,
            "force_control_plane_retry_review",
        ),
        (
            "live_eval_failed_required_source_drift",
            "critical",
            "failed_required_sources",
            thresholds.failed_required_sources_max.unwrap_or(0),
            "block_release_and_repair_required_eval_source",
        ),
        (
            "live_eval_missing_required_source_drift",
            "high",
            "missing_required_sources",
            thresholds.missing_required_sources_max,
            "raise_eval_observability_gap_alert",
        ),
    ];

    checks
        .iter()
        .filter_map(|(id, severity, metric_key, threshold, action)| {
            let observed = metrics.get(*metric_key).copied().unwrap_or(0);
            (observed > *threshold).then(|| DriftFinding {
                id: (*id).to_string(),
                severity: (*severity).to_string(),
                threshold: *threshold,
                observed,
                mitigation_action: (*action).to_string(),
            })
        })
        .collect()
}

fn write_json(path: &str, value: &LiveEvalReport) -> bool {
    if let Some(parent) = Path::new(path).parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    serde_json::to_string_pretty(value)
        .ok()
        .and_then(|raw| fs::write(path, format!("{raw}\n")).ok())
        .is_some()
}

fn append_jsonl(path: &str, value: &LiveEvalReport) -> bool {
    if let Some(parent) = Path::new(path).parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    let raw = match serde_json::to_string(value) {
        Ok(raw) => raw,
        Err(_) => return false,
    };
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| writeln!(file, "{raw}"))
        .is_ok()
}

fn write_markdown(path: &str, report: &LiveEvalReport) -> bool {
    if let Some(parent) = Path::new(path).parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    let mut body = String::new();
    body.push_str("# Live Eval Current\n\n");
    body.push_str(&format!("- ok: {}\n", report.ok));
    body.push_str(&format!("- stream: `{}`\n", report.stream_path));
    body.push_str(&format!("- source_health: `{}`\n", report.source_health));
    body.push_str("- metrics:\n");
    for (key, value) in &report.metrics {
        body.push_str(&format!("  - `{key}`: {value}\n"));
    }
    body.push_str("- drift findings:\n");
    if report.drift_findings.is_empty() {
        body.push_str("  - none\n");
    } else {
        for finding in &report.drift_findings {
            body.push_str(&format!(
                "  - `{}`: observed {} > threshold {} => `{}`\n",
                finding.id, finding.observed, finding.threshold, finding.mitigation_action
            ));
        }
    }
    fs::write(path, body).is_ok()
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter().find_map(|arg| {
        arg.strip_prefix(&format!("{flag}="))
            .map(|value| value.to_string())
    })
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> LiveEvalPolicy {
        LiveEvalPolicy {
            schema_version: 1,
            sample_sources: vec![SampleSourcePolicy {
                id: "eval_feedback_router".to_string(),
                path: "missing.json".to_string(),
                required: true,
            }],
            drift_thresholds: DriftThresholds {
                kernel_block_routes_max: 0,
                gateway_quarantine_routes_max: 0,
                control_plane_retry_routes_max: 2,
                missing_required_sources_max: 0,
                failed_required_sources_max: Some(0),
            },
            mitigations: vec![MitigationPolicy {
                id: "missing_source_alert".to_string(),
                trigger: "live_eval_missing_required_source_drift".to_string(),
                action: "raise_eval_observability_gap_alert".to_string(),
            }],
        }
    }

    #[test]
    fn missing_required_source_triggers_mitigation() {
        let report = build_live_eval_report(&policy(), DEFAULT_STREAM_PATH);
        assert!(!report.ok);
        assert_eq!(report.metrics.get("missing_required_sources"), Some(&1));
        assert!(report
            .drift_findings
            .iter()
            .any(|finding| finding.id == "live_eval_missing_required_source_drift"));
        assert_eq!(report.mitigations.len(), 1);
    }

    #[test]
    fn feedback_router_summary_counts_drive_drift() {
        let source = SourceObservation {
            id: "eval_feedback_router".to_string(),
            path: "router.json".to_string(),
            present: true,
            required: true,
            ok: Some(false),
            summary: json!({
                "destinations": {
                    "kernel_block": 1,
                    "gateway_quarantine": 2,
                    "control_plane_retry": 3
                }
            }),
        };
        let metrics = derive_metrics(&[source]);
        assert_eq!(metrics.get("kernel_block_routes"), Some(&1));
        assert_eq!(metrics.get("gateway_quarantine_routes"), Some(&2));
        assert_eq!(metrics.get("control_plane_retry_routes"), Some(&3));
    }

    #[test]
    fn critical_source_health_waits_for_repeated_stable_signature() {
        let source = SourceObservation {
            id: "eval_feedback_router".to_string(),
            path: "missing.json".to_string(),
            present: false,
            required: true,
            ok: None,
            summary: json!({}),
        };
        let health = source_health(&[source], &[]);

        assert_eq!(health.get("state").and_then(Value::as_str), Some("critical"));
        assert_eq!(
            health.get("issue_candidate_ready").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            health
                .pointer("/issue_candidate/issue_candidate_reason")
                .and_then(Value::as_str),
            Some("awaiting_repeated_stable_signature")
        );
    }

    #[test]
    fn repeated_critical_source_health_becomes_issue_ready() {
        let source = SourceObservation {
            id: "eval_feedback_router".to_string(),
            path: "missing.json".to_string(),
            present: false,
            required: true,
            ok: None,
            summary: json!({}),
        };
        let previous = json!({
            "source_health": {
                "dedupe_key": "live_eval_source_health:missing_required_source:1"
            }
        });
        let health = source_health(&[source], &[previous]);

        assert_eq!(
            health.get("issue_candidate_ready").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            health
                .pointer("/issue_candidate/stable_signature_occurrence_count")
                .and_then(Value::as_u64),
            Some(2)
        );
    }
}
