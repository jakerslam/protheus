// Layer ownership: surface/orchestration (remediation-scope stability evidence only).
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const ORCHESTRATION_STABILITY_ARTIFACT_JSON: &str =
    "core/local/artifacts/orchestration_stability_guard_current.json";
pub const ORCHESTRATION_STABILITY_ARTIFACT_MARKDOWN: &str =
    "local/workspace/reports/ORCHESTRATION_STABILITY_GUARD_CURRENT.md";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestrationStabilityCheck {
    pub id: String,
    pub ok: bool,
    pub evidence: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationStabilityReport {
    #[serde(rename = "type")]
    pub report_type: String,
    pub schema_version: u64,
    pub generated_at_unix_seconds: u64,
    pub ok: bool,
    pub remediation_scope: String,
    pub architecture_rewrite_allowed: bool,
    pub checks: Vec<OrchestrationStabilityCheck>,
    pub summary: BTreeMap<String, u64>,
}

pub fn build_orchestration_stability_report(
    root: impl AsRef<Path>,
) -> OrchestrationStabilityReport {
    let root = root.as_ref();
    let lib_rs = read_text(root, "surface/orchestration/src/lib.rs");
    let readme = read_text(root, "README.md");
    let backend_registry = read_text(root, "core/layer2/tooling/src/backend_registry.rs");
    let status_machine = read_text(root, "core/layer2/tools/task_fabric/src/status_machine.rs");
    let transient_context = read_text(root, "surface/orchestration/src/transient_context.rs");
    let hidden_state_guard = read_text(
        root,
        "tests/tooling/scripts/ci/orchestration_hidden_state_guard.ts",
    );
    let gate_registry = read_text(root, "tests/tooling/config/tooling_gate_registry.json");
    let srs = read_text(root, "docs/workspace/SRS.md");

    let checks = vec![
        architecture_split_preserved(root, &lib_rs),
        resident_ipc_topology_preserved(&readme, &backend_registry),
        terminal_task_semantics_preserved(&status_machine),
        hidden_state_guardrails_preserved(&transient_context, &hidden_state_guard, &gate_registry),
        remediation_scope_preserved(&srs),
    ];
    let failing = checks.iter().filter(|row| !row.ok).count() as u64;
    let mut summary = BTreeMap::new();
    summary.insert("total_checks".to_string(), checks.len() as u64);
    summary.insert("passing_checks".to_string(), checks.len() as u64 - failing);
    summary.insert("failing_checks".to_string(), failing);

    OrchestrationStabilityReport {
        report_type: "orchestration_stability_guard".to_string(),
        schema_version: 1,
        generated_at_unix_seconds: generated_at_unix_seconds(),
        ok: failing == 0,
        remediation_scope: "planner_truth_schema_hygiene_feedback_loop_quality".to_string(),
        architecture_rewrite_allowed: false,
        checks,
        summary,
    }
}

pub fn write_orchestration_stability_artifacts(
    root: impl AsRef<Path>,
    json_path: &str,
    markdown_path: &str,
) -> Result<OrchestrationStabilityReport, String> {
    let root = root.as_ref();
    let report = build_orchestration_stability_report(root);
    write_text(
        root,
        json_path,
        &serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?,
    )?;
    write_text(root, markdown_path, &render_markdown(&report))?;
    Ok(report)
}

fn architecture_split_preserved(root: &Path, lib_rs: &str) -> OrchestrationStabilityCheck {
    let required_paths = [
        "surface/orchestration/src/ingress",
        "surface/orchestration/src/planner",
        "surface/orchestration/src/control_plane",
        "surface/orchestration/src/recovery.rs",
        "surface/orchestration/src/sequencing.rs",
        "surface/orchestration/src/result_packaging.rs",
        "surface/orchestration/src/telemetry.rs",
    ];
    let required_modules = [
        "pub mod ingress;",
        "pub mod planner;",
        "pub mod control_plane;",
        "pub mod recovery;",
        "pub mod sequencing;",
        "pub mod result_packaging;",
        "pub mod telemetry;",
    ];
    let mut missing = Vec::new();
    for path in required_paths {
        if !root.join(path).exists() {
            missing.push(format!("missing_path:{path}"));
        }
    }
    for module in required_modules {
        if !lib_rs.contains(module) {
            missing.push(format!("missing_module:{module}"));
        }
    }
    check(
        "architecture_split_preserved",
        vec![
            "ingress/planner/control-plane/recovery/sequencing/result-packaging/telemetry remain separate paths".to_string(),
            "orchestration lib exports stable subdomains rather than a top-level rewrite module".to_string(),
        ],
        missing,
    )
}

fn resident_ipc_topology_preserved(
    readme: &str,
    backend_registry: &str,
) -> OrchestrationStabilityCheck {
    token_check(
        "resident_ipc_topology_preserved",
        &format!("{readme}\n{backend_registry}"),
        &[
            "resident-IPC authoritative",
            "process_transport_forbidden_in_production",
            "process_fallback_forbidden_in_production",
            "resident_ipc_authoritative",
        ],
        vec![
            "README states production release channels are resident-IPC authoritative".to_string(),
            "tooling backend registry still carries resident_ipc_authoritative health".to_string(),
        ],
    )
}

fn terminal_task_semantics_preserved(status_machine: &str) -> OrchestrationStabilityCheck {
    token_check(
        "terminal_task_semantics_preserved",
        status_machine,
        &[
            "failed_and_cancelled_states_do_not_reopen",
            "LifecycleStatus::Failed",
            "LifecycleStatus::Cancelled",
            "_ => false",
        ],
        vec!["Task Fabric status machine keeps Failed/Cancelled closed to reopening".to_string()],
    )
}

fn hidden_state_guardrails_preserved(
    transient_context: &str,
    hidden_state_guard: &str,
    gate_registry: &str,
) -> OrchestrationStabilityCheck {
    token_check(
        "hidden_state_guardrails_preserved",
        &format!("{transient_context}\n{hidden_state_guard}\n{gate_registry}"),
        &[
            "TransientContextStore",
            "EphemeralMemoryHeap",
            "upsert_execution_observation",
            "cleanup_with_cas",
            "ops:orchestration:hidden-state:guard",
            "orchestration_hidden_state_guard_current.json",
        ],
        vec![
            "execution observations remain in transient context plus ephemeral heap".to_string(),
            "hidden-state guard remains registered as release/tooling evidence".to_string(),
        ],
    )
}

fn remediation_scope_preserved(srs: &str) -> OrchestrationStabilityCheck {
    token_check(
        "remediation_scope_preserved",
        srs,
        &[
            "Orchestration Planner Truth and Feedback-Loop Quality Intake",
            "Avoid top-level architecture rewrites during this remediation wave",
            "planner truth, schema hygiene, and feedback-loop quality",
        ],
        vec![
            "SRS keeps this wave scoped to runtime discipline instead of top-level redesign"
                .to_string(),
        ],
    )
}

fn token_check(
    id: &str,
    source: &str,
    tokens: &[&str],
    evidence: Vec<String>,
) -> OrchestrationStabilityCheck {
    let missing = tokens
        .iter()
        .filter(|token| !source.contains(**token))
        .map(|token| format!("missing_token:{token}"))
        .collect::<Vec<_>>();
    check(id, evidence, missing)
}

fn check(id: &str, evidence: Vec<String>, missing: Vec<String>) -> OrchestrationStabilityCheck {
    OrchestrationStabilityCheck {
        id: id.to_string(),
        ok: missing.is_empty(),
        evidence,
        missing,
    }
}

fn read_text(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap_or_default()
}

fn write_text(root: &Path, path: &str, content: &str) -> Result<(), String> {
    let out = root.join(path);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create parent failed:{err}"))?;
    }
    fs::write(out, content).map_err(|err| format!("write artifact failed:{err}"))
}

fn render_markdown(report: &OrchestrationStabilityReport) -> String {
    let mut out = String::new();
    out.push_str("# Orchestration Stability Guard\n\n");
    out.push_str(&format!("- ok: `{}`\n", report.ok));
    out.push_str(&format!(
        "- remediation_scope: `{}`\n",
        report.remediation_scope
    ));
    out.push_str(&format!(
        "- architecture_rewrite_allowed: `{}`\n\n",
        report.architecture_rewrite_allowed
    ));
    out.push_str("| Check | OK | Missing |\n");
    out.push_str("| --- | --- | --- |\n");
    for check in &report.checks {
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` |\n",
            check.id,
            check.ok,
            check.missing.join(", ")
        ));
    }
    out
}

fn generated_at_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root")
    }

    #[test]
    fn current_repo_preserves_orchestration_stability_contract() {
        let report = build_orchestration_stability_report(repo_root());
        assert!(report.ok, "stability failures: {:?}", report.checks);
        assert!(!report.architecture_rewrite_allowed);
        assert_eq!(
            report.remediation_scope,
            "planner_truth_schema_hygiene_feedback_loop_quality"
        );
    }

    #[test]
    fn missing_resident_ipc_policy_fails_closed() {
        let check = resident_ipc_topology_preserved("", "");
        assert!(!check.ok);
        assert!(check
            .missing
            .iter()
            .any(|row| row.contains("resident-IPC authoritative")));
    }

    #[test]
    fn terminal_semantics_guard_fails_when_reopen_test_is_missing() {
        let check = terminal_task_semantics_preserved("LifecycleStatus::Failed");
        assert!(!check.ok);
        assert!(check
            .missing
            .iter()
            .any(|row| row.contains("failed_and_cancelled_states_do_not_reopen")));
    }
}
