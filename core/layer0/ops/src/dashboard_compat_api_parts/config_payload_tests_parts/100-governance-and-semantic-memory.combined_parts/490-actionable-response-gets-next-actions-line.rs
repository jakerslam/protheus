fn actionable_response_gets_next_actions_line() {
    let out = append_next_actions_line_if_actionable(
        "what should we do next to improve web tooling?",
        "The latest run showed low-signal results from web retrieval.",
        &[],
    );
    assert!(out.contains("Next actions:"), "{out}");
    let non_actionable = append_next_actions_line_if_actionable(
        "thanks",
        "Glad to help.",
        &[],
    );
    assert!(!non_actionable.contains("Next actions:"), "{non_actionable}");
}

#[test]
