fn maybe_tooling_failure_fallback_rewrites_context_guard_diagnosis() {
    let fallback = maybe_tooling_failure_fallback(
        "why is web search failing lately",
        "Context overflow: estimated context size exceeds safe threshold during tool loop.",
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("fit safely in context"));
    assert!(lowered.contains("partial result"));
}

#[test]
