// SPDX-License-Identifier: Apache-2.0
use super::*;
use std::path::Path;

fn assert_role_matrix(rows: &[(&str, &str, &str)]) {
    for (input, context, expected) in rows {
        assert_eq!(infer_role(input, context), *expected);
    }
}

#[test]
fn local_ollama_model_detection_is_strict() {
    assert!(is_local_ollama_model("ollama/llama3"));
    assert!(!is_local_ollama_model("ollama/llama3:cloud"));
    assert!(!is_local_ollama_model("openai/gpt-4.1"));
    assert!(is_cloud_model("openai/gpt-4.1"));
    assert!(is_cloud_model("ollama/llama3:cloud"));
    assert!(!is_cloud_model(""));
    assert_eq!(ollama_model_name("ollama/llama3"), "llama3");
    assert_eq!(ollama_model_name("openai/gpt-4.1"), "openai/gpt-4.1");
}

#[test]
fn tier_inference_matches_risk_complexity_contract() {
    assert_eq!(infer_tier("high", "low"), 3);
    assert_eq!(infer_tier("low", "high"), 3);
    assert_eq!(infer_tier("medium", "low"), 2);
    assert_eq!(infer_tier("low", "medium"), 2);
    assert_eq!(infer_tier("low", "low"), 1);
    assert_eq!(normalize_risk_level("unknown"), "medium");
    assert_eq!(normalize_complexity_level("bad"), "medium");
    assert_eq!(normalize_risk_level("HIGH"), "high");
    assert_eq!(normalize_complexity_level(" LOW "), "low");
}

#[test]
fn role_inference_preserves_persona_lens_priority() {
    assert_role_matrix(&[
        ("fix compile issue", "patch node script", "coding"),
        ("integrate with api", "cli automation", "tools"),
        ("plan next sprint", "roadmap prioritization", "planning"),
        ("derive proof", "logic constraints", "logic"),
        ("write summary", "explain status", "chat"),
        ("random", "unclassified", "general"),
    ]);
}

#[test]
fn capability_and_route_helpers_match_contract() {
    assert_eq!(
        normalize_capability_key("  Proposal:Decision@Tier!Alpha  "),
        "proposal:decision_tier_alpha"
    );
    assert_eq!(infer_capability("patch node script", "", ""), "file_edit");
    assert_eq!(infer_capability("please read config", "", ""), "file_read");
    assert_eq!(infer_capability("use cli automation", "", ""), "tool_use");
    assert_eq!(infer_capability("summar report", "", ""), "chat");
    assert_eq!(infer_capability("", "", "coding"), "role:coding");
    assert_eq!(infer_capability("", "", ""), "general");

    assert_eq!(
        capability_family_key("proposal:doctor:repair"),
        "proposal_doctor"
    );
    assert_eq!(capability_family_key("file_edit"), "file_edit");
    assert_eq!(
        task_type_key_from_route("reflex", "proposal:doctor", "logic"),
        "class:reflex"
    );
    assert_eq!(
        task_type_key_from_route("default", "proposal:doctor", "logic"),
        "cap:proposal_doctor"
    );
    assert_eq!(
        task_type_key_from_route("default", "", "planning"),
        "role:planning"
    );
}

#[test]
fn pressure_helpers_match_contract() {
    assert_eq!(pressure_order("critical"), 4);
    assert_eq!(pressure_order("high"), 3);
    assert_eq!(pressure_order("soft"), 2);
    assert_eq!(pressure_order("low"), 1);
    assert_eq!(pressure_order("none"), 0);

    assert_eq!(normalize_router_pressure("critical"), "hard");
    assert_eq!(normalize_router_pressure("high"), "hard");
    assert_eq!(normalize_router_pressure("medium"), "soft");
    assert_eq!(normalize_router_pressure("unknown"), "none");
}

#[test]
fn request_token_estimation_matches_contract() {
    assert_eq!(estimate_request_tokens(Some(42.2), "", ""), 120);
    assert_eq!(estimate_request_tokens(Some(130.6), "", ""), 131);
    assert_eq!(estimate_request_tokens(Some(14_000.0), "", ""), 12_000);
    assert_eq!(estimate_request_tokens(None, "", ""), 120);

    let text = "x".repeat(1_000);
    assert_eq!(estimate_request_tokens(None, "", &text), 359);
}

#[test]
fn model_multiplier_resolution_matches_contract() {
    let policy = json!({
        "model_token_multipliers": {
            "OpenAI/GPT-4.1": "1.8"
        },
        "class_token_multipliers": {
            "cheap_local": 0.42,
            "local": 0.5,
            "cloud": 1.4,
            "default": 1.1
        }
    });

    let by_model = resolve_model_token_multiplier("openai/gpt-4.1", "cheap_local", &policy);
    assert_eq!(by_model.source, "model");
    assert!((by_model.multiplier - 1.8).abs() < 1e-9);

    let by_class = resolve_model_token_multiplier("ollama/llama3", "cheap_local", &policy);
    assert_eq!(by_class.source, "class");
    assert!((by_class.multiplier - 0.42).abs() < 1e-9);

    let cloud_class = resolve_model_token_multiplier("frontier_provider/claude-3-5", "", &policy);
    assert_eq!(cloud_class.source, "class");
    assert!((cloud_class.multiplier - 1.4).abs() < 1e-9);
}

#[test]
fn model_multiplier_uses_js_truthy_fallback_chain() {
    let policy = json!({
        "class_token_multipliers": {
            "cheap_local": 0,
            "local": 0.5
        }
    });
    let detail = resolve_model_token_multiplier("ollama/llama3", "cheap_local", &policy);
    assert_eq!(detail.source, "class");
    assert!((detail.multiplier - 0.5).abs() < 1e-9);
}

#[test]
fn model_request_token_estimate_matches_contract() {
    let policy = json!({
        "model_token_multipliers": {
            "openai/gpt-4.1": 1.23456
        }
    });
    let out = estimate_model_request_tokens("openai/gpt-4.1", Some(1_000.0), "", &policy);
    assert_eq!(out.source, "model");
    assert_eq!(out.tokens_est, Some(1_235));
    assert_eq!(out.multiplier, Some(1.2346));

    let none = estimate_model_request_tokens("openai/gpt-4.1", Some(0.0), "", &policy);
    assert_eq!(none.source, "none");
    assert_eq!(none.tokens_est, None);
    assert_eq!(none.multiplier, None);
}

#[test]
fn communication_fast_path_policy_matches_defaults_and_overrides() {
    let defaults = communication_fast_path_policy(&json!({}));
    assert!(defaults.enabled);
    assert_eq!(defaults.match_mode, "heuristic");
    assert_eq!(defaults.max_chars, 48);
    assert_eq!(defaults.max_words, 8);
    assert_eq!(defaults.max_newlines, 0);
    assert!(defaults.patterns.is_empty());
    assert_eq!(
        defaults.disallow_regexes,
        DEFAULT_FAST_PATH_DISALLOW_REGEXES
            .iter()
            .map(|row| row.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(defaults.slot, "grunt");
    assert_eq!(defaults.prefer_model, "ollama/smallthinker");
    assert_eq!(defaults.fallback_slot, "fallback");
    assert!(defaults.skip_outcome_scan);

    let cfg = json!({
        "routing": {
            "communication_fast_path": {
                "enabled": "off",
                "match_mode": "patterns",
                "max_chars": 999,
                "max_words": "3",
                "max_newlines": -5,
                "patterns": ["status", 7],
                "disallow_regexes": ["foo", "bar"],
                "slot": "smalltalk",
                "prefer_model": "openai/gpt-4.1-mini",
                "fallback_slot": "default",
                "skip_outcome_scan": "no"
            }
        }
    });
    let overridden = communication_fast_path_policy(&cfg);
    assert!(!overridden.enabled);
    assert_eq!(overridden.match_mode, "patterns");
    assert_eq!(overridden.max_chars, 220);
    assert_eq!(overridden.max_words, 3);
    assert_eq!(overridden.max_newlines, 0);
    assert_eq!(
        overridden.patterns,
        vec!["status".to_string(), "7".to_string()]
    );
    assert_eq!(
        overridden.disallow_regexes,
        vec!["foo".to_string(), "bar".to_string()]
    );
    assert_eq!(overridden.slot, "smalltalk");
    assert_eq!(overridden.prefer_model, "openai/gpt-4.1-mini");
    assert_eq!(overridden.fallback_slot, "default");
    assert!(!overridden.skip_outcome_scan);
}

#[test]
fn communication_fast_path_detection_rejects_structured_or_disallowed_modes() {
    let empty = json!({});
    let mode_blocked =
        detect_communication_fast_path(&empty, "low", "low", "hello", "", "deep-thinker", false);
    assert!(!mode_blocked.matched);
    assert_eq!(mode_blocked.reason, "mode_disallowed");
    assert!(mode_blocked.blocked_pattern.is_none());

    let structured =
        detect_communication_fast_path(&empty, "low", "low", "", "run git status", "normal", false);
    assert!(!structured.matched);
    assert_eq!(structured.reason, "contains_structured_intent");
    assert_eq!(
        structured.blocked_pattern.as_deref(),
        Some("\\b(node|npm|pnpm|yarn|git|curl|python|bash|zsh|ollama)\\b")
    );

    let risk_blocked =
        detect_communication_fast_path(&empty, "medium", "low", "hello there", "", "normal", false);
    assert!(!risk_blocked.matched);
    assert_eq!(risk_blocked.reason, "risk_not_low");
}
