use super::*;

// SRS: V12-EVAL-AGENT-FEEDBACK-001
fn issue(agent: &str, id: &str) -> EvalIssue {
    EvalIssue {
        id: id.to_string(),
        source_kind: "eval_issue_draft".to_string(),
        title: "Wrong tool selected".to_string(),
        severity: "warn".to_string(),
        owner_component: "surface/orchestration/tool-routing".to_string(),
        issue_class: "wrong_tool_selection".to_string(),
        related_agent_id: agent.to_string(),
        expected_fix: "Use workspace route".to_string(),
        suggested_test: "Replay fixture".to_string(),
        replay_command: "cargo run replay".to_string(),
        evidence_summary: "tool route failed".to_string(),
        acceptance_criteria: vec!["No wrong tool route".to_string()],
    }
}

#[test]
fn parent_sees_child_eval_feedback_but_sibling_does_not() {
    let parent_map = BTreeMap::from([
        ("child-a".to_string(), "parent".to_string()),
        ("sibling".to_string(), "other-parent".to_string()),
    ]);
    let issues = vec![issue("child-a", "eval-child-a")];

    let parent = build_scoped_view("parent", &parent_map, &issues);
    let sibling = build_scoped_view("sibling", &parent_map, &issues);

    assert_eq!(parent.visible_issues.len(), 1);
    assert_eq!(parent.visible_issues[0].related_agent_id, "child-a");
    assert!(sibling.visible_issues.is_empty());
}

#[test]
fn eval_feedback_attention_event_is_recipient_scoped() {
    let view = ScopedView {
        agent_id: "parent".to_string(),
        scope_agent_ids: BTreeSet::from(["parent".to_string(), "child-a".to_string()]),
        visible_issues: vec![issue("child-a", "eval-child-a")],
        hidden_unscoped_count: 0,
    };
    let events = attention_events_for_view(&view);
    assert_eq!(events.len(), 1);
    assert!(attention_event_is_scoped(&events[0]));
    assert_eq!(str_at(&events[0], &["source"]), Some("agent:parent"));
    assert_eq!(
        str_at(&events[0], &["raw_event", "related_agent_id"]),
        Some("child-a")
    );
}

#[test]
fn source_event_id_extracts_agent_id() {
    assert_eq!(
        agent_id_from_source_event("agent:agent-5bc62b0875a9:passive_memory:abc").as_deref(),
        Some("agent-5bc62b0875a9")
    );
    assert_eq!(
        agent_id_from_source_event("2026-04-24T14:23:01.449Z:agent-5bc62b0875a9").as_deref(),
        Some("agent-5bc62b0875a9")
    );
}

#[test]
fn chat_monitor_no_response_issue_routes_to_evidence_agent() {
    let row = json!({
        "id": "no_response_detected",
        "severity": "high",
        "owner_component": "control_plane.finalization",
        "title": "[HIGH][no_response_detected] Detected turns where assistant returned fallback-only text without substantive response.",
        "body": "Next action:\nForce degraded one-shot answer synthesis when finalization fails and block no-answer fallback templates.\n\nAcceptance criteria:\n- User-visible responses contain substantive answer content.",
        "acceptance_criteria": [
            "User-visible responses contain substantive answer content for answerable prompts."
        ],
        "evidence": [
            {
                "turn_id": "2026-04-24T14:23:01.449Z:agent-5bc62b0875a9",
                "ts": "2026-04-24T14:23:01.449Z",
                "snippet": ""
            }
        ]
    });
    let issue = issue_from_chat_monitor_draft(&row);
    assert_eq!(issue.source_kind, "eval_agent_chat_monitor_issue");
    assert_eq!(issue.issue_class, "no_response");
    assert_eq!(issue.related_agent_id, "agent-5bc62b0875a9");
    assert!(issue.expected_fix.contains("Force degraded one-shot"));

    let view = build_scoped_view("agent-5bc62b0875a9", &BTreeMap::new(), &[issue]);
    assert_eq!(view.visible_issues.len(), 1);
    let events = attention_events_for_view(&view);
    assert_eq!(
        str_at(&events[0], &["raw_event", "source_kind"]),
        Some("eval_agent_chat_monitor_issue")
    );
}

#[test]
fn repeated_learning_candidates_emit_agent_trend_feedback() {
    // SRS: V12-EVAL-AGENT-FEEDBACK-TRENDS-001
    let mut issues = Vec::new();
    for idx in 0..4 {
        let mut row = issue("agent-5bc62b0875a9", &format!("eval-learning-case:{idx}"));
        row.source_kind = "eval_learning_loop_candidate".to_string();
        row.issue_class = "no_response,workflow_visibility".to_string();
        row.severity = "high".to_string();
        row.owner_component = "surface/orchestration".to_string();
        row.replay_command = "cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin eval_runtime -- learning-loop-issues --strict=1".to_string();
        issues.push(row);
    }

    let trends = learning_trend_issues(&issues);
    assert!(trends
        .iter()
        .any(|issue| issue.issue_class == "repeated_no_response"));
    assert!(trends
        .iter()
        .any(|issue| issue.issue_class == "repeated_workflow_visibility"));

    let mut all = issues;
    all.extend(trends);
    let view = build_scoped_view("agent-5bc62b0875a9", &BTreeMap::new(), &all);
    let report = report_for_views(
        "contracts.json",
        "issues.json",
        "learning.json",
        "chat.json",
        &[view],
    );
    assert_eq!(
        report
            .pointer("/summary/trend_item_count")
            .and_then(Value::as_u64),
        Some(2)
    );
}
