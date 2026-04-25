use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CHAT_MONITOR_PATH: &str =
    "local/state/ops/eval_agent_chat_monitor/issue_drafts_latest.json";
const DEFAULT_ISSUE_DRAFTS_PATH: &str = "core/local/artifacts/eval_issue_drafts_current.json";
const DEFAULT_ROUTER_PATH: &str = "core/local/artifacts/eval_feedback_router_current.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_issue_authority_current.json";
const DEFAULT_LATEST_PATH: &str = "artifacts/eval_issue_authority_latest.json";
const DEFAULT_REPORT_PATH: &str = "local/workspace/reports/EVAL_ISSUE_AUTHORITY_CURRENT.md";

const LIFECYCLE_STATES: &[&str] = &[
    "observe",
    "classify",
    "issue",
    "route_to_owner",
    "verify_fix",
    "close",
];

const QUALITY_CLASSES: &[(&str, &[&str], &str)] = &[
    (
        "hallucination",
        &["unsupported_claim_detected", "hallucination"],
        "surface/orchestration/synthesis",
    ),
    (
        "wrong_tool_selection",
        &[
            "wrong_tool_selection_detected",
            "wrong_tool_routing",
            "auto_tool_selection_claim",
            "bad_workflow_selection",
        ],
        "surface/orchestration/tool_routing",
    ),
    (
        "no_response",
        &["no_response_detected", "empty_assistant_response"],
        "surface/orchestration/finalization",
    ),
    (
        "repetitive_fallback",
        &[
            "repeated_response_loop_detected",
            "repetitive_fallback",
            "response_loop",
        ],
        "surface/orchestration/recovery",
    ),
    (
        "blocked_lane_misdiagnosis",
        &[
            "blocked_lane_misdiagnosis",
            "lease_denied",
            "policy_block_confusion",
        ],
        "surface/orchestration/recovery",
    ),
];

pub fn run_eval_issue_authority(args: &[String]) -> i32 {
    let strict = flag_value(args, "strict").unwrap_or_else(|| "0".to_string()) == "1";
    let chat_monitor_path =
        flag_value(args, "chat-monitor").unwrap_or_else(|| DEFAULT_CHAT_MONITOR_PATH.to_string());
    let issue_drafts_path =
        flag_value(args, "issue-drafts").unwrap_or_else(|| DEFAULT_ISSUE_DRAFTS_PATH.to_string());
    let router_path = flag_value(args, "router").unwrap_or_else(|| DEFAULT_ROUTER_PATH.to_string());
    let out_path = flag_value(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let latest_path =
        flag_value(args, "out-latest").unwrap_or_else(|| DEFAULT_LATEST_PATH.to_string());
    let report_path =
        flag_value(args, "out-markdown").unwrap_or_else(|| DEFAULT_REPORT_PATH.to_string());

    let report = build_report(&chat_monitor_path, &issue_drafts_path, &router_path);
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&latest_path, &report).is_ok()
        && write_text(&report_path, &markdown(&report)).is_ok();
    if !write_ok {
        eprintln!("eval_issue_authority: failed to write one or more outputs");
        return 2;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
    if strict && !bool_at(&report, &["ok"]) {
        return 1;
    }
    0
}

fn build_report(chat_monitor_path: &str, issue_drafts_path: &str, router_path: &str) -> Value {
    let chat_monitor = read_json(chat_monitor_path);
    let issue_drafts = read_json(issue_drafts_path);
    let router = read_json(router_path);
    let mut issues = Vec::new();
    collect_issues("eval_agent_chat_monitor", &chat_monitor, &mut issues);
    collect_issues("eval_issue_drafts", &issue_drafts, &mut issues);
    let router_routes = router
        .get("routes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let checks = vec![
        json!({
            "id": "eval_issue_authority_owner_contract",
            "ok": true,
            "detail": "owner=surface_orchestration_eval_issue_synthesis"
        }),
        json!({
            "id": "eval_issue_lifecycle_state_contract",
            "ok": lifecycle_states().len() == 6,
            "detail": lifecycle_states().join(",")
        }),
        json!({
            "id": "eval_issue_quality_taxonomy_contract",
            "ok": quality_gates().len() == 5,
            "detail": "hallucination,wrong_tool_selection,no_response,repetitive_fallback,blocked_lane_misdiagnosis"
        }),
        json!({
            "id": "eval_issue_source_monitoring_contract",
            "ok": Path::new(chat_monitor_path).exists() && Path::new(issue_drafts_path).exists(),
            "detail": format!("chat_monitor={chat_monitor_path};issue_drafts={issue_drafts_path}")
        }),
        json!({
            "id": "eval_issue_route_to_owner_contract",
            "ok": issues.iter().all(|row| str_at(row, &["owner_component"], "").starts_with("surface/orchestration")),
            "detail": format!("issues={}", issues.len())
        }),
        json!({
            "id": "eval_issue_verify_fix_close_contract",
            "ok": issues.iter().all(|row| {
                !str_at(row, &["verification", "suggested_test"], "").is_empty()
                    && !str_at(row, &["closure_policy"], "").is_empty()
            }),
            "detail": "every synthesized issue carries suggested_test and closure_policy"
        }),
        json!({
            "id": "eval_feedback_router_state_contract",
            "ok": router_routes.iter().all(|row| {
                !str_at(row, &["destination"], "").is_empty()
                    && !str_at(row, &["action"], "").is_empty()
                    && !str_at(row, &["receipt_type"], "").is_empty()
            }),
            "detail": format!("routes={}", router_routes.len())
        }),
    ];
    let ok = checks.iter().all(|row| bool_at(row, &["ok"]));
    json!({
        "type": "eval_issue_authority",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "owner": "surface_orchestration_eval_issue_synthesis",
        "lifecycle_states": lifecycle_states(),
        "quality_gates": quality_gates(),
        "summary": {
            "issue_count": issues.len(),
            "router_route_count": router_routes.len(),
            "quality_gate_count": quality_gates().len(),
            "lifecycle_state_count": lifecycle_states().len()
        },
        "issues": issues,
        "checks": checks,
        "sources": {
            "chat_monitor": chat_monitor_path,
            "issue_drafts": issue_drafts_path,
            "router": router_path
        }
    })
}

fn collect_issues(source: &str, payload: &Value, out: &mut Vec<Value>) {
    let rows = payload
        .get("issue_drafts")
        .or_else(|| payload.get("drafts"))
        .or_else(|| payload.get("issues"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in rows {
        let issue_class = canonical_issue_class(&[
            str_at(&row, &["issue_class"], ""),
            str_at(&row, &["id"], ""),
            str_at(&row, &["title"], ""),
            str_at(&row, &["failure_class"], ""),
        ]);
        out.push(json!({
            "id": str_at(&row, &["id"], "eval_issue"),
            "source": source,
            "issue_class": issue_class,
            "severity": str_at(&row, &["severity"], "medium"),
            "owner_component": owner_for_class(&issue_class),
            "lifecycle": lifecycle_states(),
            "route_state": "route_to_owner",
            "verification": {
                "suggested_test": str_at(&row, &["suggested_test"], "replay eval issue and verify non-recurrence"),
                "replay_command": str_at(&row, &["replay_command"], "cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin eval_runtime -- issue-authority --strict=1")
            },
            "closure_policy": "close only after verify_fix passes and no matching issue recurs in the live eval stream"
        }));
    }
}

fn canonical_issue_class(values: &[String]) -> String {
    let joined = values.join(" ").to_ascii_lowercase();
    for (class, aliases, _) in QUALITY_CLASSES {
        if aliases.iter().any(|alias| joined.contains(alias)) || joined.contains(class) {
            return (*class).to_string();
        }
    }
    "unknown_eval_issue".to_string()
}

fn owner_for_class(issue_class: &str) -> &'static str {
    QUALITY_CLASSES
        .iter()
        .find(|(class, _, _)| *class == issue_class)
        .map(|(_, _, owner)| *owner)
        .unwrap_or("surface/orchestration/eval")
}

fn lifecycle_states() -> Vec<&'static str> {
    LIFECYCLE_STATES.to_vec()
}

fn quality_gates() -> Vec<Value> {
    QUALITY_CLASSES
        .iter()
        .map(|(class, detectors, owner)| {
            json!({
                "issue_class": class,
                "detectors": detectors,
                "owner_component": owner,
                "gate": format!("eval_quality_gate::{class}")
            })
        })
        .collect()
}

fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
}

fn write_text(path: &str, value: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, value)
}

fn markdown(report: &Value) -> String {
    format!(
        "# Eval Issue Authority\n\n- ok: {}\n- owner: {}\n- issues: {}\n- lifecycle_states: {}\n- quality_gates: {}\n",
        bool_at(report, &["ok"]),
        str_at(report, &["owner"], ""),
        report.pointer("/summary/issue_count").and_then(Value::as_u64).unwrap_or(0),
        lifecycle_states().join(", "),
        QUALITY_CLASSES.iter().map(|(class, _, _)| *class).collect::<Vec<_>>().join(", ")
    )
}

fn flag_value(args: &[String], key: &str) -> Option<String> {
    let inline = format!("--{key}=");
    for idx in 0..args.len() {
        if let Some(value) = args[idx].strip_prefix(&inline) {
            return Some(value.to_string());
        }
        if args[idx] == format!("--{key}") {
            return args.get(idx + 1).cloned();
        }
    }
    None
}

fn str_at(value: &Value, path: &[&str], default: &str) -> String {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return default.to_string();
        };
        cursor = next;
    }
    cursor.as_str().unwrap_or(default).trim().to_string()
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

fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_real_eval_failure_modes() {
        assert_eq!(
            canonical_issue_class(&["unsupported_claim_detected".to_string()]),
            "hallucination"
        );
        assert_eq!(
            canonical_issue_class(&["wrong_tool_routing".to_string()]),
            "wrong_tool_selection"
        );
        assert_eq!(
            canonical_issue_class(&["empty_assistant_response".to_string()]),
            "no_response"
        );
        assert_eq!(
            canonical_issue_class(&["repeated_response_loop_detected".to_string()]),
            "repetitive_fallback"
        );
        assert_eq!(
            canonical_issue_class(&["response_loop".to_string()]),
            "repetitive_fallback"
        );
        assert_eq!(
            canonical_issue_class(&["bad_workflow_selection".to_string()]),
            "wrong_tool_selection"
        );
        assert_eq!(
            canonical_issue_class(&["lease_denied:client_ingress_domain_boundary".to_string()]),
            "blocked_lane_misdiagnosis"
        );
    }

    #[test]
    fn lifecycle_states_include_full_feedback_route() {
        assert_eq!(
            lifecycle_states(),
            vec![
                "observe",
                "classify",
                "issue",
                "route_to_owner",
                "verify_fix",
                "close"
            ]
        );
        assert_eq!(quality_gates().len(), 5);
    }
}
