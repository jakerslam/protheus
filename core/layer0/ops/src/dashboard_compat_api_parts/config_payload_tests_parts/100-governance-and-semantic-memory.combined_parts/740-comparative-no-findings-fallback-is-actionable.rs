fn comparative_no_findings_fallback_is_diagnostics_only() {
    let fallback = comparative_no_findings_fallback("rank infring among peers");
    assert!(fallback.is_empty());
}

#[test]
