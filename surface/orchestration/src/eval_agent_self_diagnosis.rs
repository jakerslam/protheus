// SRS: V12-MISTY-HEALTH-WAVE5-001
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_FEEDBACK_DIR: &str = "local/state/ops/eval_agent_feedback";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_agent_self_diagnosis_current.json";
const DEFAULT_LATEST_PATH: &str = "artifacts/eval_agent_self_diagnosis_latest.json";
const DEFAULT_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_AGENT_SELF_DIAGNOSIS_CURRENT.md";

pub fn run_eval_agent_self_diagnosis(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let agent_id = parse_flag(args, "agent-id")
        .or_else(|| parse_flag(args, "agent"))
        .map(|raw| normalize_agent_id(&raw))
        .unwrap_or_default();
    let feedback_dir =
        parse_flag(args, "feedback-dir").unwrap_or_else(|| DEFAULT_FEEDBACK_DIR.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let latest_path = parse_flag(args, "latest").unwrap_or_else(|| DEFAULT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());

    let report = build_self_diagnosis_report(&agent_id, &feedback_dir);
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&latest_path, &report).is_ok()
        && write_markdown(&markdown_path, &markdown_report(&report)).is_ok();
    if !write_ok {
        eprintln!("eval_agent_self_diagnosis: failed to write one or more outputs");
        return 2;
    }
    print_json_line(&report);
    if strict && !report.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        1
    } else {
        0
    }
}

fn build_self_diagnosis_report(agent_id: &str, feedback_dir: &str) -> Value {
    let normalized_agent = normalize_agent_id(agent_id);
    let state_path = Path::new(feedback_dir).join(format!("{normalized_agent}.json"));
    let state = read_json(state_path.to_str().unwrap_or(""));
    let scope = scoped_agent_ids(&state, &normalized_agent);
    let items = state
        .get("visible_feedback_items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let diagnosis_items = items.iter().map(diagnosis_item).collect::<Vec<_>>();
    let leaked = items
        .iter()
        .filter(|item| {
            let related = item_related_agent_id(item);
            !related.is_empty() && !scope.contains(&related)
        })
        .count();
    let unattributed = items
        .iter()
        .filter(|item| item_related_agent_id(item).is_empty())
        .count();
    let context = self_diagnosis_context(&normalized_agent, &scope, &diagnosis_items);
    let checks = vec![
        json!({
            "id": "self_diagnosis_agent_required",
            "ok": !normalized_agent.is_empty(),
            "detail": "agent-id must be explicit before scoped feedback is exposed"
        }),
        json!({
            "id": "self_diagnosis_scoped_feedback_only",
            "ok": leaked == 0 && unattributed == 0,
            "detail": format!("visible_items={};scope_leaks={leaked};unattributed_items={unattributed}", items.len())
        }),
        json!({
            "id": "self_diagnosis_context_available",
            "ok": !diagnosis_items.is_empty() || context.contains("No scoped eval feedback"),
            "detail": "diagnosis context is derived only from scoped eval feedback state"
        }),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    json!({
        "type": "eval_agent_self_diagnosis",
        "schema_version": 1,
        "contract": "agent_self_diagnosis_scoped_feedback_v1",
        "generated_at": now_iso_like(),
        "ok": ok,
        "agent_id": normalized_agent,
        "visibility_rule": "agent_may_read_self_and_descendant_eval_feedback_only",
        "scope_agent_ids": scope.iter().cloned().collect::<Vec<_>>(),
        "summary": {
            "visible_feedback_item_count": diagnosis_items.len(),
            "scope_leak_count": leaked,
            "unattributed_item_count": unattributed
        },
        "self_diagnosis_input": {
            "allowed_sources": ["self_eval_feedback", "descendant_eval_feedback"],
            "diagnosis_context": context,
            "items": diagnosis_items
        },
        "checks": checks,
        "sources": {
            "feedback_state": state_path.to_string_lossy()
        }
    })
}

fn scoped_agent_ids(state: &Value, agent_id: &str) -> BTreeSet<String> {
    let mut scope = string_array_at(state, &["scope_agent_ids"])
        .into_iter()
        .map(|raw| normalize_agent_id(&raw))
        .filter(|raw| !raw.is_empty())
        .collect::<BTreeSet<_>>();
    if !agent_id.is_empty() {
        scope.insert(agent_id.to_string());
    }
    scope
}

fn diagnosis_item(item: &Value) -> Value {
    json!({
        "id": required_str(item, &["id"], ""),
        "source_kind": required_str(item, &["source_kind"], ""),
        "severity": required_str(item, &["severity"], "info"),
        "title": required_str(item, &["title"], "Eval feedback"),
        "owner_component": required_str(item, &["owner_component"], ""),
        "issue_class": required_str(item, &["issue_class"], ""),
        "related_agent_id": item_related_agent_id(item),
        "expected_fix": required_str(item, &["expected_fix"], ""),
        "suggested_test": required_str(item, &["suggested_test"], ""),
        "replay_command": required_str(item, &["replay_command"], ""),
        "acceptance_criteria": string_array_at(item, &["acceptance_criteria"]),
    })
}

fn item_related_agent_id(item: &Value) -> String {
    for path in [
        ["related_agent_id"].as_slice(),
        ["agent_id"].as_slice(),
        ["raw_event", "related_agent_id"].as_slice(),
        ["raw_event", "agent_id"].as_slice(),
    ] {
        let candidate = normalize_agent_id(required_str(item, path, "").as_str());
        if !candidate.is_empty() {
            return candidate;
        }
    }
    String::new()
}

fn self_diagnosis_context(agent_id: &str, scope: &BTreeSet<String>, items: &[Value]) -> String {
    if items.is_empty() {
        return format!("No scoped eval feedback is currently visible to {agent_id}.");
    }
    let mut lines = vec![format!(
        "Agent {agent_id} may diagnose only feedback for scoped agents: {}.",
        scope.iter().cloned().collect::<Vec<_>>().join(", ")
    )];
    for item in items {
        let severity = required_str(item, &["severity"], "info");
        let title = required_str(item, &["title"], "Eval feedback");
        let related = required_str(item, &["related_agent_id"], "");
        let replay = required_str(item, &["replay_command"], "");
        lines.push(format!(
            "- [{severity}] {title}; related_agent={related}; replay={replay}"
        ));
    }
    clean_text(&lines.join("\n"), 8_000)
}

fn markdown_report(report: &Value) -> String {
    format!(
        "# Eval Agent Self-Diagnosis\n\n- generated_at: {}\n- ok: {}\n- agent_id: {}\n- visible_feedback_item_count: {}\n- scope_leak_count: {}\n",
        required_str(report, &["generated_at"], ""),
        report.get("ok").and_then(Value::as_bool).unwrap_or(false),
        required_str(report, &["agent_id"], ""),
        report
            .pointer("/summary/visible_feedback_item_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        report
            .pointer("/summary/scope_leak_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    )
}

fn write_markdown(path: impl AsRef<Path>, content: &str) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline) {
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

fn write_json(path: impl AsRef<Path>, value: &Value) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
}

fn print_json_line(value: &Value) {
    let _ = writeln!(
        io::stdout(),
        "{}",
        serde_json::to_string(value).unwrap_or_default()
    );
}

fn str_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_str().map(str::trim).filter(|raw| !raw.is_empty())
}

fn required_str(value: &Value, path: &[&str], default: &str) -> String {
    clean_text(str_at(value, path).unwrap_or(default), 2_000)
}

fn array_at(value: &Value, path: &[&str]) -> Vec<Value> {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return Vec::new();
        };
        cursor = next;
    }
    cursor.as_array().cloned().unwrap_or_default()
}

fn string_array_at(value: &Value, path: &[&str]) -> Vec<String> {
    array_at(value, path)
        .iter()
        .filter_map(Value::as_str)
        .map(|raw| clean_text(raw, 500))
        .filter(|raw| !raw.is_empty())
        .collect()
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect()
}

fn normalize_agent_id(raw: &str) -> String {
    clean_text(raw, 160)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect()
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
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{name}_{}", now_iso_like().replace(':', "_")))
    }

    #[test]
    fn agent_self_diagnosis_reads_only_self_and_descendant_feedback() {
        let root = temp_dir("agent_self_diagnosis_scope");
        fs::create_dir_all(&root).expect("root");
        write_json(
            root.join("parent-agent.json"),
            &json!({
                "agent_id": "parent-agent",
                "scope_agent_ids": ["parent-agent", "child-agent"],
                "visible_feedback_items": [{
                    "id": "issue-1",
                    "title": "Child repeated fallback",
                    "severity": "high",
                    "related_agent_id": "child-agent",
                    "owner_component": "surface.orchestration.finalization",
                    "replay_command": "cargo run -- synthetic-user-chat-harness --agent-id=child-agent"
                }]
            }),
        )
        .expect("state write");

        let report = build_self_diagnosis_report("parent-agent", root.to_str().unwrap());
        assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            report
                .pointer("/self_diagnosis_input/items/0/related_agent_id")
                .and_then(Value::as_str),
            Some("child-agent")
        );
        assert!(
            report
                .pointer("/self_diagnosis_input/diagnosis_context")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("Child repeated fallback")
        );
    }

    #[test]
    fn agent_self_diagnosis_fails_closed_on_cross_agent_leak() {
        let root = temp_dir("agent_self_diagnosis_leak");
        fs::create_dir_all(&root).expect("root");
        write_json(
            root.join("agent-a.json"),
            &json!({
                "agent_id": "agent-a",
                "scope_agent_ids": ["agent-a"],
                "visible_feedback_items": [{
                    "id": "issue-foreign",
                    "title": "Foreign issue leaked",
                    "related_agent_id": "agent-b"
                }]
            }),
        )
        .expect("state write");

        let report = build_self_diagnosis_report("agent-a", root.to_str().unwrap());
        assert_eq!(report.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            report.pointer("/summary/scope_leak_count").and_then(Value::as_u64),
            Some(1)
        );
    }
}
