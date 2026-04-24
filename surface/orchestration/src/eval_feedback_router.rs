use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_REGRESSION_PATH: &str = "core/local/artifacts/eval_regression_guard_current.json";
const DEFAULT_ISSUE_DRAFTS_PATH: &str = "core/local/artifacts/eval_issue_drafts_current.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_feedback_router_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_feedback_router_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/EVAL_FEEDBACK_ROUTER_CURRENT.md";

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
        EnforcementDestination::KernelBlock => (
            "block_release_or_runtime_escalation",
            "kernel_block_event",
        ),
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

fn destination_count(routes: &[Value], destination: EnforcementDestination) -> usize {
    routes
        .iter()
        .filter(|row| str_at(row, &["destination"]) == Some(destination.as_str()))
        .count()
}

fn route_is_well_formed(route: &Value) -> bool {
    ["source", "failure_id", "failure_class", "severity", "destination", "action", "receipt_type"]
        .iter()
        .all(|field| str_at(route, &[*field]).is_some())
}

fn build_report(regression_path: &str, issue_drafts_path: &str) -> Value {
    let regression = read_json(regression_path);
    let issue_drafts = read_json(issue_drafts_path);
    let mut routes = Vec::new();
    push_issue_routes(&mut routes, &issue_drafts);
    push_regression_routes(&mut routes, &regression);
    let malformed = routes.iter().filter(|row| !route_is_well_formed(row)).count();
    let regression_failed = !bool_at(&regression, &["ok"]);
    let regression_blocked = !regression_failed
        || routes
            .iter()
            .any(|row| str_at(row, &["destination"]) == Some("kernel_block"));
    let checks = vec![
        json!({"id": "eval_feedback_route_coverage_contract", "ok": malformed == 0, "detail": format!("routes={};malformed={malformed}", routes.len())}),
        json!({"id": "eval_feedback_destination_set_contract", "ok": routes.iter().all(|row| matches!(str_at(row, &["destination"]), Some("control_plane_retry" | "gateway_quarantine" | "kernel_block"))), "detail": "destinations=control_plane_retry,gateway_quarantine,kernel_block"}),
        json!({"id": "eval_regression_release_block_contract", "ok": regression_blocked, "detail": format!("regression_failed={regression_failed};kernel_block_routes={}", destination_count(&routes, EnforcementDestination::KernelBlock))}),
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
            "control_plane_retry": destination_count(&routes, EnforcementDestination::ControlPlaneRetry),
            "gateway_quarantine": destination_count(&routes, EnforcementDestination::GatewayQuarantine),
            "kernel_block": destination_count(&routes, EnforcementDestination::KernelBlock),
            "eval_release_gate": if regression_failed { "blocked" } else { "not_blocked" },
        },
        "checks": checks,
        "routes": routes,
        "sources": {
            "eval_regression_guard": regression_path,
            "eval_issue_drafts": issue_drafts_path,
        }
    })
}

fn markdown(report: &Value) -> String {
    format!(
        "# Eval Feedback Router (Current)\n\n- generated_at: {}\n- ok: {}\n- route_count: {}\n- control_plane_retry: {}\n- gateway_quarantine: {}\n- kernel_block: {}\n- eval_release_gate: {}\n",
        str_at(report, &["generated_at"]).unwrap_or(""),
        bool_at(report, &["ok"]),
        report.pointer("/summary/route_count").and_then(Value::as_u64).unwrap_or(0),
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
}
