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

fn build_live_eval_report(policy: &LiveEvalPolicy, stream_path: &str) -> LiveEvalReport {
    let generated_unix_seconds = now_unix_seconds();
    let sources: Vec<SourceObservation> =
        policy.sample_sources.iter().map(observe_source).collect();
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
        sources,
        metrics,
        drift_findings,
        mitigations,
        checks,
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
}
