fn tooling_failure_fallback_triggers_for_diagnostic_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "so the tooling isnt working at all?",
        "I couldn't extract usable findings for \"current technology news\" yet.",
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("partially working"));
    assert!(lowered.contains("batch_query"));
    assert!(lowered.contains("doctor --json"));
}

#[test]
