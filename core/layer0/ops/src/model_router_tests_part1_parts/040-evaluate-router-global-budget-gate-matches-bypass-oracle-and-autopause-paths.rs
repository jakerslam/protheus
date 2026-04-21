
#[test]
fn evaluate_router_global_budget_gate_matches_bypass_oracle_and_autopause_paths() {
    let bypass = evaluate_router_global_budget_gate(RouterGlobalBudgetGateInput {
        request_tokens_est: Some(800.0),
        dry_run: Some(&json!(false)),
        execution_intent: Some(&json!(false)),
        enforce_execution_only: true,
        nonexec_max_tokens: 900,
        autopause: Some(&json!({"active": true, "source": "operator", "reason": "manual"})),
        oracle: None,
        guard: None,
    });
    assert!(bypass.enabled);
    assert!(!bypass.blocked);
    assert!(!bypass.deferred);
    assert!(bypass.bypassed);
    assert_eq!(
        bypass.reason.as_deref(),
        Some("budget_guard_nonexecute_bypass")
    );
    assert!(bypass.autopause_active);

    let oracle_block = evaluate_router_global_budget_gate(RouterGlobalBudgetGateInput {
        request_tokens_est: Some(1200.0),
        dry_run: Some(&json!(false)),
        execution_intent: Some(&json!(true)),
        enforce_execution_only: true,
        nonexec_max_tokens: 900,
        autopause: Some(&json!({"active": false})),
        oracle: Some(&json!({"available": true, "pressure": "hard"})),
        guard: None,
    });
    assert!(oracle_block.enabled);
    assert!(oracle_block.blocked);
    assert!(!oracle_block.deferred);
    assert_eq!(
        oracle_block.reason.as_deref(),
        Some("budget_oracle_runway_critical")
    );
    assert_eq!(
        oracle_block.oracle.as_ref().map(|v| v["pressure"].clone()),
        Some(json!("hard"))
    );

    let recovered_autopause = evaluate_router_global_budget_gate(RouterGlobalBudgetGateInput {
        request_tokens_est: Some(1000.0),
        dry_run: Some(&json!(false)),
        execution_intent: Some(&json!(true)),
        enforce_execution_only: true,
        nonexec_max_tokens: 900,
        autopause: Some(
            &json!({"active": true, "source": "model_router", "reason": "prior_hard_stop", "until": "2026-03-05T10:00:00.000Z"}),
        ),
        oracle: Some(&json!({"available": false})),
        guard: Some(&json!({"hard_stop": false, "pressure": "none"})),
    });
    assert!(recovered_autopause.enabled);
    assert!(!recovered_autopause.blocked);
    assert!(!recovered_autopause.deferred);
    assert!(!recovered_autopause.autopause_active);
    assert!(recovered_autopause.reason.is_none());
}
