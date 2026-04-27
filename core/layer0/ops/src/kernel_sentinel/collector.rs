// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::cli_args::{option_path, option_usize, state_dir_from_args};
use super::write_json;

const DEFAULT_COLLECTOR_ARTIFACT: &str = "core/local/artifacts/kernel_sentinel_collector_current.json";
const DEFAULT_MAX_FILES_PER_PRODUCER: usize = 200;

struct ProducerSpec {
    id: &'static str,
    path: &'static str,
    target_stream: &'static str,
    authority_class: &'static str,
    kind: &'static str,
}

fn producer_specs() -> Vec<ProducerSpec> {
    vec![
        ProducerSpec {
            id: "verity_receipts",
            path: "local/state/ops/verity",
            target_stream: "kernel_receipts.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "kernel_receipt_bridge",
        },
        ProducerSpec {
            id: "system_health_audit",
            path: "local/state/ops/system_health_audit",
            target_stream: "runtime_observations.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "runtime_observation_bridge",
        },
        ProducerSpec {
            id: "eval_agent_feedback",
            path: "local/state/ops/eval_agent_feedback",
            target_stream: "control_plane_eval.jsonl",
            authority_class: "advisory_workflow_quality",
            kind: "control_plane_eval_bridge",
        },
        ProducerSpec {
            id: "eval_learning_loop",
            path: "local/state/ops/eval_learning_loop",
            target_stream: "control_plane_eval.jsonl",
            authority_class: "advisory_workflow_quality",
            kind: "control_plane_eval_bridge",
        },
        ProducerSpec {
            id: "synthetic_user_chat_harness",
            path: "local/state/ops/synthetic_user_chat_harness",
            target_stream: "runtime_observations.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "runtime_observation_bridge",
        },
        ProducerSpec {
            id: "shell_telemetry",
            path: "local/state/ops/shell_telemetry",
            target_stream: "shell_telemetry.jsonl",
            authority_class: "presentation_telemetry_only",
            kind: "shell_telemetry_bridge",
        },
        ProducerSpec {
            id: "runtime_telemetry_shell",
            path: "local/state/ops/runtime_telemetry",
            target_stream: "shell_telemetry.jsonl",
            authority_class: "presentation_telemetry_only",
            kind: "shell_telemetry_bridge",
        },
    ]
}

fn artifact_specs(root: &Path) -> Vec<ProducerSpec> {
    let artifact_root = root.join("core/local/artifacts");
    if !artifact_root.exists() {
        return Vec::new();
    }
    vec![
        ProducerSpec {
            id: "release_proof_pack_artifacts",
            path: "core/local/artifacts",
            target_stream: "release_proof_packs.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "release_proof_pack_bridge",
        },
        ProducerSpec {
            id: "release_repair_artifacts",
            path: "core/local/artifacts",
            target_stream: "release_repairs.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "release_repair_bridge",
        },
        ProducerSpec {
            id: "state_mutation_artifacts",
            path: "core/local/artifacts",
            target_stream: "state_mutations.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "state_mutation_bridge",
        },
        ProducerSpec {
            id: "scheduler_admission_artifacts",
            path: "core/local/artifacts",
            target_stream: "scheduler_admission.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "scheduler_admission_bridge",
        },
        ProducerSpec {
            id: "live_recovery_artifacts",
            path: "core/local/artifacts",
            target_stream: "live_recovery.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "live_recovery_bridge",
        },
        ProducerSpec {
            id: "gateway_artifacts",
            path: "core/local/artifacts",
            target_stream: "gateway_health.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "gateway_health_bridge",
        },
        ProducerSpec {
            id: "gateway_quarantine_artifacts",
            path: "core/local/artifacts",
            target_stream: "gateway_quarantine.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "gateway_quarantine_bridge",
        },
        ProducerSpec {
            id: "gateway_recovery_artifacts",
            path: "core/local/artifacts",
            target_stream: "gateway_recovery.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "gateway_recovery_bridge",
        },
        ProducerSpec {
            id: "gateway_isolation_artifacts",
            path: "core/local/artifacts",
            target_stream: "gateway_isolation.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "gateway_isolation_bridge",
        },
        ProducerSpec {
            id: "queue_backpressure_artifacts",
            path: "core/local/artifacts",
            target_stream: "queue_backpressure.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "queue_backpressure_bridge",
        },
        ProducerSpec {
            id: "boundedness_artifacts",
            path: "core/local/artifacts",
            target_stream: "boundedness_observations.jsonl",
            authority_class: "deterministic_kernel_authority",
            kind: "boundedness_bridge",
        },
        ProducerSpec {
            id: "shell_telemetry_artifacts",
            path: "core/local/artifacts",
            target_stream: "shell_telemetry.jsonl",
            authority_class: "presentation_telemetry_only",
            kind: "shell_telemetry_bridge",
        },
    ]
}

fn artifact_matches(spec_id: &str, path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    match spec_id {
        "release_proof_pack_artifacts" => name.contains("proof") || name.contains("release_verdict"),
        "release_repair_artifacts" => name.contains("repair") || name.contains("fallback"),
        "gateway_artifacts" => {
            (name.contains("gateway") || name.contains("adapter"))
                && !name.contains("quarantine")
                && !name.contains("recovery")
                && !name.contains("isolation")
                && !name.contains("sandbox")
                && !name.contains("chaos")
                && !name.contains("boundary")
                && !name.contains("flapping")
                && !name.contains("breaker")
        }
        "state_mutation_artifacts" => {
            name.contains("state_mutation")
                || name.contains("stateful")
                || name.contains("rollback")
                || name.contains("upgrade")
        }
        "scheduler_admission_artifacts" => {
            name.contains("scheduler")
                || name.contains("schedule")
                || name.contains("admission")
                || name.contains("agent_surface_status_guard")
                || name.contains("layer3_contract_guard")
        }
        "live_recovery_artifacts" => {
            name.contains("recovery")
                || name.contains("auto_heal")
                || name.contains("rollback")
                || name.contains("retry")
        }
        "gateway_quarantine_artifacts" => {
            name.contains("gateway")
                && (name.contains("quarantine")
                    || name.contains("flapping")
                    || name.contains("breaker"))
        }
        "gateway_recovery_artifacts" => {
            name.contains("gateway") && (name.contains("recovery") || name.contains("auto_heal"))
        }
        "gateway_isolation_artifacts" => {
            (name.contains("gateway") || name.contains("adapter"))
                && (name.contains("isolation")
                    || name.contains("sandbox")
                    || name.contains("chaos")
                    || name.contains("boundary"))
        }
        "queue_backpressure_artifacts" => name.contains("queue") || name.contains("backpressure"),
        "boundedness_artifacts" => name.contains("boundedness") || name.contains("soak"),
        "shell_telemetry_artifacts" => {
            name.contains("shell")
                || name.contains("dashboard")
                || name.contains("chat_phase")
                || name.contains("thinking")
        }
        _ => true,
    }
}

fn collect_files(root: &Path, spec: &ProducerSpec, max_files: usize) -> Vec<PathBuf> {
    let base = root.join(spec.path);
    let mut out = Vec::new();
    collect_files_inner(spec, &base, max_files, &mut out);
    out.sort();
    out
}

fn collect_files_inner(spec: &ProducerSpec, path: &Path, max_files: usize, out: &mut Vec<PathBuf>) {
    if out.len() >= max_files || !path.exists() {
        return;
    }
    if path.is_file() {
        let extension = path.extension().and_then(|value| value.to_str()).unwrap_or("");
        if matches!(extension, "json" | "jsonl") && artifact_matches(spec.id, path) {
            out.push(path.to_path_buf());
        }
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= max_files {
            break;
        }
        collect_files_inner(spec, &entry.path(), max_files, out);
    }
}

fn source_details(path: &Path, spec: &ProducerSpec, raw: &Value) -> Value {
    let mut details = match raw.as_object() {
        Some(object) => object.clone(),
        None => {
            let mut object = Map::new();
            object.insert("raw_value".to_string(), raw.clone());
            object
        }
    };
    details.insert(
        "source_artifact".to_string(),
        Value::from(path.display().to_string()),
    );
    details.insert("collector_id".to_string(), Value::from(spec.id));
    details.insert(
        "authority_class".to_string(),
        Value::from(spec.authority_class),
    );
    Value::Object(details)
}

fn row_subject(path: &Path, raw: &Value) -> String {
    raw.get("subject")
        .and_then(Value::as_str)
        .or_else(|| raw.get("id").and_then(Value::as_str))
        .map(str::to_string)
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("unknown_artifact")
                .to_string()
        })
}

fn raw_string(raw: &Value, key: &str) -> Option<String> {
    raw.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn stream_category(spec: &ProducerSpec, raw: &Value) -> String {
    raw_string(raw, "category").unwrap_or_else(|| match spec.target_stream {
        "kernel_receipts.jsonl" => "ReceiptIntegrity",
        "runtime_observations.jsonl" => "RuntimeCorrectness",
        "release_proof_packs.jsonl" | "release_repairs.jsonl" => "ReleaseEvidence",
        "gateway_health.jsonl" | "gateway_quarantine.jsonl" | "gateway_recovery.jsonl" | "gateway_isolation.jsonl" => {
            "GatewayIsolation"
        }
        "queue_backpressure.jsonl" => "QueueBackpressure",
        "boundedness_observations.jsonl" => "Boundedness",
        "control_plane_eval.jsonl" => "RuntimeCorrectness",
        _ => "RuntimeCorrectness",
    }
    .to_string())
}

fn stream_severity(spec: &ProducerSpec, raw: &Value) -> Option<String> {
    if let Some(severity) = raw_string(raw, "severity") {
        return Some(severity);
    }
    let failed = raw
        .get("ok")
        .and_then(Value::as_bool)
        .map(|ok| !ok)
        .or_else(|| raw.get("pass").and_then(Value::as_bool).map(|pass| !pass))
        .unwrap_or(false);
    if !failed {
        return None;
    }
    Some(match spec.target_stream {
        "kernel_receipts.jsonl" | "release_proof_packs.jsonl" => "Critical",
        "control_plane_eval.jsonl" => "Medium",
        _ => "High",
    }
    .to_string())
}

fn stream_status(raw: &Value) -> String {
    raw_string(raw, "status").unwrap_or_else(|| {
        let failed = raw
            .get("ok")
            .and_then(Value::as_bool)
            .map(|ok| !ok)
            .or_else(|| raw.get("pass").and_then(Value::as_bool).map(|pass| !pass))
            .unwrap_or(false);
        if failed {
            "failed"
        } else {
            "observed"
        }
        .to_string()
    })
}

fn bridge_row(path: &Path, spec: &ProducerSpec, raw: &Value, line: Option<usize>) -> Value {
    let subject = row_subject(path, raw);
    let line_suffix = line.map(|value| format!(":{value}")).unwrap_or_default();
    let source_artifact = path.display().to_string();
    let category = stream_category(spec, raw);
    let severity = stream_severity(spec, raw);
    let mut details = source_details(path, spec, raw);
    if let Some(object) = details.as_object_mut() {
        object
            .entry("source_reference".to_string())
            .or_insert_with(|| Value::from(source_artifact.clone()));
        if spec.target_stream == "control_plane_eval.jsonl" {
            object
                .entry("safe_to_auto_apply_patch".to_string())
                .or_insert(Value::Bool(false));
            object
                .entry("safe_to_auto_file_issue".to_string())
                .or_insert(Value::Bool(true));
            object
                .entry("may_block_release".to_string())
                .or_insert(Value::Bool(false));
            object
                .entry("may_write_verdict".to_string())
                .or_insert(Value::Bool(false));
            object
                .entry("may_waive_finding".to_string())
                .or_insert(Value::Bool(false));
        }
    }
    json!({
        "id": raw.get("id").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| format!("{}:{}{}", spec.id, subject, line_suffix)),
        "ok": raw.get("ok").and_then(Value::as_bool).or_else(|| raw.get("pass").and_then(Value::as_bool)),
        "status": stream_status(raw),
        "severity": severity,
        "category": category,
        "fingerprint": raw.get("fingerprint").and_then(Value::as_str).or_else(|| raw.get("dedupe_key").and_then(Value::as_str)).map(str::to_string).unwrap_or_else(|| format!("{}:{subject}", spec.id)),
        "subject": subject,
        "kind": raw.get("kind").and_then(Value::as_str).unwrap_or(spec.kind),
        "summary": raw.get("summary").and_then(Value::as_str),
        "recommended_action": raw.get("recommended_action").and_then(Value::as_str).or_else(|| raw.get("recommended_fix").and_then(Value::as_str)),
        "evidence": [source_artifact],
        "details": details
    })
}

fn rows_from_file(path: &Path, spec: &ProducerSpec) -> (Vec<Value>, Vec<Value>) {
    let Ok(body) = fs::read_to_string(path) else {
        return (
            Vec::new(),
            vec![json!({"path": path, "error": "read_failed"})],
        );
    };
    if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
        let mut rows = Vec::new();
        let mut malformed = Vec::new();
        for (idx, line) in body.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<Value>(trimmed) {
                Ok(raw) => rows.push(bridge_row(path, spec, &raw, Some(idx + 1))),
                Err(err) => malformed.push(json!({"path": path, "line": idx + 1, "error": err.to_string()})),
            }
        }
        return (rows, malformed);
    }
    match serde_json::from_str::<Value>(&body) {
        Ok(raw) => (vec![bridge_row(path, spec, &raw, None)], Vec::new()),
        Err(err) => (
            Vec::new(),
            vec![json!({"path": path, "error": err.to_string()})],
        ),
    }
}

fn write_jsonl(path: &Path, rows: &[Value]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut file = fs::File::create(path).map_err(|err| err.to_string())?;
    for row in rows {
        let body = serde_json::to_string(row).map_err(|err| err.to_string())?;
        writeln!(file, "{body}").map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn producer_required_for_observation(spec: &ProducerSpec) -> bool {
    spec.authority_class != "presentation_telemetry_only"
}

pub fn build_collector_report(root: &Path, args: &[String]) -> Result<Value, String> {
    let state_dir = state_dir_from_args(root, args);
    let evidence_dir = option_path(args, "--evidence-dir", state_dir.join("evidence"));
    let max_files = option_usize(args, "--max-source-files", DEFAULT_MAX_FILES_PER_PRODUCER);
    let mut by_stream: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut source_reports = Vec::new();
    let mut total_read = 0usize;
    let mut total_written = 0usize;
    let mut total_malformed = 0usize;
    let mut present_required_source_count = 0usize;
    let mut missing_required_source_count = 0usize;
    let mut present_optional_source_count = 0usize;
    let mut missing_optional_source_count = 0usize;
    let mut specs = producer_specs();
    specs.extend(artifact_specs(root));
    for spec in specs {
        let files = collect_files(root, &spec, max_files);
        let required_for_observation = producer_required_for_observation(&spec);
        if files.is_empty() {
            if required_for_observation {
                missing_required_source_count += 1;
            } else {
                missing_optional_source_count += 1;
            }
        } else if required_for_observation {
            present_required_source_count += 1;
        } else {
            present_optional_source_count += 1;
        }
        let mut records_read = 0usize;
        let mut malformed_records = Vec::new();
        for file in &files {
            let (rows, malformed) = rows_from_file(file, &spec);
            records_read += rows.len();
            malformed_records.extend(malformed);
            by_stream
                .entry(spec.target_stream.to_string())
                .or_default()
                .extend(rows);
        }
        total_read += records_read;
        total_malformed += malformed_records.len();
        source_reports.push(json!({
            "producer_id": spec.id,
            "producer_path": spec.path,
            "target_stream": spec.target_stream,
            "authority_class": spec.authority_class,
            "required_for_observation": required_for_observation,
            "files_seen": files.len(),
            "records_read": records_read,
            "records_written": records_read,
            "malformed_count": malformed_records.len(),
            "skipped": files.is_empty(),
            "malformed_records": malformed_records
        }));
    }
    let mut output_streams = Vec::new();
    for (stream, rows) in &by_stream {
        let path = evidence_dir.join(stream);
        write_jsonl(&path, rows)?;
        total_written += rows.len();
        output_streams.push(json!({
            "stream": stream,
            "path": path,
            "record_count": rows.len()
        }));
    }
    let mut report = json!({
        "type": "kernel_sentinel_collector_run",
        "generated_at": crate::now_iso(),
        "evidence_dir": evidence_dir,
        "max_files_per_producer": max_files,
        "records_read": total_read,
        "records_written": total_written,
        "malformed_record_count": total_malformed,
        "source_count": source_reports.len(),
        "coverage": {
            "present_required_source_count": present_required_source_count,
            "missing_required_source_count": missing_required_source_count,
            "present_optional_source_count": present_optional_source_count,
            "missing_optional_source_count": missing_optional_source_count,
            "expected_required_source_count": present_required_source_count + missing_required_source_count,
            "expected_optional_source_count": present_optional_source_count + missing_optional_source_count,
            "required_observation_ready": total_written > 0 && missing_required_source_count == 0 && total_malformed == 0
        },
        "sources": source_reports,
        "output_streams": output_streams
    });
    report["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&report));
    Ok(report)
}

pub fn run_collect(root: &Path, args: &[String]) -> i32 {
    let artifact_path = option_path(
        args,
        "--collector-artifact",
        root.join(DEFAULT_COLLECTOR_ARTIFACT),
    );
    let report = match build_collector_report(root, args) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("kernel_sentinel_collector_failed: {err}");
            return 1;
        }
    };
    if let Err(err) = write_json(&artifact_path, &report) {
        eprintln!("kernel_sentinel_collector_artifact_write_failed: {err}");
        return 1;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collector_writes_evidence_stream_from_existing_verity_data() {
        let root = std::env::temp_dir().join(format!(
            "kernel-sentinel-collector-{}",
            crate::deterministic_receipt_hash(&json!({"test": "collector"}))
        ));
        let verity = root.join("local/state/ops/verity");
        fs::create_dir_all(&verity).unwrap();
        fs::write(
            verity.join("receipt.jsonl"),
            r#"{"id":"r1","ok":false,"subject":"mutation-1","kind":"receipt_check","summary":"receipt missing","recommended_action":"restore receipt","evidence":["receipt://missing"]}"#,
        )
        .unwrap();
        let args = vec![];
        let report = build_collector_report(&root, &args).unwrap();
        assert_eq!(report["records_written"], Value::from(1));
        assert_eq!(report["coverage"]["present_required_source_count"], 1);
        assert_eq!(report["coverage"]["missing_optional_source_count"], 2);
        let evidence = root.join("local/state/kernel_sentinel/evidence/kernel_receipts.jsonl");
        assert!(evidence.exists());
        let body = fs::read_to_string(evidence).unwrap();
        assert!(body.contains("mutation-1"));
    }

    #[test]
    fn collector_marks_shell_telemetry_sources_optional() {
        let root = std::env::temp_dir().join(format!(
            "kernel-sentinel-collector-shell-{}",
            crate::deterministic_receipt_hash(&json!({"test": "collector-shell"}))
        ));
        let shell = root.join("local/state/ops/shell_telemetry");
        fs::create_dir_all(&shell).unwrap();
        fs::write(
            shell.join("phase.jsonl"),
            r#"{"id":"shell-1","ok":false,"subject":"thinking-bubble","kind":"presentation_status","summary":"stale thinking text","evidence":["shell://thinking"]}"#,
        )
        .unwrap();
        let report = build_collector_report(&root, &[]).unwrap();
        assert_eq!(report["coverage"]["present_optional_source_count"], 1);
        assert_eq!(report["coverage"]["present_required_source_count"], 0);
        assert_eq!(report["coverage"]["required_observation_ready"], false);
        let shell_source = report["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["producer_id"] == "shell_telemetry")
            .expect("shell source");
        assert_eq!(shell_source["required_for_observation"], false);
        assert_eq!(shell_source["target_stream"], "shell_telemetry.jsonl");
    }

    #[test]
    fn collector_maps_required_runtime_artifact_families() {
        let root = std::env::temp_dir().join(format!(
            "kernel-sentinel-collector-required-artifacts-{}",
            crate::deterministic_receipt_hash(&json!({"test": "collector-required-artifacts"}))
        ));
        let artifacts = root.join("core/local/artifacts");
        fs::create_dir_all(&artifacts).unwrap();
        for (file_name, subject) in [
            ("stateful_upgrade_rollback_gate_current.json", "stateful-upgrade"),
            ("agent_surface_status_guard_current.json", "scheduler-admission"),
            ("workflow_failure_recovery_current.json", "workflow-recovery"),
            ("gateway_boundary_guard_current.json", "gateway-boundary"),
        ] {
            fs::write(
                artifacts.join(file_name),
                format!(
                    "{{\"id\":\"{subject}\",\"ok\":true,\"subject\":\"{subject}\",\"summary\":\"{subject} observed\"}}"
                ),
            )
            .unwrap();
        }
        let report = build_collector_report(&root, &[]).unwrap();
        let streams: Vec<String> = report["output_streams"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|row| row["stream"].as_str().map(str::to_string))
            .collect();
        assert!(streams.iter().any(|stream| stream == "state_mutations.jsonl"));
        assert!(streams
            .iter()
            .any(|stream| stream == "scheduler_admission.jsonl"));
        assert!(streams.iter().any(|stream| stream == "live_recovery.jsonl"));
        assert!(streams.iter().any(|stream| stream == "gateway_isolation.jsonl"));
    }
}
