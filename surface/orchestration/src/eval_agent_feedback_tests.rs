use super::*;

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
}
