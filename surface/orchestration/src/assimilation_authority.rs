use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_OUT_PATH: &str = "core/local/artifacts/assimilation_authority_guard_current.json";
const DEFAULT_REPORT_PATH: &str =
    "local/workspace/reports/ASSIMILATION_AUTHORITY_GUARD_CURRENT.md";
const MANUAL_TEMPLATE_PATH: &str = "docs/workspace/ASSIMILATION_MANUAL_TEMPLATE.md";
const WAVE_RUNBOOK_PATH: &str = "docs/workspace/CODEX_ASSIMILATION_WAVE_RUNBOOK.md";
const WORKFLOW_POLICY_PATH: &str = "docs/workspace/workflow_json_format_policy.md";
const TEMPLATE_REGISTRY_PATH: &str = "surface/orchestration/src/control_plane/templates.rs";
const LIFECYCLE_PATH: &str = "surface/orchestration/src/control_plane/lifecycle.rs";

const WORKFLOW_PATHS: &[&str] = &[
    "surface/orchestration/src/control_plane/workflows/codex_tooling_synthesis.workflow.json",
    "surface/orchestration/src/control_plane/workflows/forgecode_agent_composition.workflow.json",
    "surface/orchestration/src/control_plane/workflows/forgecode_raw_capability_assimilation.workflow.json",
];

#[derive(Debug, Clone, Serialize)]
struct CheckRow {
    id: String,
    ok: bool,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct WorkflowObservation {
    path: String,
    parse_ok: bool,
    name: String,
    workflow_type: String,
    stage_count: usize,
    subtemplate_count: usize,
    required_signal_count: usize,
    source_ref_count: usize,
    placement: String,
}

#[derive(Debug, Clone, Serialize)]
struct AssimilationAuthorityReport {
    ok: bool,
    r#type: String,
    generated_unix_seconds: u64,
    owner: String,
    artifact_paths: Value,
    workflow_observations: Vec<WorkflowObservation>,
    checks: Vec<CheckRow>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct WorkflowSpec {
    #[serde(default)]
    name: String,
    #[serde(default)]
    workflow_type: String,
    #[serde(default)]
    stages: Vec<String>,
    #[serde(default)]
    subtemplates: Vec<WorkflowSubtemplateSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct WorkflowSubtemplateSpec {
    #[serde(default)]
    required_signals: Vec<String>,
    #[serde(default)]
    source_refs: Vec<String>,
}

pub fn run_assimilation_authority_guard(args: &[String]) -> i32 {
    let strict = flag_value(args, "--strict").unwrap_or_else(|| "0".to_string()) == "1";
    let out_path = flag_value(args, "--out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let report_path =
        flag_value(args, "--report").unwrap_or_else(|| DEFAULT_REPORT_PATH.to_string());

    let report = build_report();
    let wrote = write_json(&out_path, &report) && write_markdown(&report_path, &report);
    if !wrote {
        eprintln!("assimilation_authority_guard: failed to write outputs");
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

fn build_report() -> AssimilationAuthorityReport {
    let manual_template = read_text(MANUAL_TEMPLATE_PATH);
    let runbook = read_text(WAVE_RUNBOOK_PATH);
    let workflow_policy = read_text(WORKFLOW_POLICY_PATH);
    let template_registry = read_text(TEMPLATE_REGISTRY_PATH);
    let lifecycle = read_text(LIFECYCLE_PATH);
    let workflow_observations = observe_workflows();
    let checks = build_checks(
        &manual_template,
        &runbook,
        &workflow_policy,
        &template_registry,
        &lifecycle,
        &workflow_observations,
    );
    AssimilationAuthorityReport {
        ok: checks.iter().all(|row| row.ok),
        r#type: "assimilation_authority_guard".to_string(),
        generated_unix_seconds: now_unix_seconds(),
        owner: "surface_orchestration_assimilation_authority".to_string(),
        artifact_paths: json!({
            "manual_template": MANUAL_TEMPLATE_PATH,
            "wave_runbook": WAVE_RUNBOOK_PATH,
            "workflow_policy": WORKFLOW_POLICY_PATH,
            "template_registry": TEMPLATE_REGISTRY_PATH,
            "lifecycle_selector": LIFECYCLE_PATH,
            "workflow_specs": WORKFLOW_PATHS,
        }),
        workflow_observations,
        checks,
    }
}

fn build_checks(
    manual_template: &str,
    runbook: &str,
    workflow_policy: &str,
    template_registry: &str,
    lifecycle: &str,
    workflows: &[WorkflowObservation],
) -> Vec<CheckRow> {
    let mut checks = Vec::new();
    checks.push(check(
        "assimilation_three_layer_authority_contract",
        all_present(
            manual_template,
            &[
                "Assimilation Map",
                "Priority Ledger",
                "Active Queue",
                "source-burn-down.tsv",
                "decisions-log.md",
            ],
        ),
        "manual template must define map, ledger, queue, decisions, and source burn-down",
    ));
    checks.push(check(
        "assimilation_burn_down_runbook_contract",
        all_present(
            runbook,
            &["source-burn-down.tsv", "burned_down", "4-8", "unique"],
        ),
        "wave runbook must require source-file burn-down and disjoint wave rows",
    ));
    checks.push(check(
        "workflow_raw_capability_boundary_contract",
        all_present(
            workflow_policy,
            &[
                "Raw system capability/mechanics belong in Rust authority paths",
                "Workflow structure belongs in JSON workflow specs",
                "sequencing/flow shape only",
            ],
        ),
        "workflow policy must separate Rust raw capability from JSON workflow shape",
    ));
    checks.push(check(
        "workflow_specs_parse_and_stage_contract",
        !workflows.is_empty() && workflows.iter().all(|row| row.parse_ok && row.stage_count > 0),
        "every assimilation workflow JSON spec must parse and declare stages",
    ));
    checks.push(check(
        "workflow_specs_have_subtemplate_evidence_contract",
        workflows
            .iter()
            .all(|row| row.subtemplate_count > 0 && row.source_ref_count > 0),
        "assimilation workflows must carry source-ref-backed subtemplates",
    ));
    checks.push(check(
        "workflow_template_registry_wires_json_contract",
        WORKFLOW_PATHS.iter().all(|path| {
            let needle = path
                .strip_prefix("surface/orchestration/src/control_plane/")
                .unwrap_or(path);
            template_registry.contains(needle)
        }),
        "templates.rs must include and register all assimilation workflow JSON specs",
    ));
    checks.push(check(
        "assimilation_selector_raw_vs_structured_contract",
        all_present(
            lifecycle,
            &[
                "ForgeCodeRawCapabilityAssimilation",
                "ForgeCodeAgentComposition",
                "raw capability",
                "no workflow wrapper",
            ],
        ),
        "lifecycle selector must route raw-capability assimilation separately from structured workflow composition",
    ));
    checks.push(check(
        "raw_capability_template_declares_no_wrapper_contract",
        workflows.iter().any(|row| {
            row.name == "forgecode_raw_capability_assimilation"
                && row.placement == "rust_raw_capability"
                && row.required_signal_count >= 2
        }),
        "raw capability workflow must be declared as Rust/runtime placement, not a composed wrapper",
    ));
    checks.push(check(
        "structured_workflow_template_declares_json_composition_contract",
        workflows.iter().any(|row| {
            row.name == "forgecode_agent_composition"
                && row.placement == "json_workflow_composition"
                && row.subtemplate_count >= 3
        }),
        "structured ForgeCode workflow must remain JSON composition with multiple subtemplates",
    ));
    checks
}

fn observe_workflows() -> Vec<WorkflowObservation> {
    WORKFLOW_PATHS
        .iter()
        .map(|path| observe_workflow(path))
        .collect()
}

fn observe_workflow(path: &str) -> WorkflowObservation {
    let raw = read_text(path);
    let parsed = serde_json::from_str::<WorkflowSpec>(&raw);
    match parsed {
        Ok(spec) => {
            let required_signal_count = spec
                .subtemplates
                .iter()
                .map(|row| row.required_signals.len())
                .sum();
            let source_ref_count = spec
                .subtemplates
                .iter()
                .map(|row| row.source_refs.len())
                .sum();
            let placement = placement_for(&spec);
            WorkflowObservation {
                path: path.to_string(),
                parse_ok: true,
                name: spec.name,
                workflow_type: spec.workflow_type,
                stage_count: spec.stages.iter().filter(|row| !row.trim().is_empty()).count(),
                subtemplate_count: spec.subtemplates.len(),
                required_signal_count,
                source_ref_count,
                placement,
            }
        }
        Err(err) => WorkflowObservation {
            path: path.to_string(),
            parse_ok: false,
            name: String::new(),
            workflow_type: String::new(),
            stage_count: 0,
            subtemplate_count: 0,
            required_signal_count: 0,
            source_ref_count: 0,
            placement: format!("parse_error:{err}"),
        },
    }
}

fn placement_for(spec: &WorkflowSpec) -> String {
    let name = spec.name.to_ascii_lowercase();
    let description = serde_json::to_string(spec).unwrap_or_default().to_ascii_lowercase();
    if name.contains("raw_capability") || description.contains("raw capability") {
        return "rust_raw_capability".to_string();
    }
    "json_workflow_composition".to_string()
}

fn check(id: &str, ok: bool, detail: &str) -> CheckRow {
    CheckRow {
        id: id.to_string(),
        ok,
        detail: detail.to_string(),
    }
}

fn all_present(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().all(|needle| haystack.contains(needle))
}

fn read_text(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn write_json(path: &str, report: &AssimilationAuthorityReport) -> bool {
    if let Some(parent) = Path::new(path).parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    let Ok(raw) = serde_json::to_string_pretty(report) else {
        return false;
    };
    fs::write(path, format!("{raw}\n")).is_ok()
}

fn write_markdown(path: &str, report: &AssimilationAuthorityReport) -> bool {
    if let Some(parent) = Path::new(path).parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    let mut lines = Vec::new();
    lines.push("# Assimilation Authority Guard".to_string());
    lines.push(String::new());
    lines.push(format!("Pass: {}", report.ok));
    lines.push(format!("Owner: {}", report.owner));
    lines.push(String::new());
    lines.push("## Checks".to_string());
    for row in &report.checks {
        lines.push(format!(
            "- [{}] {} :: {}",
            if row.ok { "x" } else { " " },
            row.id,
            row.detail
        ));
    }
    lines.push(String::new());
    lines.push("## Workflow Observations".to_string());
    for row in &report.workflow_observations {
        lines.push(format!(
            "- {} :: parse_ok={} :: placement={} :: stages={} :: subtemplates={} :: source_refs={}",
            row.name,
            row.parse_ok,
            row.placement,
            row.stage_count,
            row.subtemplate_count,
            row.source_ref_count
        ));
    }
    fs::write(path, format!("{}\n", lines.join("\n"))).is_ok()
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    for idx in 0..args.len() {
        let value = &args[idx];
        if value == name {
            return args.get(idx + 1).cloned();
        }
        if let Some(rest) = value.strip_prefix(&prefix) {
            return Some(rest.to_string());
        }
    }
    None
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|row| row.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_placement_keeps_raw_capability_out_of_composed_wrapper() {
        let raw = WorkflowSpec {
            name: "forgecode_raw_capability_assimilation".to_string(),
            workflow_type: "control_plane_orchestration_workflow".to_string(),
            stages: vec!["intake_normalization".to_string()],
            subtemplates: Vec::new(),
        };
        let structured = WorkflowSpec {
            name: "forgecode_agent_composition".to_string(),
            workflow_type: "control_plane_orchestration_workflow".to_string(),
            stages: vec!["intake_normalization".to_string()],
            subtemplates: Vec::new(),
        };
        assert_eq!(placement_for(&raw), "rust_raw_capability");
        assert_eq!(placement_for(&structured), "json_workflow_composition");
    }

    #[test]
    fn authority_report_requires_burn_down_and_json_workflow_contracts() {
        let workflows = vec![
            WorkflowObservation {
                path: "a.workflow.json".to_string(),
                parse_ok: true,
                name: "forgecode_agent_composition".to_string(),
                workflow_type: "control_plane_orchestration_workflow".to_string(),
                stage_count: 6,
                subtemplate_count: 3,
                required_signal_count: 9,
                source_ref_count: 9,
                placement: "json_workflow_composition".to_string(),
            },
            WorkflowObservation {
                path: "b.workflow.json".to_string(),
                parse_ok: true,
                name: "forgecode_raw_capability_assimilation".to_string(),
                workflow_type: "control_plane_orchestration_workflow".to_string(),
                stage_count: 6,
                subtemplate_count: 1,
                required_signal_count: 2,
                source_ref_count: 2,
                placement: "rust_raw_capability".to_string(),
            },
        ];
        let checks = build_checks(
            "Assimilation Map Priority Ledger Active Queue source-burn-down.tsv decisions-log.md",
            "source-burn-down.tsv burned_down 4-8 unique",
            "Raw system capability/mechanics belong in Rust authority paths Workflow structure belongs in JSON workflow specs sequencing/flow shape only",
            "workflows/codex_tooling_synthesis.workflow.json workflows/forgecode_agent_composition.workflow.json workflows/forgecode_raw_capability_assimilation.workflow.json",
            "ForgeCodeRawCapabilityAssimilation ForgeCodeAgentComposition raw capability no workflow wrapper",
            &workflows,
        );
        assert!(checks.iter().all(|row| row.ok));
    }
}
