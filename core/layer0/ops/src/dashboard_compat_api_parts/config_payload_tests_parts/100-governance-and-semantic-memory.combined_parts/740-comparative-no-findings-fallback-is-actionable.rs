fn comparative_no_findings_fallback_is_actionable() {
    let fallback = comparative_no_findings_fallback("rank infring among peers");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("infring"));
    assert!(lowered.contains("strongest"));
    assert!(lowered.contains("batch_query"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
