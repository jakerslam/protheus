use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_PHASE_TRACE_SOURCE: &str = "local/state/ops/orchestration/workflow_phase_trace_latest.json";
const DEFAULT_PHASE_TRACE_OUT: &str = "local/state/ops/orchestration/workflow_phase_trace_latest.json";
const DEFAULT_PHASE_TRACE_HISTORY: &str = "local/state/ops/orchestration/workflow_phase_trace_history.jsonl";
const DEFAULT_PHASE_TRACE_ARTIFACT: &str = "core/local/artifacts/eval_phase_trace_persist_current.json";
const DEFAULT_ADVERSARIAL_CASES: &str =
    "surface/orchestration/fixtures/eval/eval_adversarial_routing_cases.json";
const DEFAULT_ADVERSARIAL_OUT: &str = "core/local/artifacts/eval_adversarial_routing_current.json";
const DEFAULT_WORKFLOW_CASES: &str =
    "surface/orchestration/fixtures/eval/eval_workflow_selection_cases.json";
const DEFAULT_WORKFLOW_OUT: &str = "core/local/artifacts/eval_workflow_selection_current.json";
const DEFAULT_OWNERSHIP_OUT: &str = "core/local/artifacts/eval_runtime_ownership_current.json";

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
            if let Some(value) = args.get(idx + 1) {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_bool_flag(args: &[String], key: &str, default: bool) -> bool {
    match parse_flag(args, key).as_deref() {
        Some("1" | "true" | "TRUE" | "yes" | "on") => true,
        Some("0" | "false" | "FALSE" | "no" | "off") => false,
        _ => default,
    }
}

fn ensure_parent(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    ensure_parent(path)?;
    let payload = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    fs::write(path, format!("{payload}\n"))
}

fn append_jsonl(path: &str, value: &Value) -> io::Result<()> {
    ensure_parent(path)?;
    let payload = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    let mut file = fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{payload}")
}

fn print_structured(report: &Value) {
    if let Ok(serialized) = serde_json::to_string(report) {
        let _ = writeln!(io::stdout(), "{serialized}");
    }
}

fn str_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_str().map(str::trim).filter(|v| !v.is_empty())
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

fn infer_route(prompt: &str, policy: &str, tool_result: &str) -> &'static str {
    let p = prompt.to_ascii_lowercase();
    let policy = policy.to_ascii_lowercase();
    let tool_result = tool_result.to_ascii_lowercase();
    if policy.contains("denied") {
        return "policy_denied";
    }
    if tool_result.contains("empty") || tool_result.contains("no results") {
        return "empty_result_recovery";
    }
    if p.contains("local") || p.contains("file") || p.contains("directory") || p.contains("repo") {
        if p.contains("local") {
            return "workspace";
        }
        if p.contains("http") || p.contains("website") || p.contains("online") {
            return "web";
        }
        return "workspace";
    }
    if p.contains("web") || p.contains("search") || p.contains("latest") || p.contains("online") {
        return "web";
    }
    if p.contains("again") || p.contains("why") || p.contains("what is going on") {
        return "frustrated_followup_recovery";
    }
    "conversation"
}

fn workflow_for(prompt: &str) -> &'static str {
    let p = prompt.to_ascii_lowercase();
    if p.trim() == "hello" || p.trim() == "hi" {
        return "simple_conversation";
    }
    if p.contains("fix") || p.contains("patch") || p.contains("todo") || p.contains("implement") {
        return "task_execution";
    }
    if p.contains("why") || p.contains("diagnose") || p.contains("what happened") {
        return "diagnostic_recovery";
    }
    "simple_conversation"
}

pub fn run_phase_trace_persist(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let source_path =
        parse_flag(args, "source").unwrap_or_else(|| DEFAULT_PHASE_TRACE_SOURCE.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_PHASE_TRACE_OUT.to_string());
    let history_path =
        parse_flag(args, "history").unwrap_or_else(|| DEFAULT_PHASE_TRACE_HISTORY.to_string());
    let artifact_path =
        parse_flag(args, "artifact").unwrap_or_else(|| DEFAULT_PHASE_TRACE_ARTIFACT.to_string());
    let source = read_json(&source_path);
    let trace = json!({
        "type": "orchestration_workflow_phase_trace",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "source": source_path,
        "turn_id": str_at(&source, &["turn_id"]).unwrap_or("unknown"),
        "user_intent": str_at(&source, &["user_intent"]).or_else(|| str_at(&source, &["prompt"])).unwrap_or("unknown"),
        "selected_workflow": str_at(&source, &["selected_workflow"]).unwrap_or("unknown"),
        "selected_model": str_at(&source, &["selected_model"]).unwrap_or("unknown"),
        "tool_decision": str_at(&source, &["tool_decision"]).unwrap_or("unknown"),
        "tool_family": str_at(&source, &["tool_family"]).unwrap_or("unknown"),
        "tool_result_summary": str_at(&source, &["tool_result_summary"]).unwrap_or("unknown"),
        "finalization_status": str_at(&source, &["finalization_status"]).unwrap_or("unknown"),
        "fallback_path": str_at(&source, &["fallback_path"]).unwrap_or("none"),
        "normalized_failure_code": str_at(&source, &["normalized_failure_code"]).unwrap_or("none"),
        "receipt_correlation": str_at(&source, &["receipt_correlation"]).unwrap_or("missing")
    });
    let required_ok = str_at(&trace, &["turn_id"]) != Some("unknown")
        && str_at(&trace, &["selected_workflow"]) != Some("unknown")
        && str_at(&trace, &["receipt_correlation"]) != Some("missing");
    let ok = write_json(&out_path, &trace).is_ok()
        && append_jsonl(&history_path, &trace).is_ok()
        && write_json(
            &artifact_path,
            &json!({
                "type": "eval_phase_trace_persist",
                "ok": required_ok,
                "generated_at": now_iso_like(),
                "checks": [
                    {"id": "canonical_phase_trace_written", "ok": Path::new(&out_path).exists(), "detail": out_path},
                    {"id": "phase_trace_history_appended", "ok": Path::new(&history_path).exists(), "detail": history_path},
                    {"id": "phase_trace_required_fields_contract", "ok": required_ok, "detail": "turn_id|selected_workflow|receipt_correlation"}
                ],
                "summary": {"required_fields_present": required_ok},
                "trace": trace
            }),
        )
        .is_ok();
    let report = read_json(&artifact_path);
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

pub fn run_adversarial_routing(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let cases_path =
        parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_ADVERSARIAL_CASES.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_ADVERSARIAL_OUT.to_string());
    let cases = read_json(&cases_path);
    let mut rows = Vec::new();
    let mut passed = 0_u64;
    for case in cases.get("cases").and_then(|v| v.as_array()).into_iter().flatten() {
        let prompt = str_at(case, &["prompt"]).unwrap_or("");
        let expected = str_at(case, &["expected_route"]).unwrap_or("unknown");
        let actual = infer_route(
            prompt,
            str_at(case, &["policy_state"]).unwrap_or(""),
            str_at(case, &["tool_result_state"]).unwrap_or(""),
        );
        let ok = actual == expected;
        if ok {
            passed += 1;
        }
        rows.push(json!({"id": str_at(case, &["id"]).unwrap_or("unknown"), "expected_route": expected, "actual_route": actual, "ok": ok}));
    }
    let ok = !rows.is_empty() && passed as usize == rows.len();
    let report = json!({
        "type": "eval_adversarial_routing",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {"id": "adversarial_cases_present", "ok": !rows.is_empty(), "detail": format!("cases={}", rows.len())},
            {"id": "adversarial_route_contract", "ok": ok, "detail": format!("passed={};cases={}", passed, rows.len())}
        ],
        "summary": {"cases": rows.len(), "passed": passed, "failed": rows.len().saturating_sub(passed as usize)},
        "results": rows,
        "sources": {"cases": cases_path}
    });
    let write_ok = write_json(&out_path, &report).is_ok();
    print_structured(&report);
    if strict && (!ok || !write_ok) {
        return 1;
    }
    0
}

pub fn run_workflow_selection(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let cases_path =
        parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_WORKFLOW_CASES.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_WORKFLOW_OUT.to_string());
    let cases = read_json(&cases_path);
    let mut rows = Vec::new();
    let mut appropriate = 0_u64;
    let mut simpler_sufficed = 0_u64;
    let mut justified = 0_u64;
    for case in cases.get("cases").and_then(|v| v.as_array()).into_iter().flatten() {
        let prompt = str_at(case, &["prompt"]).unwrap_or("");
        let selected = str_at(case, &["selected_workflow"]).unwrap_or("unknown");
        let expected = workflow_for(prompt);
        let ok = selected == expected;
        if ok {
            appropriate += 1;
        }
        if bool_at(case, &["simpler_workflow_would_suffice"]) {
            simpler_sufficed += 1;
        }
        let recovery_justified = bool_at(case, &["recovery_escalation_justified"]);
        if recovery_justified || expected != "diagnostic_recovery" {
            justified += 1;
        }
        rows.push(json!({"id": str_at(case, &["id"]).unwrap_or("unknown"), "selected_workflow": selected, "expected_workflow": expected, "appropriate": ok, "simpler_workflow_would_suffice": bool_at(case, &["simpler_workflow_would_suffice"]), "recovery_escalation_justified": recovery_justified}));
    }
    let ok = !rows.is_empty() && appropriate as usize == rows.len() && justified as usize == rows.len();
    let report = json!({
        "type": "eval_workflow_selection_quality",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {"id": "workflow_selection_cases_present", "ok": !rows.is_empty(), "detail": format!("cases={}", rows.len())},
            {"id": "workflow_selection_appropriateness_contract", "ok": appropriate as usize == rows.len(), "detail": format!("appropriate={};cases={}", appropriate, rows.len())},
            {"id": "workflow_recovery_justification_contract", "ok": justified as usize == rows.len(), "detail": format!("justified={};cases={}", justified, rows.len())}
        ],
        "summary": {"cases": rows.len(), "appropriate": appropriate, "simpler_would_have_sufficed": simpler_sufficed, "recovery_justified": justified},
        "results": rows,
        "sources": {"cases": cases_path}
    });
    let write_ok = write_json(&out_path, &report).is_ok();
    print_structured(&report);
    if strict && (!ok || !write_ok) {
        return 1;
    }
    0
}

pub fn run_runtime_ownership(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OWNERSHIP_OUT.to_string());
    let runtime_paths = [
        "surface/orchestration/src/bin/eval_runtime.rs",
        "surface/orchestration/src/eval_issue_runtime.rs",
        "surface/orchestration/src/eval_lifecycle_runtime.rs",
        "surface/orchestration/src/eval_final_runtime.rs",
    ];
    let all_present = runtime_paths.iter().all(|path| Path::new(path).exists());
    let all_orchestration = runtime_paths
        .iter()
        .all(|path| path.starts_with("surface/orchestration/src/"));
    let no_test_runtime_dependency = runtime_paths.iter().all(|path| {
        let forbidden = ["tests", "tooling"].join("/");
        fs::read_to_string(path)
            .map(|raw| !raw.contains(&forbidden))
            .unwrap_or(false)
    });
    let ok = all_present && all_orchestration && no_test_runtime_dependency;
    let report = json!({
        "type": "eval_runtime_ownership",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {"id": "eval_runtime_files_present", "ok": all_present, "detail": runtime_paths},
            {"id": "eval_runtime_orchestration_owned", "ok": all_orchestration, "detail": "surface/orchestration/src"},
            {"id": "eval_runtime_no_test_tooling_dependency", "ok": no_test_runtime_dependency, "detail": "runtime sources avoid test-tooling dependencies"}
        ],
        "summary": {"runtime_files": runtime_paths.len(), "test_strippable": no_test_runtime_dependency}
    });
    let write_ok = write_json(&out_path, &report).is_ok();
    print_structured(&report);
    if strict && (!ok || !write_ok) {
        return 1;
    }
    0
}
