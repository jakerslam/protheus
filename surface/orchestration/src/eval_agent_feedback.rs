use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[path = "eval_agent_feedback_utils.rs"]
mod eval_agent_feedback_utils;
use eval_agent_feedback_utils::*;

const DEFAULT_CONTRACTS_PATH: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_contracts.json";
const DEFAULT_ISSUES_PATH: &str = "core/local/artifacts/eval_issue_drafts_current.json";
const DEFAULT_LEARNING_ISSUES_PATH: &str =
    "core/local/artifacts/eval_learning_loop_issue_candidates_current.json";
const DEFAULT_CHAT_MONITOR_ISSUES_PATH: &str =
    "local/state/ops/eval_agent_chat_monitor/issue_drafts_latest.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_agent_feedback_scope_current.json";
const DEFAULT_LATEST_PATH: &str = "artifacts/eval_agent_feedback_scope_latest.json";
const DEFAULT_AGENT_QUEUE_DIR: &str = "local/state/ops/eval_agent_feedback";

#[derive(Debug, Clone)]
struct EvalIssue {
    id: String,
    source_kind: String,
    title: String,
    severity: String,
    owner_component: String,
    issue_class: String,
    related_agent_id: String,
    expected_fix: String,
    suggested_test: String,
    replay_command: String,
    evidence_summary: String,
    acceptance_criteria: Vec<String>,
}

#[derive(Debug, Clone)]
struct ScopedView {
    agent_id: String,
    scope_agent_ids: BTreeSet<String>,
    visible_issues: Vec<EvalIssue>,
    hidden_unscoped_count: usize,
}

pub fn run_eval_agent_feedback(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let agent_id = parse_flag(args, "agent-id")
        .or_else(|| parse_flag(args, "agent"))
        .map(|raw| normalize_agent_id(&raw))
        .unwrap_or_default();
    let contracts_path =
        parse_flag(args, "contracts").unwrap_or_else(|| DEFAULT_CONTRACTS_PATH.to_string());
    let issues_path = parse_flag(args, "issues").unwrap_or_else(|| DEFAULT_ISSUES_PATH.to_string());
    let learning_path =
        parse_flag(args, "learning").unwrap_or_else(|| DEFAULT_LEARNING_ISSUES_PATH.to_string());
    let chat_monitor_path = parse_flag(args, "chat-monitor-issues")
        .unwrap_or_else(|| DEFAULT_CHAT_MONITOR_ISSUES_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let latest_path = parse_flag(args, "latest").unwrap_or_else(|| DEFAULT_LATEST_PATH.to_string());
    let queue_dir =
        parse_flag(args, "queue-dir").unwrap_or_else(|| DEFAULT_AGENT_QUEUE_DIR.to_string());

    let parent_map = load_parent_map(&contracts_path);
    let issues = load_eval_issues(&issues_path, &learning_path, &chat_monitor_path);
    let views = if agent_id.is_empty() || parse_bool_flag(args, "all", false) {
        build_all_views(&parent_map, &issues)
    } else {
        vec![build_scoped_view(&agent_id, &parent_map, &issues)]
    };
    let report = report_for_views(
        &contracts_path,
        &issues_path,
        &learning_path,
        &chat_monitor_path,
        &views,
    );
    let mut write_ok =
        write_json(&out_path, &report).is_ok() && write_json(&latest_path, &report).is_ok();
    for view in &views {
        if view.agent_id.is_empty() {
            continue;
        }
        let queue_path = Path::new(&queue_dir).join(format!("{}.attention.jsonl", view.agent_id));
        let state_path = Path::new(&queue_dir).join(format!("{}.json", view.agent_id));
        let events = attention_events_for_view(view);
        let state = scoped_state_for_view(view, &events);
        write_ok = write_ok
            && write_json(&state_path, &state).is_ok()
            && write_jsonl(&queue_path, &events).is_ok();
    }
    if !write_ok {
        eprintln!("eval_agent_feedback: failed to write one or more outputs");
        return 2;
    }
    print_json_line(&report);
    if strict && !report.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return 1;
    }
    0
}

fn report_for_views(
    contracts_path: &str,
    issues_path: &str,
    learning_path: &str,
    chat_monitor_path: &str,
    views: &[ScopedView],
) -> Value {
    let trend_items = views
        .iter()
        .flat_map(|view| view.visible_issues.iter())
        .filter(|issue| issue.source_kind == "eval_feedback_trend")
        .count();
    let total_visible = views
        .iter()
        .map(|view| view.visible_issues.len())
        .sum::<usize>();
    let leaked = views
        .iter()
        .flat_map(|view| view.visible_issues.iter().map(move |issue| (view, issue)))
        .filter(|(view, issue)| {
            !issue.related_agent_id.is_empty()
                && !view.scope_agent_ids.contains(&issue.related_agent_id)
        })
        .count();
    let checks = vec![
        json!({
            "id": "eval_agent_feedback_scope_contract",
            "ok": leaked == 0,
            "detail": format!("views={};visible_items={total_visible};scope_leaks={leaked}", views.len())
        }),
        json!({
            "id": "eval_agent_feedback_attention_event_contract",
            "ok": views.iter().all(|view| attention_events_for_view(view).iter().all(attention_event_is_scoped)),
            "detail": "attention_events carry recipient agent source and related agent lineage"
        }),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    json!({
        "type": "eval_agent_feedback_scope",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "summary": {
            "agent_view_count": views.len(),
            "visible_item_count": total_visible,
            "trend_item_count": trend_items,
            "hidden_unscoped_count": views.iter().map(|view| view.hidden_unscoped_count).sum::<usize>(),
            "attention_event_count": views.iter().map(|view| view.visible_issues.len()).sum::<usize>()
        },
        "checks": checks,
        "agent_views": views.iter().map(view_to_json).collect::<Vec<_>>(),
        "sources": {
            "contracts": contracts_path,
            "issue_drafts": issues_path,
            "learning_issues": learning_path,
            "chat_monitor_issues": chat_monitor_path
        }
    })
}

fn scoped_state_for_view(view: &ScopedView, events: &[Value]) -> Value {
    json!({
        "type": "eval_agent_feedback_attention_state",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "agent_id": view.agent_id,
        "scope_agent_ids": view.scope_agent_ids.iter().cloned().collect::<Vec<_>>(),
        "visible_feedback_items": view.visible_issues.iter().map(issue_to_json).collect::<Vec<_>>(),
        "attention_events": events,
        "visibility_rule": "agent_may_see_self_and_descendant_eval_feedback_only",
        "hidden_unscoped_count": view.hidden_unscoped_count
    })
}

fn view_to_json(view: &ScopedView) -> Value {
    let trend_item_count = view
        .visible_issues
        .iter()
        .filter(|issue| issue.source_kind == "eval_feedback_trend")
        .count();
    json!({
        "agent_id": view.agent_id,
        "scope_agent_ids": view.scope_agent_ids.iter().cloned().collect::<Vec<_>>(),
        "visible_item_count": view.visible_issues.len(),
        "trend_item_count": trend_item_count,
        "hidden_unscoped_count": view.hidden_unscoped_count,
        "items": view.visible_issues.iter().map(issue_to_json).collect::<Vec<_>>()
    })
}

fn issue_to_json(issue: &EvalIssue) -> Value {
    json!({
        "id": issue.id,
        "source_kind": issue.source_kind,
        "title": issue.title,
        "severity": issue.severity,
        "owner_component": issue.owner_component,
        "issue_class": issue.issue_class,
        "related_agent_id": issue.related_agent_id,
        "expected_fix": issue.expected_fix,
        "suggested_test": issue.suggested_test,
        "replay_command": issue.replay_command,
        "evidence_summary": issue.evidence_summary,
        "acceptance_criteria": issue.acceptance_criteria
    })
}

fn build_all_views(parent_map: &BTreeMap<String, String>, issues: &[EvalIssue]) -> Vec<ScopedView> {
    let mut agents = parent_map.keys().cloned().collect::<BTreeSet<_>>();
    for issue in issues {
        if !issue.related_agent_id.is_empty() {
            agents.insert(issue.related_agent_id.clone());
            for ancestor in ancestors_of(parent_map, &issue.related_agent_id) {
                agents.insert(ancestor);
            }
        }
    }
    agents
        .iter()
        .map(|agent| build_scoped_view(agent, parent_map, issues))
        .filter(|view| !view.visible_issues.is_empty())
        .collect()
}

fn build_scoped_view(
    agent_id: &str,
    parent_map: &BTreeMap<String, String>,
    issues: &[EvalIssue],
) -> ScopedView {
    let mut scope = descendants_of(parent_map, agent_id);
    scope.insert(agent_id.to_string());
    let mut hidden_unscoped_count = 0usize;
    let visible_issues = issues
        .iter()
        .filter_map(|issue| {
            if issue.related_agent_id.is_empty() {
                hidden_unscoped_count += 1;
                None
            } else if scope.contains(&issue.related_agent_id) {
                Some(issue.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    ScopedView {
        agent_id: agent_id.to_string(),
        scope_agent_ids: scope,
        visible_issues,
        hidden_unscoped_count,
    }
}

fn attention_events_for_view(view: &ScopedView) -> Vec<Value> {
    view.visible_issues
        .iter()
        .map(|issue| {
            let summary = format!(
                "Eval feedback for {}: {}",
                if issue.related_agent_id == view.agent_id {
                    "this agent"
                } else {
                    issue.related_agent_id.as_str()
                },
                issue.title
            );
            json!({
                "ts": now_iso_like(),
                "source": format!("agent:{}", view.agent_id),
                "source_type": "eval_issue_feedback",
                "severity": issue.severity,
                "summary": clean_text(&summary, 280),
                "attention_key": format!("eval_feedback:{}:{}", view.agent_id, stable_hash_hex(&issue.id)),
                "raw_event": {
                    "agent_id": view.agent_id,
                    "related_agent_id": issue.related_agent_id,
                    "issue_id": issue.id,
                    "issue_class": issue.issue_class,
                    "source_kind": issue.source_kind,
                    "owner_component": issue.owner_component,
                    "expected_fix": issue.expected_fix,
                    "suggested_test": issue.suggested_test,
                    "replay_command": issue.replay_command,
                    "terms": feedback_terms(issue)
                }
            })
        })
        .collect()
}

fn attention_event_is_scoped(row: &Value) -> bool {
    let source = str_at(row, &["source"]).unwrap_or("");
    let agent_id = str_at(row, &["raw_event", "agent_id"]).unwrap_or("");
    let related_agent_id = str_at(row, &["raw_event", "related_agent_id"]).unwrap_or("");
    source == format!("agent:{agent_id}")
        && str_at(row, &["source_type"]) == Some("eval_issue_feedback")
        && !agent_id.is_empty()
        && !related_agent_id.is_empty()
}

fn load_eval_issues(
    issue_path: &str,
    learning_path: &str,
    chat_monitor_path: &str,
) -> Vec<EvalIssue> {
    let mut out = Vec::new();
    let issues = read_json(issue_path);
    for row in array_at(&issues, &["issue_drafts"]) {
        out.push(issue_from_eval_draft(&row));
    }
    let learning = read_json(learning_path);
    let learning_start = out.len();
    for row in array_at(&learning, &["candidates"]) {
        out.push(issue_from_learning_candidate(&row));
    }
    let learning_issues = out[learning_start..].to_vec();
    out.extend(learning_trend_issues(&learning_issues));
    let chat_monitor = read_json(chat_monitor_path);
    for row in array_at(&chat_monitor, &["issue_drafts"]) {
        out.push(issue_from_chat_monitor_draft(&row));
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn issue_from_eval_draft(row: &Value) -> EvalIssue {
    EvalIssue {
        id: required_str(row, &["id"], "eval_issue"),
        source_kind: "eval_issue_draft".to_string(),
        title: required_str(row, &["title"], "Eval issue"),
        severity: normalized_severity(str_at(row, &["severity"]).unwrap_or("warn")),
        owner_component: required_str(row, &["affected_owner_component"], "unknown"),
        issue_class: required_str(row, &["issue_class"], "unknown"),
        related_agent_id: related_agent_id(row),
        expected_fix: required_str(row, &["expected_fix"], ""),
        suggested_test: required_str(row, &["suggested_test"], ""),
        replay_command: required_str(row, &["replay_command"], ""),
        evidence_summary: evidence_summary(row),
        acceptance_criteria: string_array_at(row, &["acceptance_criteria"]),
    }
}

fn issue_from_chat_monitor_draft(row: &Value) -> EvalIssue {
    let issue_id = required_str(row, &["id"], "eval_chat_monitor_issue");
    EvalIssue {
        id: issue_id.clone(),
        source_kind: "eval_agent_chat_monitor_issue".to_string(),
        title: required_str(row, &["title"], "Live eval chat-monitor issue"),
        severity: normalized_severity(str_at(row, &["severity"]).unwrap_or("warn")),
        owner_component: required_str(row, &["owner_component"], "control_plane.eval_chat_monitor"),
        issue_class: issue_id
            .strip_suffix("_detected")
            .unwrap_or(&issue_id)
            .to_string(),
        related_agent_id: related_agent_id(row),
        expected_fix: chat_monitor_next_action(row),
        suggested_test: chat_monitor_suggested_test(row),
        replay_command: required_str(row, &["replay_command"], ""),
        evidence_summary: evidence_summary(row),
        acceptance_criteria: string_array_at(row, &["acceptance_criteria"]),
    }
}

fn issue_from_learning_candidate(row: &Value) -> EvalIssue {
    EvalIssue {
        id: required_str(row, &["id"], "eval_learning_issue"),
        source_kind: "eval_learning_loop_candidate".to_string(),
        title: required_str(row, &["symptom"], "Eval learning-loop issue"),
        severity: normalized_severity(str_at(row, &["severity"]).unwrap_or("warn")),
        owner_component: required_str(row, &["owner_component"], "unknown"),
        issue_class: string_array_at(row, &["root_cause_basis"]).join(","),
        related_agent_id: related_agent_id(row),
        expected_fix: required_str(row, &["expected_behavior"], ""),
        suggested_test: required_str(row, &["suggested_test"], ""),
        replay_command: required_str(row, &["repro_path"], ""),
        evidence_summary: evidence_summary(row),
        acceptance_criteria: string_array_at(row, &["acceptance_criteria"]),
    }
}

fn learning_trend_issues(issues: &[EvalIssue]) -> Vec<EvalIssue> {
    let mut grouped: BTreeMap<(String, String), Vec<&EvalIssue>> = BTreeMap::new();
    for issue in issues {
        if issue.source_kind != "eval_learning_loop_candidate" || issue.related_agent_id.is_empty()
        {
            continue;
        }
        for signal in learning_issue_signals(issue) {
            grouped
                .entry((issue.related_agent_id.clone(), signal))
                .or_default()
                .push(issue);
        }
    }

    let mut out = Vec::new();
    for ((agent_id, signal), rows) in grouped {
        if rows.len() < 3 {
            continue;
        }
        out.push(trend_issue_for_group(&agent_id, &signal, &rows));
    }
    out
}

fn learning_issue_signals(issue: &EvalIssue) -> Vec<String> {
    let mut out = BTreeSet::new();
    for raw in issue.issue_class.split(',') {
        let cleaned = raw.trim();
        if !cleaned.is_empty() {
            out.insert(cleaned.to_string());
        }
    }
    if out.is_empty() && issue.source_kind == "eval_learning_loop_candidate" {
        out.insert("unknown".to_string());
    }
    out.into_iter().collect()
}

fn trend_issue_for_group(agent_id: &str, signal: &str, rows: &[&EvalIssue]) -> EvalIssue {
    let mut case_terms = BTreeSet::new();
    let mut high_count = 0usize;
    let mut owner_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut sample_ids = Vec::new();
    for issue in rows {
        if issue.severity == "high" || issue.severity == "critical" {
            high_count += 1;
        }
        *owner_counts
            .entry(issue.owner_component.clone())
            .or_insert(0) += 1;
        if sample_ids.len() < 3 {
            sample_ids.push(issue.id.clone());
        }
        for token in issue.id.split([':', '/']) {
            if token.ends_with("_request")
                || token.ends_with("_recovery")
                || token.ends_with("_answer")
                || token.ends_with("_exit")
            {
                case_terms.insert(token.to_string());
            }
        }
    }
    let owner_component = owner_counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(owner, _)| owner)
        .unwrap_or_else(|| "surface/orchestration".to_string());
    let case_summary = if case_terms.is_empty() {
        "multiple cases".to_string()
    } else {
        case_terms.into_iter().collect::<Vec<_>>().join(", ")
    };
    let severity = if rows.len() >= 10 && high_count >= 3 {
        "critical"
    } else if high_count > 0 {
        "high"
    } else {
        "warn"
    };
    let sample_summary = sample_ids.join("; ");
    EvalIssue {
        id: format!(
            "eval-feedback-trend-{}-{}-{}",
            normalize_agent_id(agent_id),
            signal,
            stable_hash_hex(&sample_summary)
        ),
        source_kind: "eval_feedback_trend".to_string(),
        title: format!("Repeated eval pattern: {signal} recurred {} times.", rows.len()),
        severity: severity.to_string(),
        owner_component,
        issue_class: format!("repeated_{signal}"),
        related_agent_id: agent_id.to_string(),
        expected_fix: format!(
            "Treat this as a repeated pattern before individual trace triage: {signal} recurred across {case_summary}."
        ),
        suggested_test: format!(
            "Run the synthetic-user chat harness twice and verify no repeated {signal} trend is emitted."
        ),
        replay_command: rows
            .first()
            .map(|issue| issue.replay_command.clone())
            .unwrap_or_default(),
        evidence_summary: format!(
            "{} scoped learning-loop candidates share `{}`; sample issue ids: {}",
            rows.len(),
            signal,
            sample_summary
        ),
        acceptance_criteria: vec![
            format!("Repeated `{signal}` candidates collapse into one trend item for agent attention."),
            format!("A replay no longer emits three or more `{signal}` learning-loop candidates for the same agent."),
            "The detailed candidate evidence remains available for audit.".to_string(),
        ],
    }
}

fn related_agent_id(row: &Value) -> String {
    for path in [
        ["agent_id"].as_slice(),
        ["source_agent_id"].as_slice(),
        ["target_agent_id"].as_slice(),
        ["exact_evidence", "agent_id"].as_slice(),
        ["exact_evidence", "source_agent_id"].as_slice(),
        ["evidence", "agent_id"].as_slice(),
        ["raw_event", "agent_id"].as_slice(),
    ] {
        let candidate = normalize_agent_id(str_at(row, path).unwrap_or(""));
        if !candidate.is_empty() {
            return candidate;
        }
    }
    for path in [
        ["exact_evidence", "source_event_id"].as_slice(),
        ["evidence", "source_event_id"].as_slice(),
        ["turn_id"].as_slice(),
        ["source_event_id"].as_slice(),
    ] {
        if let Some(agent) = agent_id_from_source_event(str_at(row, path).unwrap_or("")) {
            return agent;
        }
    }
    for evidence in array_at(row, &["evidence"]) {
        for path in [
            ["agent_id"].as_slice(),
            ["turn_id"].as_slice(),
            ["source_event_id"].as_slice(),
        ] {
            if let Some(agent) = agent_id_from_source_event(str_at(&evidence, path).unwrap_or("")) {
                return agent;
            }
        }
    }
    String::new()
}

fn feedback_terms(issue: &EvalIssue) -> Vec<String> {
    let text = format!(
        "{} {} {} {} {}",
        issue.title,
        issue.issue_class,
        issue.owner_component,
        issue.expected_fix,
        issue.suggested_test
    )
    .to_ascii_lowercase();
    let mut out = BTreeSet::new();
    for token in text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-') {
        let cleaned = token.trim();
        if cleaned.len() >= 4 {
            out.insert(cleaned.to_string());
        }
        if out.len() >= 16 {
            break;
        }
    }
    out.into_iter().collect()
}

fn load_parent_map(path: &str) -> BTreeMap<String, String> {
    let contracts = read_json(path);
    let mut out = BTreeMap::new();
    let Some(map) = contracts.get("contracts").and_then(Value::as_object) else {
        return out;
    };
    for (id, row) in map {
        let agent_id = normalize_agent_id(
            row.get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or(id.as_str()),
        );
        let parent = normalize_agent_id(
            row.get("parent_agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
        if !agent_id.is_empty() {
            out.entry(agent_id.clone()).or_default();
            if !parent.is_empty() && parent != agent_id {
                out.insert(agent_id, parent);
            }
        }
    }
    out
}

fn descendants_of(parent_map: &BTreeMap<String, String>, agent_id: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut changed = true;
    while changed {
        changed = false;
        for (child, parent) in parent_map {
            if parent == agent_id || out.contains(parent) {
                changed |= out.insert(child.clone());
            }
        }
    }
    out
}

fn ancestors_of(parent_map: &BTreeMap<String, String>, agent_id: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut cursor = agent_id;
    while let Some(parent) = parent_map.get(cursor) {
        if parent.is_empty() || !out.insert(parent.clone()) {
            break;
        }
        cursor = parent;
    }
    out
}

#[cfg(test)]
#[path = "eval_agent_feedback_tests.rs"]
mod eval_agent_feedback_tests;
