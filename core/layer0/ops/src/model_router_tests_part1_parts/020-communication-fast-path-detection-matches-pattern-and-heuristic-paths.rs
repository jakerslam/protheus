
#[test]
fn communication_fast_path_detection_matches_pattern_and_heuristic_paths() {
    let pattern_cfg = json!({
        "routing": {
            "communication_fast_path": {
                "match_mode": "patterns",
                "patterns": ["status"],
                "disallow_regexes": [],
                "slot": "grunt",
                "prefer_model": "ollama/smallthinker",
                "fallback_slot": "fallback",
                "skip_outcome_scan": true
            }
        }
    });
    let by_pattern = detect_communication_fast_path(
        &pattern_cfg,
        "low",
        "medium",
        "status",
        "",
        "normal",
        false,
    );
    assert!(by_pattern.matched);
    assert_eq!(by_pattern.reason, "communication_fast_path_pattern");
    assert_eq!(by_pattern.matched_pattern.as_deref(), Some("status"));
    assert_eq!(by_pattern.text.as_deref(), Some("status"));
    assert_eq!(by_pattern.slot.as_deref(), Some("grunt"));
    assert_eq!(
        by_pattern.prefer_model.as_deref(),
        Some("ollama/smallthinker")
    );
    assert_eq!(by_pattern.fallback_slot.as_deref(), Some("fallback"));
    assert_eq!(by_pattern.skip_outcome_scan, Some(true));

    let no_pattern = detect_communication_fast_path(
        &pattern_cfg,
        "low",
        "medium",
        "hello there",
        "",
        "normal",
        false,
    );
    assert!(!no_pattern.matched);
    assert_eq!(no_pattern.reason, "no_pattern_match");

    let heuristic = detect_communication_fast_path(
        &json!({}),
        "medium",
        "high",
        "how are you",
        "",
        "normal",
        true,
    );
    assert!(heuristic.matched);
    assert_eq!(heuristic.reason, "communication_fast_path_heuristic");
    assert_eq!(heuristic.text.as_deref(), Some("how are you"));
    assert_eq!(heuristic.slot.as_deref(), Some("grunt"));
    assert_eq!(heuristic.skip_outcome_scan, Some(true));
}

#[test]
fn fallback_classification_policy_matches_defaults_and_bounds() {
    let defaults = fallback_classification_policy(&json!({}));
    assert!(defaults.enabled);
    assert!(defaults.only_when_medium_medium);
    assert!(defaults.prefer_chat_fast_path);
    assert!((defaults.low_chars_max - 220.0).abs() < 1e-9);
    assert!((defaults.low_newlines_max - 1.0).abs() < 1e-9);
    assert!((defaults.high_chars_min - 1200.0).abs() < 1e-9);
    assert!((defaults.high_newlines_min - 8.0).abs() < 1e-9);
    assert!((defaults.high_tokens_min - 2200.0).abs() < 1e-9);

    let cfg = json!({
        "routing": {
            "fallback_classification_policy": {
                "enabled": "off",
                "only_when_medium_medium": "0",
                "prefer_chat_fast_path": "false",
                "low_chars_max": 9999,
                "low_newlines_max": -4,
                "high_chars_min": 9,
                "high_newlines_min": 222,
                "high_tokens_min": "30123"
            }
        }
    });
    let overridden = fallback_classification_policy(&cfg);
    assert!(!overridden.enabled);
    assert!(!overridden.only_when_medium_medium);
    assert!(!overridden.prefer_chat_fast_path);
    assert!((overridden.low_chars_max - 600.0).abs() < 1e-9);
    assert!((overridden.low_newlines_max - 0.0).abs() < 1e-9);
    assert!((overridden.high_chars_min - 240.0).abs() < 1e-9);
    assert!((overridden.high_newlines_min - 80.0).abs() < 1e-9);
    assert!((overridden.high_tokens_min - 30000.0).abs() < 1e-9);
}

#[test]
fn fallback_route_classification_respects_disable_force_and_generic_medium_gate() {
    let disabled_cfg = json!({
        "routing": {
            "fallback_classification_policy": {
                "enabled": false
            }
        }
    });
    let disabled = fallback_route_classification(FallbackRouteClassificationInput {
        cfg: &disabled_cfg,
        requested_risk: "unknown",
        requested_complexity: "unknown",
        intent: "hello",
        task: "",
        mode: "normal",
        role: "",
        tokens_est: None,
        class_policy: None,
    });
    assert!(!disabled.enabled);
    assert!(!disabled.applied);
    assert_eq!(disabled.reason, "disabled");
    assert_eq!(disabled.risk, "medium");
    assert_eq!(disabled.complexity, "medium");
    assert_eq!(disabled.role, "general");

    let forced_class = route_class_policy(&json!({}), "reflex");
    let forced = fallback_route_classification(FallbackRouteClassificationInput {
        cfg: &json!({}),
        requested_risk: "medium",
        requested_complexity: "medium",
        intent: "hello",
        task: "",
        mode: "normal",
        role: "general",
        tokens_est: None,
        class_policy: Some(&forced_class),
    });
    assert!(forced.enabled);
    assert!(!forced.applied);
    assert_eq!(forced.reason, "route_class_forced");

    let not_generic = fallback_route_classification(FallbackRouteClassificationInput {
        cfg: &json!({}),
        requested_risk: "low",
        requested_complexity: "medium",
        intent: "hello",
        task: "",
        mode: "normal",
        role: "general",
        tokens_est: None,
        class_policy: None,
    });
    assert!(not_generic.enabled);
    assert!(!not_generic.applied);
    assert_eq!(not_generic.reason, "not_generic_medium");
}

#[test]
fn fallback_route_classification_matches_fast_path_escalation_and_short_text_paths() {
    let fast_path = fallback_route_classification(FallbackRouteClassificationInput {
        cfg: &json!({}),
        requested_risk: "medium",
        requested_complexity: "medium",
        intent: "quick status",
        task: "",
        mode: "normal",
        role: "general",
        tokens_est: None,
        class_policy: None,
    });
    assert!(fast_path.applied);
    assert_eq!(fast_path.reason, "generic_medium_fast_path");
    assert_eq!(fast_path.risk, "low");
    assert_eq!(fast_path.complexity, "low");
    assert_eq!(fast_path.role, "chat");

    let escalation_cfg = json!({
        "routing": {
            "fallback_classification_policy": {
                "prefer_chat_fast_path": false,
                "high_chars_min": 30,
                "high_newlines_min": 5,
                "high_tokens_min": 1000
            }
        }
    });
    let escalated = fallback_route_classification(FallbackRouteClassificationInput {
        cfg: &escalation_cfg,
        requested_risk: "medium",
        requested_complexity: "medium",
        intent: "a fairly long request body that should escalate by character count",
        task: "",
        mode: "normal",
        role: "chat",
        tokens_est: Some(1200.0),
        class_policy: None,
    });
    assert!(escalated.applied);
    assert_eq!(escalated.reason, "generic_medium_complexity_escalation");
    assert_eq!(escalated.risk, "medium");
    assert_eq!(escalated.complexity, "high");
    assert_eq!(escalated.role, "general");

    let short_cfg = json!({
        "routing": {
            "fallback_classification_policy": {
                "prefer_chat_fast_path": false,
                "high_chars_min": 5000,
                "high_newlines_min": 99,
                "high_tokens_min": 5000
            }
        }
    });
    let short = fallback_route_classification(FallbackRouteClassificationInput {
        cfg: &short_cfg,
        requested_risk: "medium",
        requested_complexity: "medium",
        intent: "thanks",
        task: "",
        mode: "normal",
        role: "general",
        tokens_est: None,
        class_policy: None,
    });
    assert!(short.applied);
    assert_eq!(short.reason, "generic_medium_short_text");
    assert_eq!(short.risk, "low");
    assert_eq!(short.complexity, "low");
    assert_eq!(short.role, "chat");

    let no_override = fallback_route_classification(FallbackRouteClassificationInput {
        cfg: &short_cfg,
        requested_risk: "medium",
        requested_complexity: "medium",
        intent: "",
        task: "git status",
        mode: "normal",
        role: "general",
        tokens_est: None,
        class_policy: None,
    });
    assert!(!no_override.applied);
    assert_eq!(no_override.reason, "no_override");
    assert_eq!(no_override.risk, "medium");
    assert_eq!(no_override.complexity, "medium");
    assert_eq!(no_override.role, "general");
}
