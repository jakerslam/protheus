fn final_answer_contract_reports_claim_sources_from_tool_receipts() {
    let (finalized, report, _outcome) = enforce_user_facing_finalization_contract(
        "what happened with the web tooling",
        "The web run returned low-signal snippets in this turn.".to_string(),
        &[json!({
            "name": "batch_query",
            "status": "no_results",
            "is_error": true,
            "result": "low-signal snippets",
            "tool_attempt_receipt": {"receipt_hash": "abc123"}
        })],
    );
    assert!(!finalized.trim().is_empty());
    let claim_sources = report
        .pointer("/final_answer_contract/claim_sources")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(
        claim_sources
            .iter()
            .any(|row| row.contains("tool_receipt:abc123")),
        "{claim_sources:?}"
    );
    assert_eq!(
        report
            .pointer("/final_answer_contract/no_unsourced_claims")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
