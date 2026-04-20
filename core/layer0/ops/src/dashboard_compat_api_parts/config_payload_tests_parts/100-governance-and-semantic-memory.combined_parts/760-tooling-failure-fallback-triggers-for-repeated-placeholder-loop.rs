fn tooling_failure_fallback_triggers_for_repeated_placeholder_loop() {
    let repeated = "I couldn't extract usable findings for \"current technology news\" yet.";
    let fallback = maybe_tooling_failure_fallback("?", repeated, repeated).expect("fallback");
    assert!(fallback.to_ascii_lowercase().contains("parse miss"));
}

#[test]
