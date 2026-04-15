        let live_at = "2999-01-01T00:00:00.000Z".to_string();
        let _ = compute_save_active_sessions(&SaveActiveSessionsInput {
            file_path: Some(active_sessions_path.to_string_lossy().to_string()),
            store: Some(json!({
                "sessions":[
                    {"session_id":"exp","objective":"old","signature":"old sig","target":"directive","impact":"high","certainty":0.5,"expires_at": expired_at},
                    {"session_id":"live","objective":"new","signature":"new sig","target":"directive","impact":"high","certainty":0.6,"expires_at": live_at}
                ]
            })),
            now_iso: Some(now.clone()),
        });
        let sweep = compute_sweep_expired_sessions(&SweepExpiredSessionsInput {
            paths: Some(json!({
                "active_sessions_path": active_sessions_path.to_string_lossy().to_string(),
                "receipts_path": receipts_path.to_string_lossy().to_string(),
                "library_path": library_path.to_string_lossy().to_string(),
                "events_dir": temp_root.join("events").to_string_lossy().to_string()
            })),
            policy: Some(json!({"telemetry":{"emit_events":false},"library":{"max_entries":200}})),
            date_str: Some("2026-03-04".to_string()),
            now_iso: Some(now.clone()),
        });
        assert_eq!(sweep.expired_count, 1);
        assert_eq!(sweep.sessions.len(), 1);

        let _ = fs::write(
            temp_root.join("regime.json"),
            serde_json::to_string(&json!({
                "selected_regime":"constrained",
                "candidate_confidence":0.8,
                "context":{"trit":{"trit":-1}}
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("mirror.json"),
            serde_json::to_string(
                &json!({"pressure_score":0.7,"confidence":0.75,"reasons":["pressure","drift"]}),
            )
            .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("drift_governor.json"),
            serde_json::to_string(&json!({"last_decision":{"trit_shadow":{"belief":{"trit":-1}}}}))
                .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("simulation").join("2026-03-04.json"),
            serde_json::to_string(&json!({"checks_effective":{"drift_rate":{"value":0.09},"yield_rate":{"value":0.4}}}))
                .unwrap_or_else(|_| "{}".to_string()),
        );
        let _ = fs::write(
            temp_root.join("red_team").join("latest.json"),
            serde_json::to_string(
                &json!({"summary":{"critical_fail_cases":2,"pass_cases":1,"fail_cases":3}}),
            )
            .unwrap_or_else(|_| "{}".to_string()),
        );

        let signals = compute_load_impossibility_signals(&LoadImpossibilitySignalsInput {
            policy: Some(json!({
                "organ": {
                    "trigger_detection": {
                        "paths": {
                            "regime_latest_path":"regime.json",
                            "mirror_latest_path":"mirror.json",
                            "simulation_dir":"simulation",
                            "red_team_runs_dir":"red_team",
                            "drift_governor_path":"drift_governor.json"
                        }
                    }
                }
            })),
            date_str: Some("2026-03-04".to_string()),
            root: Some(temp_root.to_string_lossy().to_string()),
        });
        assert_eq!(
            value_path(Some(&signals.signals), &["trit", "value"])
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            -1
        );

        let trigger = compute_evaluate_impossibility_trigger(&EvaluateImpossibilityTriggerInput {
            policy: Some(json!({
                "organ": {
                    "trigger_detection": {
                        "enabled": true,
                        "min_impossibility_score": 0.58,
                        "min_signal_count": 2,
                        "thresholds": {"predicted_drift_warn":0.03,"predicted_yield_warn":0.68},
                        "weights": {
                            "trit_pain":0.2,
                            "mirror_pressure":0.2,
                            "predicted_drift":0.18,
                            "predicted_yield_gap":0.18,
                            "red_team_critical":0.14,
                            "regime_constrained":0.1
                        }
                    }
                }
            })),
            signals: Some(signals.signals.clone()),
            force: Some(false),
        });
        assert!(trigger.triggered);
        assert!(trigger.signal_count >= 2);

        let fp_policy = json!({
            "first_principles": {
                "enabled": true,
                "auto_extract_on_success": true,
                "max_strategy_bonus": 0.12,
                "allow_failure_cluster_extraction": true,
                "failure_cluster_min": 4
            },
            "library": {
                "min_similarity_for_reuse": 0.2,
                "token_weight": 0.6,
                "trit_weight": 0.3,
                "target_weight": 0.1
            }
        });
        let session = json!({
            "session_id":"sfp",
            "objective":"Reduce drift safely",
            "objective_id":"BL-263",
            "target":"directive",
            "certainty":0.8,
            "filter_stack":["drift_guard"],
            "signature":"drift guard stable",
            "signature_tokens":["drift","guard","stable"]
        });
        let first_principle = compute_extract_first_principle(&ExtractFirstPrincipleInput {
            policy: Some(fp_policy.clone()),
            session: Some(session.clone()),
            args: Some(json!({})),
            result: Some("success".to_string()),
            now_iso: Some(now_iso_runtime()),
        });
        assert!(first_principle.principle.is_some());

        let failure_principle =
            compute_extract_failure_cluster_principle(&ExtractFailureClusterPrincipleInput {
                paths: Some(json!({"library_path": library_path.to_string_lossy().to_string()})),
                policy: Some(fp_policy),
                session: Some(session.clone()),
                now_iso: Some(now_iso_runtime()),
            });
        assert!(failure_principle.principle.is_some());

        let persisted = compute_persist_first_principle(&PersistFirstPrincipleInput {
            paths: Some(json!({
                "first_principles_latest_path": fp_latest_path.to_string_lossy().to_string(),
                "first_principles_history_path": fp_history_path.to_string_lossy().to_string(),
                "first_principles_lock_path": fp_lock_path.to_string_lossy().to_string()
            })),
            session: Some(session),
            principle: first_principle.principle.clone(),
            now_iso: Some(now_iso_runtime()),
        });
        assert!(persisted.principle.is_object());
        assert!(fp_latest_path.exists());
    }

    fn extract_mode_literals(text: &str, call_name: &str) -> std::collections::BTreeSet<String> {
        let pattern = format!(r#"{}\s*\(\s*['"`]([^'"`]+)['"`]"#, regex::escape(call_name));
        let re = Regex::new(&pattern).expect("valid call regex");
        let static_mode_re =
            Regex::new(r"^[a-zA-Z0-9_-]+$").expect("valid static mode token regex");
        let block_comment_re = Regex::new(r"(?s)/\*.*?\*/").expect("valid block comment regex");
        let line_comment_re = Regex::new(r"(?m)//.*$").expect("valid line comment regex");
        let without_block = block_comment_re.replace_all(text, "");
        let cleaned = line_comment_re.replace_all(&without_block, "");
        re.captures_iter(cleaned.as_ref())
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
            .filter(|mode| !mode.is_empty() && static_mode_re.is_match(mode))
            .collect()
    }

    fn extract_bridge_modes(text: &str, fn_name: &str) -> std::collections::BTreeSet<String> {
        let section_re = Regex::new(&format!(
            r#"(?s)function {}\s*\([^)]*\)\s*\{{.*?const fieldByMode:\s*AnyObj\s*=\s*\{{(.*?)\}}\s*(?:;|\r?\n)?"#,
            regex::escape(fn_name)
        ))
        .expect("valid section regex");
        let keys_re = Regex::new(r#"(?m)^\s*(?:([a-zA-Z0-9_]+)|['"]([^'"]+)['"])\s*:"#)
            .expect("valid key regex");
        let Some(section) = section_re
            .captures(text)
            .and_then(|cap| cap.get(1).map(|m| m.as_str()))
        else {
            return std::collections::BTreeSet::new();
        };
        keys_re
            .captures_iter(section)
            .filter_map(|cap| {
                cap.get(1)
                    .or_else(|| cap.get(2))
                    .map(|m| m.as_str().trim().to_string())
            })
            .filter(|key| !key.is_empty())
            .collect()
    }

    fn extract_dispatch_modes(text: &str) -> std::collections::BTreeSet<String> {
        let re = Regex::new(r#"(?m)^\s*(?:if|else if) mode == "([^"]+)""#)
            .expect("valid dispatch regex");
        let block_comment_re = Regex::new(r"(?s)/\*.*?\*/").expect("valid block comment regex");
        let line_comment_re = Regex::new(r"(?m)//.*$").expect("valid line comment regex");
        let without_block = block_comment_re.replace_all(text, "");
        let cleaned = line_comment_re.replace_all(&without_block, "");
        re.captures_iter(cleaned.as_ref())
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
            .filter(|mode| !mode.is_empty())
            .collect()
    }

    #[test]
    fn extract_mode_literals_accepts_all_quote_styles() {
        let text = r#"
const a = runInversionPrimitive("alpha", {});
const b = runInversionPrimitive('beta', {});
const c = runInversionPrimitive(`gamma`, {});
"#;
        let parsed = extract_mode_literals(text, "runInversionPrimitive");
        let expected = ["alpha", "beta", "gamma"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_accepts_quoted_and_unquoted_keys() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha",
    "beta-mode": "payload_beta",
    'gamma_mode': "payload_gamma"
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta-mode", "gamma_mode"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_allows_non_string_values() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: payloadAlpha,
    "beta-mode": payloadBeta,
    'gamma_mode': payloadGamma
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta-mode", "gamma_mode"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_selects_requested_function_section() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha"
  };
}
function runOtherPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    rogue: "payload_rogue"
  };
}
"#;
        let parsed_inversion = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected_inversion = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed_inversion, expected_inversion);

        let parsed_other = extract_bridge_modes(bridge, "runOtherPrimitive");
        let expected_other = ["rogue"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed_other, expected_other);
    }

    #[test]
    fn extract_bridge_modes_allows_missing_trailing_semicolon() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha",
    beta: "payload_beta"
  }
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_returns_empty_when_function_missing() {
        let bridge = r#"
function runOtherPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    rogue: "payload_rogue"
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        assert!(parsed.is_empty());
    }

    #[test]
    fn extract_bridge_modes_supports_crlf_lines() {
        let bridge = "function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {\r\n  const fieldByMode: AnyObj = {\r\n    alpha: \"payload_alpha\",\r\n    beta: \"payload_beta\"\r\n  }\r\n}\r\n";
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_mode_literals_ignores_dynamic_template_modes() {
        let text = r#"
const a = runInversionPrimitive("alpha", {});
const b = runInversionPrimitive(`beta_${suffix}`, {});
const c = runInversionPrimitive(modeName, {});
"#;
        let parsed = extract_mode_literals(text, "runInversionPrimitive");
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_mode_literals_ignores_commented_calls() {
        let text = r#"
// runInversionPrimitive("ignored_line", {});
/* runInversionPrimitive("ignored_block", {}); */
const a = runInversionPrimitive(
  "alpha",
  {}
);
"#;
        let parsed = extract_mode_literals(text, "runInversionPrimitive");
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_dispatch_modes_accepts_if_and_else_if() {
        let text = r#"
if mode == "alpha" {
}
else if mode == "beta" {
}
if another == "gamma" {
}
"#;
        let parsed = extract_dispatch_modes(text);
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_dispatch_modes_ignores_commented_branches() {
        let text = r#"
// if mode == "ignored_line" {
// }
/* else if mode == "ignored_block" {
} */
if mode == "alpha" {
}
"#;
        let parsed = extract_dispatch_modes(text);
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    fn read_optional_autonomy_surface(rel: &str) -> String {
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel))
            .unwrap_or_default()
    }

    #[test]
    fn inversion_bridge_is_wrapper_only_in_coreized_layout() {
        let ts_autonomy = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/autonomy_controller.ts",
        );
        let ts_inversion = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/inversion_controller.ts",
        );
        let bridge = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/backlog_autoscale_rust_bridge.ts",
        );
        let mut called = extract_mode_literals(&ts_inversion, "runInversionPrimitive");
        called.extend(extract_mode_literals(&ts_autonomy, "runInversionPrimitive"));
        if bridge.is_empty() {
            assert!(
                called.is_empty(),
                "coreized wrappers should not carry inversion mode calls"
            );
            return;
        }
        assert!(
            bridge.contains("createLegacyRetiredModule"),
            "backlog_autoscale_rust_bridge.js must remain a thin wrapper"
        );
        assert!(
            !bridge.contains("fieldByMode"),
            "wrapper-only bridge must not contain legacy inversion mode maps"
        );
        assert!(
            called.is_empty(),
            "coreized wrappers should not carry inversion mode calls"
        );
    }

