use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_ISSUE_SOURCE_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_gold_dataset_v1.jsonl";
const DEFAULT_ISSUE_OUT_PATH: &str = "core/local/artifacts/eval_issue_drafts_current.json";
const DEFAULT_ISSUE_OUT_LATEST_PATH: &str = "artifacts/eval_issue_drafts_latest.json";
const DEFAULT_ISSUE_MARKDOWN_PATH: &str = "local/workspace/reports/EVAL_ISSUE_DRAFTS_CURRENT.md";
const DEFAULT_REPLAY_FIXTURE_DIR: &str = "local/state/ops/eval_replay_fixtures";
const DEFAULT_REPLAY_OUT_PATH: &str = "core/local/artifacts/eval_replay_runner_current.json";
const DEFAULT_REPLAY_OUT_LATEST_PATH: &str = "artifacts/eval_replay_runner_latest.json";
const DEFAULT_REPLAY_MARKDOWN_PATH: &str = "local/workspace/reports/EVAL_REPLAY_RUNNER_CURRENT.md";

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
    let Some(raw) = parse_flag(args, key) else {
        return default;
    };
    match raw.trim() {
        "1" | "true" | "TRUE" | "yes" | "on" => true,
        "0" | "false" | "FALSE" | "no" | "off" => false,
        _ => default,
    }
}

fn read_jsonl(path: &str) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            serde_json::from_str::<Value>(trimmed).ok()
        })
        .collect()
}

fn ensure_parent(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    ensure_parent(path)?;
    let payload = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    fs::write(path, format!("{payload}\n"))
}

fn write_text(path: &str, value: &str) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(path, value)
}

fn print_structured(report: &Value) {
    if let Ok(serialized) = serde_json::to_string(report) {
        let _ = writeln!(io::stdout(), "{serialized}");
    }
}

fn slugify(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "eval-issue".to_string()
    } else {
        trimmed
    }
}

fn string_from_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_str().map(str::trim).filter(|v| !v.is_empty())
}

fn bool_from_path(value: &Value, path: &[&str]) -> bool {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return false;
        };
        cursor = next;
    }
    cursor.as_bool().unwrap_or(false)
}

fn issue_owner(issue_class: &str) -> &'static str {
    match issue_class {
        "wrong_tool_selection" | "auto_tool_selection_claim" | "bad_workflow_selection" => {
            "surface/orchestration/tool-routing"
        }
        "no_response" | "response_loop" | "policy_block_confusion" => {
            "surface/orchestration/workflow-finalization"
        }
        "hallucination" | "tool_output_misdirection" => "surface/orchestration/synthesis",
        "non_actionable_feedback" => "surface/orchestration/eval",
        _ => "surface/orchestration",
    }
}

fn root_cause(issue_class: &str) -> &'static str {
    match issue_class {
        "wrong_tool_selection" => "intent-to-tool routing selected an incompatible tool family",
        "auto_tool_selection_claim" => {
            "response synthesis inferred unsupported tool-routing causes"
        }
        "no_response" => "workflow finalization returned fallback text instead of a direct answer",
        "response_loop" => "fallback recovery repeated boilerplate without loop breaking",
        "policy_block_confusion" => {
            "policy-denied state was not translated into user-actionable remediation"
        }
        "bad_workflow_selection" => "workflow selector chose a diagnostic path for a simple turn",
        "hallucination" => "answer was not grounded against current date or available evidence",
        "tool_output_misdirection" => {
            "tool output synthesis drifted away from the user's actual intent"
        }
        "non_actionable_feedback" => {
            "eval finding omitted owner, acceptance, replay, or concrete evidence"
        }
        _ => "eval finding requires triage",
    }
}

fn is_high_severity(severity: &str) -> bool {
    matches!(severity, "high" | "critical")
}

fn make_issue_draft(row: &Value, fixture_dir: &str) -> Option<Value> {
    if !bool_from_path(row, &["labels", "is_failure"]) {
        return None;
    }
    let id = string_from_path(row, &["id"]).unwrap_or("eval-issue");
    let issue_class = string_from_path(row, &["labels", "issue_class"]).unwrap_or("unknown");
    let severity = string_from_path(row, &["labels", "severity"]).unwrap_or("medium");
    let prompt = string_from_path(row, &["prompt"]).unwrap_or("");
    let assistant_text = string_from_path(row, &["assistant_text"]).unwrap_or("");
    let expected_fix = string_from_path(row, &["labels", "expected_fix"]).unwrap_or("");
    let owner = issue_owner(issue_class);
    let replay_fixture_path = format!("{}/{}.json", fixture_dir.trim_end_matches('/'), slugify(id));
    let replay_command = format!(
        "cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin eval_runtime -- replay --fixture={}",
        replay_fixture_path
    );
    Some(json!({
        "id": id,
        "issue_class": issue_class,
        "severity": severity,
        "title": format!("Eval detected {} in {}", issue_class, id),
        "exact_evidence": {
            "source_event_id": string_from_path(row, &["source_event_id"]).unwrap_or(""),
            "prompt": prompt,
            "assistant_text": assistant_text,
            "tool_trace": row.get("tool_trace").cloned().unwrap_or_else(|| json!([])),
            "runtime_quality": row.get("runtime_quality").cloned().unwrap_or_else(|| json!({})),
            "workflow_quality": row.get("workflow_quality").cloned().unwrap_or_else(|| json!(null))
        },
        "affected_owner_component": owner,
        "suspected_root_cause": root_cause(issue_class),
        "acceptance_criteria": [
            format!("A replay of {} no longer triggers {}", id, issue_class),
            "The fix includes a deterministic regression check or fixture replay evidence",
            "The issue draft remains grounded in exact prompt/assistant/tool evidence"
        ],
        "suggested_test": format!("Add or update an eval fixture covering {}", issue_class),
        "replay_fixture_path": replay_fixture_path,
        "replay_command": replay_command,
        "expected_fix": expected_fix,
        "persistent_high_severity": is_high_severity(severity),
    }))
}

fn required_issue_fields_present(draft: &Value) -> bool {
    let string_fields = [
        "id",
        "issue_class",
        "severity",
        "title",
        "affected_owner_component",
        "suspected_root_cause",
        "suggested_test",
        "replay_fixture_path",
        "replay_command",
        "expected_fix",
    ];
    let strings_ok = string_fields
        .iter()
        .all(|key| string_from_path(draft, &[*key]).is_some());
    let evidence_ok = string_from_path(draft, &["exact_evidence", "prompt"]).is_some()
        && string_from_path(draft, &["exact_evidence", "assistant_text"]).is_some();
    let acceptance_ok = draft
        .get("acceptance_criteria")
        .and_then(|v| v.as_array())
        .map(|rows| rows.len() >= 3 && rows.iter().all(|row| row.as_str().is_some()))
        .unwrap_or(false);
    strings_ok && evidence_ok && acceptance_ok
}

fn write_replay_fixture(draft: &Value) -> bool {
    let Some(path) = string_from_path(draft, &["replay_fixture_path"]) else {
        return false;
    };
    let fixture = json!({
        "type": "eval_replay_fixture",
        "schema_version": 1,
        "issue_id": string_from_path(draft, &["id"]).unwrap_or(""),
        "issue_class": string_from_path(draft, &["issue_class"]).unwrap_or("unknown"),
        "severity": string_from_path(draft, &["severity"]).unwrap_or("medium"),
        "prompt": string_from_path(draft, &["exact_evidence", "prompt"]).unwrap_or(""),
        "assistant_text": string_from_path(draft, &["exact_evidence", "assistant_text"]).unwrap_or(""),
        "expected_fix": string_from_path(draft, &["expected_fix"]).unwrap_or(""),
        "acceptance_criteria": draft.get("acceptance_criteria").cloned().unwrap_or_else(|| json!([])),
    });
    write_json(path, &fixture).is_ok()
}

pub fn run_issue_drafts(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let source_path =
        parse_flag(args, "source").unwrap_or_else(|| DEFAULT_ISSUE_SOURCE_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_ISSUE_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_ISSUE_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_ISSUE_MARKDOWN_PATH.to_string());
    let fixture_dir =
        parse_flag(args, "fixture-dir").unwrap_or_else(|| DEFAULT_REPLAY_FIXTURE_DIR.to_string());
    let rows = read_jsonl(&source_path);
    let drafts: Vec<Value> = rows
        .iter()
        .filter_map(|row| make_issue_draft(row, &fixture_dir))
        .collect();
    let complete_count = drafts
        .iter()
        .filter(|draft| required_issue_fields_present(draft))
        .count();
    let high_severity_count = drafts
        .iter()
        .filter(|draft| {
            draft
                .get("persistent_high_severity")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .count();
    let mut replay_fixture_count = 0_usize;
    for draft in drafts.iter().filter(|draft| {
        draft
            .get("persistent_high_severity")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }) {
        if write_replay_fixture(draft) {
            replay_fixture_count += 1;
        }
    }
    let ok = !drafts.is_empty()
        && complete_count == drafts.len()
        && replay_fixture_count == high_severity_count;
    let report = json!({
        "type": "eval_issue_drafts",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {
                "id": "issue_drafts_present",
                "ok": !drafts.is_empty(),
                "detail": format!("drafts={};source={}", drafts.len(), source_path)
            },
            {
                "id": "issue_draft_required_fields_contract",
                "ok": complete_count == drafts.len(),
                "detail": format!("complete={};drafts={}", complete_count, drafts.len())
            },
            {
                "id": "persistent_high_severity_replay_fixture_contract",
                "ok": replay_fixture_count == high_severity_count,
                "detail": format!("fixtures={};high_severity={}", replay_fixture_count, high_severity_count)
            }
        ],
        "summary": {
            "source_rows": rows.len(),
            "issue_drafts": drafts.len(),
            "complete_issue_drafts": complete_count,
            "persistent_high_severity": high_severity_count,
            "replay_fixtures_generated": replay_fixture_count,
        },
        "issue_drafts": drafts,
        "sources": {
            "source": source_path,
            "fixture_dir": fixture_dir
        }
    });
    let markdown = format!(
        "# Eval Issue Drafts (Current)\n\n- generated_at: {}\n- ok: {}\n- issue_drafts: {}\n- complete_issue_drafts: {}\n- persistent_high_severity: {}\n- replay_fixtures_generated: {}\n",
        report.get("generated_at").and_then(|v| v.as_str()).unwrap_or(""),
        ok,
        drafts.len(),
        complete_count,
        high_severity_count,
        replay_fixture_count
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write issue-draft outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_draft_preserves_runtime_and_workflow_quality_evidence() {
        let row = json!({
            "id": "forgecode-quality-drift",
            "prompt": "assimilate ForgeCode workflow",
            "assistant_text": "fallback loop",
            "labels": {
                "is_failure": true,
                "issue_class": "bad_workflow_selection",
                "severity": "high",
                "expected_fix": "preserve workflow-specific quality adjuncts"
            },
            "runtime_quality": {
                "candidate_count": 4,
                "zero_executable_candidates": false,
                "typed_probe_contract_gap_count": 0
            },
            "workflow_quality": {
                "workflow": "forge_code",
                "signals": {
                    "mcp_alias_route_required": true,
                    "subagent_result_synthesis_required": true
                }
            }
        });

        let draft =
            make_issue_draft(&row, "local/state/ops/eval_replay_fixtures").expect("issue draft");

        assert_eq!(
            draft.pointer("/exact_evidence/runtime_quality/candidate_count"),
            Some(&json!(4))
        );
        assert_eq!(
            draft.pointer(
                "/exact_evidence/workflow_quality/signals/subagent_result_synthesis_required"
            ),
            Some(&json!(true))
        );
    }
}

fn read_fixture_paths(args: &[String], fixture_dir: &str) -> Vec<PathBuf> {
    if let Some(single) = parse_flag(args, "fixture") {
        return vec![PathBuf::from(single)];
    }
    let Ok(entries) = fs::read_dir(fixture_dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|row| row.path()))
        .filter(|path| path.extension().and_then(|v| v.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths
}

pub fn run_replay(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let fixture_dir =
        parse_flag(args, "fixture-dir").unwrap_or_else(|| DEFAULT_REPLAY_FIXTURE_DIR.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_REPLAY_OUT_PATH.to_string());
    let out_latest_path = parse_flag(args, "out-latest")
        .unwrap_or_else(|| DEFAULT_REPLAY_OUT_LATEST_PATH.to_string());
    let markdown_path = parse_flag(args, "out-markdown")
        .unwrap_or_else(|| DEFAULT_REPLAY_MARKDOWN_PATH.to_string());
    let fixture_paths = read_fixture_paths(args, &fixture_dir);
    let mut results = Vec::new();
    let mut passed = 0_usize;
    for path in &fixture_paths {
        let raw = fs::read_to_string(path).unwrap_or_default();
        let fixture: Value = serde_json::from_str(&raw).unwrap_or_else(|_| json!({}));
        let issue_class = string_from_path(&fixture, &["issue_class"]).unwrap_or("unknown");
        let prompt_ok = string_from_path(&fixture, &["prompt"]).is_some();
        let response_ok = string_from_path(&fixture, &["assistant_text"]).is_some();
        let acceptance_ok = fixture
            .get("acceptance_criteria")
            .and_then(|v| v.as_array())
            .map(|rows| !rows.is_empty())
            .unwrap_or(false);
        let class_ok = issue_class != "unknown";
        let ok = prompt_ok && response_ok && acceptance_ok && class_ok;
        if ok {
            passed += 1;
        }
        results.push(json!({
            "path": path.to_string_lossy(),
            "ok": ok,
            "issue_id": string_from_path(&fixture, &["issue_id"]).unwrap_or(""),
            "issue_class": issue_class,
            "checks": {
                "prompt_present": prompt_ok,
                "assistant_text_present": response_ok,
                "acceptance_present": acceptance_ok,
                "issue_class_present": class_ok,
            }
        }));
    }
    let ok = !fixture_paths.is_empty() && passed == fixture_paths.len();
    let report = json!({
        "type": "eval_replay_runner",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {
                "id": "replay_fixture_presence_contract",
                "ok": !fixture_paths.is_empty(),
                "detail": format!("fixtures={};dir={}", fixture_paths.len(), fixture_dir)
            },
            {
                "id": "replay_fixture_execution_contract",
                "ok": passed == fixture_paths.len(),
                "detail": format!("passed={};fixtures={}", passed, fixture_paths.len())
            }
        ],
        "summary": {
            "fixtures": fixture_paths.len(),
            "passed": passed,
            "failed": fixture_paths.len().saturating_sub(passed),
        },
        "results": results,
        "sources": {
            "fixture_dir": fixture_dir
        }
    });
    let markdown = format!(
        "# Eval Replay Runner (Current)\n\n- generated_at: {}\n- ok: {}\n- fixtures: {}\n- passed: {}\n- failed: {}\n",
        report.get("generated_at").and_then(|v| v.as_str()).unwrap_or(""),
        ok,
        fixture_paths.len(),
        passed,
        fixture_paths.len().saturating_sub(passed)
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write replay outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}
