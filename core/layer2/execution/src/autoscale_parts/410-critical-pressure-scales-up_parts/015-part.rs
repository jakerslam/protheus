                "module": " autonomy_backlog_autoscale ",
                "current_cells": 2.8,
                "target_cells": "5",
                "last_run_ts": " 2026-03-04T00:00:00.000Z ",
                "last_high_pressure_ts": "",
                "last_action": " scale_up ",
                "updated_at": null
            })),
        });
        assert_eq!(out.module, "autonomy_backlog_autoscale");
        assert_eq!(out.current_cells, 2.8);
        assert_eq!(out.target_cells, 5.0);
        assert_eq!(
            out.last_run_ts,
            Some("2026-03-04T00:00:00.000Z".to_string())
        );
        assert_eq!(out.last_high_pressure_ts, None);
        assert_eq!(out.last_action, Some("scale_up".to_string()));
        assert_eq!(out.updated_at, None);
    }

    #[test]
    fn autoscale_json_normalize_backlog_autoscale_state_path_works() {
        let payload = serde_json::json!({
            "mode": "normalize_backlog_autoscale_state",
            "normalize_backlog_autoscale_state_input": {
                "module": "autonomy_backlog_autoscale",
                "src": {
                    "current_cells": 1,
                    "target_cells": 3
                }
            }
        })
        .to_string();
        let out =
            run_autoscale_json(&payload).expect("autoscale normalize_backlog_autoscale_state");
        assert!(out.contains("\"mode\":\"normalize_backlog_autoscale_state\""));
    }

    #[test]
    fn spawn_allocated_cells_prefers_active_then_current_then_allocated() {
        let out = compute_spawn_allocated_cells(&SpawnAllocatedCellsInput {
            active_cells: Some(4.2),
            current_cells: Some(7.0),
            allocated_cells: Some(9.0),
        });
        assert_eq!(out.active_cells, Some(4));
        let out = compute_spawn_allocated_cells(&SpawnAllocatedCellsInput {
            active_cells: None,
            current_cells: Some(7.8),
            allocated_cells: Some(9.0),
        });
        assert_eq!(out.active_cells, Some(7));
        let out = compute_spawn_allocated_cells(&SpawnAllocatedCellsInput {
            active_cells: None,
            current_cells: None,
            allocated_cells: Some(2.0),
        });
        assert_eq!(out.active_cells, Some(2));
    }

    #[test]
    fn autoscale_json_spawn_allocated_cells_path_works() {
        let payload = serde_json::json!({
            "mode": "spawn_allocated_cells",
            "spawn_allocated_cells_input": {
                "active_cells": null,
                "current_cells": 3.4,
                "allocated_cells": 8
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale spawn_allocated_cells");
        assert!(out.contains("\"mode\":\"spawn_allocated_cells\""));
    }

    #[test]
    fn spawn_capacity_boost_snapshot_counts_recent_spawn_grants() {
        let out = compute_spawn_capacity_boost_snapshot(&SpawnCapacityBoostSnapshotInput {
            enabled: true,
            lookback_minutes: 30.0,
            min_granted_cells: 1.0,
            now_ms: 1_700_000_000_000.0,
            rows: vec![
                SpawnCapacityBoostRowInput {
                    r#type: Some("spawn_request".to_string()),
                    ts: Some("2023-11-14T22:13:20.000Z".to_string()),
                    granted_cells: Some(2.0),
                },
                SpawnCapacityBoostRowInput {
                    r#type: Some("spawn_request".to_string()),
                    ts: Some("2023-11-14T22:12:20.000Z".to_string()),
                    granted_cells: Some(1.0),
                },
            ],
        });
        assert!(out.active);
        assert_eq!(out.grant_count, 2);
        assert_eq!(out.granted_cells, 3.0);
        assert_eq!(out.latest_ts, Some("2023-11-14T22:12:20.000Z".to_string()));
    }

    #[test]
    fn autoscale_json_spawn_capacity_boost_snapshot_path_works() {
        let payload = serde_json::json!({
            "mode": "spawn_capacity_boost_snapshot",
            "spawn_capacity_boost_snapshot_input": {
                "enabled": true,
                "lookback_minutes": 30,
                "min_granted_cells": 1,
                "now_ms": 1700000000000i64,
                "rows": [
                    {
                        "type": "spawn_request",
                        "ts": "2023-11-14T22:13:20.000Z",
                        "granted_cells": 1
                    }
                ]
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale spawn_capacity_boost_snapshot");
        assert!(out.contains("\"mode\":\"spawn_capacity_boost_snapshot\""));
    }

    #[test]
    fn inversion_maturity_score_matches_expected_banding() {
        let out = compute_inversion_maturity_score(&InversionMaturityScoreInput {
            total_tests: 40.0,
            passed_tests: 32.0,
            destructive_failures: 1.0,
            target_test_count: 40.0,
            weight_pass_rate: 0.5,
            weight_non_destructive_rate: 0.3,
            weight_experience: 0.2,
            band_novice: 0.25,
            band_developing: 0.45,
            band_mature: 0.65,
            band_seasoned: 0.82,
        });
        assert_eq!(out.band, "legendary");
        assert!(out.score >= 0.82);
        assert!(out.pass_rate >= 0.8 && out.pass_rate <= 0.81);
        assert!(out.non_destructive_rate >= 0.97 && out.non_destructive_rate <= 0.98);
    }

    #[test]
    fn autoscale_json_inversion_maturity_score_path_works() {
        let payload = serde_json::json!({
            "mode": "inversion_maturity_score",
            "inversion_maturity_score_input": {
                "total_tests": 10,
                "passed_tests": 6,
                "destructive_failures": 1,
                "target_test_count": 40,
                "weight_pass_rate": 0.5,
                "weight_non_destructive_rate": 0.3,
                "weight_experience": 0.2,
                "band_novice": 0.25,
                "band_developing": 0.45,
                "band_mature": 0.65,
                "band_seasoned": 0.82
            }
        })
        .to_string();
        let out = run_autoscale_json(&payload).expect("autoscale inversion_maturity_score");
        assert!(out.contains("\"mode\":\"inversion_maturity_score\""));
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
const a = runBacklogAutoscalePrimitive("alpha", {});
const b = runBacklogAutoscalePrimitive('beta', {});
const c = runBacklogAutoscalePrimitive(`gamma`, {});
"#;
        let parsed = extract_mode_literals(text, "runBacklogAutoscalePrimitive");
        let expected = ["alpha", "beta", "gamma"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_accepts_quoted_and_unquoted_keys() {
        let bridge = r#"
function runBacklogAutoscalePrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha",
    "beta-mode": "payload_beta",
    'gamma_mode': "payload_gamma"
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
        let expected = ["alpha", "beta-mode", "gamma_mode"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_allows_non_string_values() {
        let bridge = r#"
function runBacklogAutoscalePrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: payloadAlpha,
    "beta-mode": payloadBeta,
    'gamma_mode': payloadGamma
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
        let expected = ["alpha", "beta-mode", "gamma_mode"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_selects_requested_function_section() {
        let bridge = r#"
function runBacklogAutoscalePrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
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
        let parsed_backlog = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
        let expected_backlog = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed_backlog, expected_backlog);

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
function runBacklogAutoscalePrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha",
    beta: "payload_beta"
  }
}
"#;
        let parsed = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
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
        let parsed = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
        assert!(parsed.is_empty());
    }

    #[test]
    fn extract_bridge_modes_supports_crlf_lines() {
        let bridge = "function runBacklogAutoscalePrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {\r\n  const fieldByMode: AnyObj = {\r\n    alpha: \"payload_alpha\",\r\n    beta: \"payload_beta\"\r\n  }\r\n}\r\n";
        let parsed = extract_bridge_modes(bridge, "runBacklogAutoscalePrimitive");
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_mode_literals_ignores_dynamic_template_modes() {
        let text = r#"
const a = runBacklogAutoscalePrimitive("alpha", {});
const b = runBacklogAutoscalePrimitive(`beta_${suffix}`, {});
const c = runBacklogAutoscalePrimitive(modeName, {});
"#;
        let parsed = extract_mode_literals(text, "runBacklogAutoscalePrimitive");
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_mode_literals_ignores_commented_calls() {
        let text = r#"
// runBacklogAutoscalePrimitive("ignored_line", {});
/* runBacklogAutoscalePrimitive("ignored_block", {}); */
const a = runBacklogAutoscalePrimitive(
  "alpha",
  {}
);
"#;
        let parsed = extract_mode_literals(text, "runBacklogAutoscalePrimitive");
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
    fn backlog_bridge_is_wrapper_only_in_coreized_layout() {
        let ts_autonomy = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/autonomy_controller.ts",
        );
        let ts_inversion = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/inversion_controller.ts",
        );
        let bridge = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/backlog_autoscale_rust_bridge.ts",
        );
        let mut called = extract_mode_literals(&ts_autonomy, "runBacklogAutoscalePrimitive");
        called.extend(extract_mode_literals(
            &ts_inversion,
            "runBacklogAutoscalePrimitive",
        ));
        if bridge.is_empty() {
            assert!(
                called.is_empty(),
                "coreized wrappers should not carry backlog autoscale mode calls"
            );
            return;
        }
        assert!(
            bridge.contains("createLegacyRetiredModule"),
            "backlog_autoscale_rust_bridge.js must remain a thin wrapper"
        );
        assert!(
            !bridge.contains("fieldByMode"),
            "wrapper-only bridge must not contain legacy mode maps"
