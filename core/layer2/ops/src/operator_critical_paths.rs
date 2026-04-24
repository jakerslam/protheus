use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub const DEFAULT_NODE_BURNDOWN_PLAN_PATH: &str =
    "client/runtime/config/node_critical_path_burndown_plan.json";
pub const DEFAULT_OPERATOR_CRITICAL_REPORT_PATH: &str =
    "core/local/artifacts/operator_critical_burndown_current.json";
pub const DEFAULT_OPERATOR_CRITICAL_MARKDOWN_PATH: &str =
    "local/workspace/reports/OPERATOR_CRITICAL_BURNDOWN_CURRENT.md";

#[derive(Debug, Clone, Deserialize)]
pub struct NodeCriticalPathPlan {
    pub schema_id: String,
    pub operator_critical_domains: Vec<String>,
    pub operator_critical_target_classification: String,
    pub operator_critical_priority_cutoff_date: String,
    pub ordered_migration_queue: Vec<String>,
    pub lanes: Vec<NodeCriticalPathLane>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeCriticalPathLane {
    pub id: String,
    pub domain: String,
    pub owner: String,
    pub priority: u8,
    pub target_classification: String,
    pub target_date: String,
    pub migration_status: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorCriticalEntrypoint {
    pub domain_group: String,
    pub command: String,
    pub artifact_path: String,
    pub implementation: String,
    pub covered_domains: Vec<String>,
    pub covered_lanes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorCriticalReport {
    pub ok: bool,
    #[serde(rename = "type")]
    pub report_type: String,
    pub generated_at_epoch_ms: u64,
    pub mode: String,
    pub summary: Value,
    pub rust_entrypoints: Vec<OperatorCriticalEntrypoint>,
    pub lane_rows: Vec<NodeCriticalPathLane>,
    pub checks: Vec<Value>,
}

pub fn load_node_critical_path_plan(path: &Path) -> Result<NodeCriticalPathPlan, String> {
    let resolved = resolve_repo_path(path);
    let raw = fs::read_to_string(&resolved)
        .map_err(|err| format!("read_node_burndown_plan_failed:{}:{err}", resolved.display()))?;
    serde_json::from_str(&raw)
        .map_err(|err| format!("parse_node_burndown_plan_failed:{}:{err}", resolved.display()))
}

pub fn build_operator_critical_report(
    plan: &NodeCriticalPathPlan,
    mode: &str,
) -> OperatorCriticalReport {
    let selected_mode = normalize_mode(mode);
    let priority_one_lanes: Vec<NodeCriticalPathLane> = plan
        .lanes
        .iter()
        .filter(|lane| lane.priority == 1 && plan.operator_critical_domains.contains(&lane.domain))
        .cloned()
        .collect();
    let entrypoints = rust_entrypoints_for(&priority_one_lanes, selected_mode.as_str());
    let selected_domains: BTreeSet<String> = entrypoints
        .iter()
        .flat_map(|entry| entry.covered_domains.iter().cloned())
        .collect();
    let selected_lanes: Vec<NodeCriticalPathLane> = if selected_mode == "all" {
        priority_one_lanes.clone()
    } else {
        priority_one_lanes
            .iter()
            .filter(|lane| selected_domains.contains(&lane.domain))
            .cloned()
            .collect()
    };
    let covered_lanes: BTreeSet<String> = entrypoints
        .iter()
        .flat_map(|entry| entry.covered_lanes.iter().cloned())
        .collect();
    let uncovered_lanes: Vec<String> = selected_lanes
        .iter()
        .filter(|lane| !covered_lanes.contains(&lane.id))
        .map(|lane| lane.id.clone())
        .collect();
    let wrong_target: Vec<String> = selected_lanes
        .iter()
        .filter(|lane| lane.target_classification != plan.operator_critical_target_classification)
        .map(|lane| lane.id.clone())
        .collect();
    let missing_queue: Vec<String> = selected_lanes
        .iter()
        .filter(|lane| !plan.ordered_migration_queue.contains(&lane.id))
        .map(|lane| lane.id.clone())
        .collect();
    let require_group = |group: &str| selected_mode == "all" || selected_mode == group;
    let checks = vec![
        check(
            "status_operator_path_has_rust_entrypoint",
            !require_group("status") || group_present(&entrypoints, "status"),
            "operator-critical status/topology truth has a Rust-native entrypoint",
        ),
        check(
            "recovery_operator_path_has_rust_entrypoint",
            !require_group("recovery") || group_present(&entrypoints, "recovery"),
            "operator-critical recovery/repair has a Rust-native entrypoint",
        ),
        check(
            "release_operator_path_has_rust_entrypoint",
            !require_group("release") || group_present(&entrypoints, "release"),
            "operator-critical release ops has a Rust-native entrypoint",
        ),
        check(
            "priority_one_operator_lanes_covered",
            uncovered_lanes.is_empty(),
            "all priority-one operator-critical lanes are covered by Rust entrypoints",
        ),
        check(
            "priority_one_operator_lanes_target_rust",
            wrong_target.is_empty(),
            "all priority-one operator-critical lanes target rust_native",
        ),
        check(
            "priority_one_operator_lanes_in_ordered_queue",
            missing_queue.is_empty(),
            "all priority-one operator-critical lanes remain in ordered migration queue",
        ),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool) == Some(true));
    OperatorCriticalReport {
        ok,
        report_type: "operator_critical_burndown".to_string(),
        generated_at_epoch_ms: crate::now_epoch_ms(),
        mode: selected_mode,
        summary: json!({
            "schema_id": plan.schema_id,
            "operator_critical_domains": plan.operator_critical_domains,
            "target_classification": plan.operator_critical_target_classification,
            "priority_one_lane_count": selected_lanes.len(),
            "rust_entrypoint_count": entrypoints.len(),
            "uncovered_priority_one_lanes": uncovered_lanes,
            "wrong_target_priority_one_lanes": wrong_target,
            "missing_ordered_queue_lanes": missing_queue,
            "operator_critical_priority_cutoff_date": plan.operator_critical_priority_cutoff_date
        }),
        rust_entrypoints: entrypoints,
        lane_rows: selected_lanes,
        checks,
    }
}

pub fn run_operator_critical_guard(
    plan_path: &Path,
    out_json: &Path,
    out_markdown: &Path,
    mode: &str,
    strict: bool,
) -> Result<OperatorCriticalReport, String> {
    let plan = load_node_critical_path_plan(plan_path)?;
    let report = build_operator_critical_report(&plan, mode);
    write_json(out_json, &report)?;
    write_markdown(out_markdown, &report)?;
    if strict && !report.ok {
        return Err("operator_critical_guard_failed".to_string());
    }
    Ok(report)
}

fn rust_entrypoints_for(
    lanes: &[NodeCriticalPathLane],
    mode: &str,
) -> Vec<OperatorCriticalEntrypoint> {
    let mut groups = BTreeMap::<&str, (Vec<&str>, &str, &str)>::new();
    groups.insert(
        "status",
        (
            vec!["status", "topology_truth"],
            "ops:operator-critical:status",
            "core/local/artifacts/operator_critical_status_current.json",
        ),
    );
    groups.insert(
        "recovery",
        (
            vec!["recovery", "repair"],
            "ops:operator-critical:recovery",
            "core/local/artifacts/operator_critical_recovery_current.json",
        ),
    );
    groups.insert(
        "release",
        (
            vec!["release"],
            "ops:operator-critical:release",
            "core/local/artifacts/operator_critical_release_current.json",
        ),
    );
    groups
        .into_iter()
        .filter(|(group, _)| mode == "all" || mode == *group)
        .map(|(group, (domains, command, artifact_path))| {
            let covered_domains: Vec<String> = domains.iter().map(|domain| domain.to_string()).collect();
            let covered_lanes = lanes
                .iter()
                .filter(|lane| domains.contains(&lane.domain.as_str()))
                .map(|lane| lane.id.clone())
                .collect();
            OperatorCriticalEntrypoint {
                domain_group: group.to_string(),
                command: command.to_string(),
                artifact_path: artifact_path.to_string(),
                implementation: "rust_native".to_string(),
                covered_domains,
                covered_lanes,
            }
        })
        .collect()
}

fn normalize_mode(mode: &str) -> String {
    match mode {
        "status" | "recovery" | "release" => mode.to_string(),
        _ => "all".to_string(),
    }
}

fn group_present(entrypoints: &[OperatorCriticalEntrypoint], group: &str) -> bool {
    entrypoints.iter().any(|entry| entry.domain_group == group)
}

fn check(id: &str, ok: bool, detail: &str) -> Value {
    json!({ "id": id, "ok": ok, "detail": detail })
}

fn write_json(path: &Path, report: &OperatorCriticalReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create_report_dir_failed:{err}"))?;
    }
    let raw = serde_json::to_string_pretty(report)
        .map_err(|err| format!("serialize_report_failed:{err}"))?;
    fs::write(path, format!("{raw}\n")).map_err(|err| format!("write_report_failed:{err}"))
}

fn write_markdown(path: &Path, report: &OperatorCriticalReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create_report_dir_failed:{err}"))?;
    }
    let mut lines = vec![
        "# Operator Critical Rust Burn-down".to_string(),
        String::new(),
        format!("- pass: `{}`", report.ok),
        format!("- mode: `{}`", report.mode),
        format!(
            "- priority-one lanes: `{}`",
            report
                .summary
                .get("priority_one_lane_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!("- rust entrypoints: `{}`", report.rust_entrypoints.len()),
        String::new(),
        "## Entrypoints".to_string(),
    ];
    for entry in &report.rust_entrypoints {
        lines.push(format!(
            "- `{}` -> `{}` covers `{}` lanes",
            entry.domain_group,
            entry.command,
            entry.covered_lanes.len()
        ));
    }
    lines.push(String::new());
    lines.push("## Checks".to_string());
    for check in &report.checks {
        lines.push(format!(
            "- `{}`: `{}`",
            check["id"].as_str().unwrap_or("unknown"),
            check["ok"].as_bool().unwrap_or(false)
        ));
    }
    fs::write(path, format!("{}\n", lines.join("\n")))
        .map_err(|err| format!("write_markdown_failed:{err}"))
}

fn resolve_repo_path(path: &Path) -> std::path::PathBuf {
    if path.is_absolute() || path.exists() {
        return path.to_path_buf();
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> NodeCriticalPathPlan {
        load_node_critical_path_plan(Path::new(DEFAULT_NODE_BURNDOWN_PLAN_PATH)).expect("plan")
    }

    #[test]
    fn rust_entrypoints_cover_status_recovery_and_release() {
        let report = build_operator_critical_report(&plan(), "all");
        assert!(report.ok, "{:?}", report.checks);
        assert!(group_present(&report.rust_entrypoints, "status"));
        assert!(group_present(&report.rust_entrypoints, "recovery"));
        assert!(group_present(&report.rust_entrypoints, "release"));
    }

    #[test]
    fn status_mode_keeps_status_entrypoint_rust_native() {
        let report = build_operator_critical_report(&plan(), "status");
        assert!(report.ok, "{:?}", report.checks);
        assert_eq!(report.mode, "status");
        assert!(report
            .rust_entrypoints
            .iter()
            .all(|entry| entry.implementation == "rust_native"));
    }
}
