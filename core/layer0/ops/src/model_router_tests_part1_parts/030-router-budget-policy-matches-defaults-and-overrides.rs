
#[test]
fn router_budget_policy_matches_defaults_and_overrides() {
    let defaults = router_budget_policy(&json!({}), Path::new("/repo"), ROUTER_BUDGET_DIR_DEFAULT);
    assert!(defaults.enabled);
    assert!(defaults.allow_strategy_override);
    assert!((defaults.soft_ratio - 0.75).abs() < 1e-9);
    assert!((defaults.hard_ratio - 0.92).abs() < 1e-9);
    assert!(defaults.enforce_hard_cap);
    assert!(defaults.escalate_on_no_local_fallback);
    assert!((defaults.cloud_penalty_soft - 4.0).abs() < 1e-9);
    assert!((defaults.cloud_penalty_hard - 10.0).abs() < 1e-9);
    assert!(defaults
        .state_dir
        .ends_with("local/state/autonomy/daily_budget"));
    assert_eq!(
        defaults
            .class_token_multipliers
            .get("cheap_local")
            .and_then(Value::as_f64),
        Some(0.42)
    );
    assert_eq!(
        defaults
            .class_token_multipliers
            .get("default")
            .and_then(Value::as_f64),
        Some(1.0)
    );

    let cfg = json!({
        "routing": {
            "router_budget_policy": {
                "enabled": "off",
                "state_dir": "tmp/router_budget",
                "allow_strategy_override": "0",
                "soft_ratio": 1.5,
                "hard_ratio": 0.1,
                "enforce_hard_cap": "false",
                "escalate_on_no_local_fallback": "no",
                "cloud_penalty_soft": 99,
                "cloud_penalty_hard": -10,
                "cheap_local_bonus_soft": 77,
                "cheap_local_bonus_hard": 88,
                "model_token_multipliers": {
                    "openai/gpt-4.1": "1.8"
                },
                "class_token_multipliers": {
                    "cloud": 2.5,
                    "local": 0
                }
            }
        }
    });
    let overridden = router_budget_policy(&cfg, Path::new("/repo"), ROUTER_BUDGET_DIR_DEFAULT);
    assert!(!overridden.enabled);
    assert!(!overridden.allow_strategy_override);
    assert!((overridden.soft_ratio - 0.98).abs() < 1e-9);
    assert!((overridden.hard_ratio - 0.3).abs() < 1e-9);
    assert!(!overridden.enforce_hard_cap);
    assert!(!overridden.escalate_on_no_local_fallback);
    assert!((overridden.cloud_penalty_soft - 40.0).abs() < 1e-9);
    assert!((overridden.cloud_penalty_hard - 0.0).abs() < 1e-9);
    assert!((overridden.cheap_local_bonus_soft - 40.0).abs() < 1e-9);
    assert!((overridden.cheap_local_bonus_hard - 60.0).abs() < 1e-9);
    assert!(overridden.state_dir.ends_with("tmp/router_budget"));
    assert_eq!(
        overridden
            .model_token_multipliers
            .get("openai/gpt-4.1")
            .and_then(Value::as_str),
        Some("1.8")
    );
    assert_eq!(
        overridden
            .class_token_multipliers
            .get("cloud")
            .and_then(Value::as_f64),
        Some(2.5)
    );
    assert_eq!(
        overridden
            .class_token_multipliers
            .get("local")
            .and_then(Value::as_i64),
        Some(0)
    );
}

#[test]
fn budget_date_str_prefers_valid_override() {
    assert_eq!(
        budget_date_str("2026-03-01", "2020-01-01T00:00:00.000Z"),
        "2026-03-01"
    );
    assert_eq!(
        budget_date_str("bad-date", "2026-03-05T12:34:56.000Z"),
        "2026-03-05"
    );
    assert_eq!(budget_date_str("", "short"), "short");
}

#[test]
fn router_burn_oracle_signal_normalizes_pressure_and_limits_reason_codes() {
    let signal = router_burn_oracle_signal(
        Some(&json!({
            "available": true,
            "pressure": "CRITICAL",
            "projected_runway_days": "1.5",
            "projected_days_to_reset": 3,
            "reason_codes": ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"],
            "latest_path_rel": "local/state/ops/dynamic_burn_budget_oracle/latest.json"
        })),
        ROUTER_BURN_ORACLE_LATEST_PATH_REL_DEFAULT,
    );
    assert_eq!(signal["available"], true);
    assert_eq!(signal["pressure"], "hard");
    assert_eq!(signal["pressure_rank"], 4);
    assert_eq!(signal["projected_runway_days"], 1.5);
    assert_eq!(signal["projected_days_to_reset"], 3.0);
    assert_eq!(
        signal["source_path"],
        "local/state/ops/dynamic_burn_budget_oracle/latest.json"
    );
    assert_eq!(
        signal["reason_codes"].as_array().map(|rows| rows.len()),
        Some(10)
    );

    let fallback = router_burn_oracle_signal(None, "local/state/default/latest.json");
    assert_eq!(fallback["available"], false);
    assert_eq!(fallback["pressure"], "none");
    assert_eq!(fallback["pressure_rank"], 0);
    assert_eq!(fallback["source_path"], "local/state/default/latest.json");
    assert_eq!(
        fallback["reason_codes"].as_array().map(|rows| rows.len()),
        Some(0)
    );
}

#[test]
fn router_budget_state_matches_disabled_unavailable_and_oracle_override_paths() {
    let disabled = router_budget_state(RouterBudgetStateInput {
        cfg: &json!({
            "routing": {
                "router_budget_policy": {
                    "enabled": false
                }
            }
        }),
        repo_root: Path::new("/repo"),
        default_state_dir: ROUTER_BUDGET_DIR_DEFAULT,
        today_override: "2026-03-05",
        now_iso: "2026-03-05T00:00:00.000Z",
        budget_state: None,
        oracle_signal: None,
        default_oracle_source_path: ROUTER_BURN_ORACLE_LATEST_PATH_REL_DEFAULT,
    });
    assert_eq!(disabled["enabled"], false);
    assert_eq!(disabled["available"], false);
    assert_eq!(disabled["path"], Value::Null);

    let unavailable = router_budget_state(RouterBudgetStateInput {
        cfg: &json!({}),
        repo_root: Path::new("/repo"),
        default_state_dir: ROUTER_BUDGET_DIR_DEFAULT,
        today_override: "2026-03-06",
        now_iso: "2026-03-05T00:00:00.000Z",
        budget_state: None,
        oracle_signal: Some(&json!({
            "available": true,
            "pressure": "soft"
        })),
        default_oracle_source_path: ROUTER_BURN_ORACLE_LATEST_PATH_REL_DEFAULT,
    });
    assert_eq!(unavailable["enabled"], true);
    assert_eq!(unavailable["available"], false);
    assert_eq!(
        unavailable["path"],
        "/repo/local/state/autonomy/daily_budget/2026-03-06.json"
    );
    assert_eq!(unavailable["pressure"], "none");
    assert_eq!(unavailable["oracle"]["pressure"], "soft");

    let overridden = router_budget_state(RouterBudgetStateInput {
        cfg: &json!({}),
        repo_root: Path::new("/repo"),
        default_state_dir: ROUTER_BUDGET_DIR_DEFAULT,
        today_override: "2026-03-07",
        now_iso: "2026-03-05T00:00:00.000Z",
        budget_state: Some(&json!({
            "available": true,
            "path": "/tmp/router-budget.json",
            "token_cap": 1000,
            "used_est": 760,
            "strategy_id": "strat-1"
        })),
        oracle_signal: Some(&json!({
            "available": true,
            "pressure": "hard"
        })),
        default_oracle_source_path: ROUTER_BURN_ORACLE_LATEST_PATH_REL_DEFAULT,
    });
    assert_eq!(overridden["available"], true);
    assert_eq!(overridden["path"], "/tmp/router-budget.json");
    assert_eq!(overridden["ratio"], 0.76);
    assert_eq!(overridden["token_cap"], 1000.0);
    assert_eq!(overridden["used_est"], 760.0);
    assert_eq!(overridden["pressure"], "hard");
    assert_eq!(overridden["strategy_id"], "strat-1");
}
