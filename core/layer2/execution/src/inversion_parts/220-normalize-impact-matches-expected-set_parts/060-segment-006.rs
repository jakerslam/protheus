        let enabled = compute_attractor_score(&ComputeAttractorScoreInput {
            attractor: Some(json!({
                "enabled": true,
                "weights": {
                    "objective_specificity": 0.3,
                    "evidence_backing": 0.2,
                    "constraint_evidence": 0.15,
                    "measurable_outcome": 0.1,
                    "external_grounding": 0.05,
                    "certainty": 0.1,
                    "trit_alignment": 0.05,
                    "impact_alignment": 0.05,
                    "verbosity_penalty": 0.15
                },
                "verbosity": {
                    "soft_word_cap": 18,
                    "hard_word_cap": 80,
                    "low_diversity_floor": 0.22
                },
                "min_alignment_by_target": {
                    "directive": 0.2
                }
            })),
            objective: Some(
                "Must reduce drift below 2% within 7 days with measurable latency impact."
                    .to_string(),
            ),
            signature: Some(
                "Use github telemetry and external api evidence to improve throughput by 20%."
                    .to_string(),
            ),
            external_signals_count: Some(json!(3)),
            evidence_count: Some(json!(4)),
            effective_certainty: Some(json!(0.9)),
            trit: Some(json!(1)),
            impact: Some("high".to_string()),
            target: Some("directive".to_string()),
        });
        assert!(enabled.enabled);
        assert!(enabled.score >= 0.0 && enabled.score <= 1.0);
        assert!(enabled.required >= 0.0 && enabled.required <= 1.0);
        assert!(enabled.components.as_object().is_some());
        let word_count = enabled
            .components
            .as_object()
            .and_then(|m| m.get("word_count"))
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);
        assert!(word_count >= 0);
    }

    #[test]
    fn helper_primitives_batch10_match_contract() {
        let out = compute_build_output_interfaces(&BuildOutputInterfacesInput {
            outputs: Some(json!({
                "default_channel": "strategy_hint",
                "belief_update": { "enabled": true, "test_enabled": true, "live_enabled": false, "require_sandbox_verification": false, "require_explicit_emit": false },
                "strategy_hint": { "enabled": true, "test_enabled": true, "live_enabled": true, "require_sandbox_verification": false, "require_explicit_emit": false },
                "workflow_hint": { "enabled": false, "test_enabled": true, "live_enabled": true, "require_sandbox_verification": false, "require_explicit_emit": false },
                "code_change_proposal": { "enabled": true, "test_enabled": true, "live_enabled": true, "require_sandbox_verification": true, "require_explicit_emit": true }
            })),
            mode: Some("test".to_string()),
            sandbox_verified: Some(json!(false)),
            explicit_code_proposal_emit: Some(json!(false)),
            channel_payloads: Some(json!({
                "strategy_hint": { "hint": "x" }
            })),
            base_payload: Some(json!({ "base": true })),
        });

        assert_eq!(out.default_channel, "strategy_hint".to_string());
        assert_eq!(out.active_channel, Some("strategy_hint".to_string()));
        let channels = out.channels.as_object().expect("channels object");
        assert_eq!(channels.len(), 4);
        let proposal = channels
            .get("code_change_proposal")
            .and_then(|v| v.as_object())
            .expect("proposal object");
        assert!(!proposal
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true));
        assert!(proposal
            .get("gated_reasons")
            .and_then(|v| v.as_array())
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("sandbox_verification_required")))
            .unwrap_or(false));
    }

    #[test]
    fn helper_primitives_batch11_match_contract() {
        let out = compute_build_code_change_proposal_draft(&BuildCodeChangeProposalDraftInput {
            base: Some(json!({
                "objective": "Harden inversion lane",
                "objective_id": "BL-214",
                "ts": "2026-03-04T00:00:00.000Z",
                "mode": "test",
                "impact": "high",
                "target": "directive",
                "certainty": 0.7333333,
                "maturity_band": "developing",
                "reasons": ["one", "two"],
                "shadow_mode": true
            })),
            args: Some(json!({
                "code_change_title": "Migrate proposal draft builder",
                "code_change_summary": "Rust-first proposal generation with parity fallback.",
                "code_change_files": ["client/runtime/systems/autonomy/inversion_controller.ts"],
                "code_change_tests": ["tests/client-memory-tools/inversion_helper_batch11_rust_parity.test.ts"],
                "code_change_risk": "low"
            })),
            opts: Some(json!({
                "session_id": "ivs_123",
                "sandbox_verified": true
            })),
        });
        let proposal = out.proposal.as_object().expect("proposal object");
        assert_eq!(
            proposal.get("type").and_then(|v| v.as_str()).unwrap_or(""),
            "code_change_proposal"
        );
        assert!(proposal
            .get("proposal_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .starts_with("icp_"));
        assert!(proposal
            .get("sandbox_verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
    }

    #[test]
    fn helper_primitives_batch12_match_contract() {
        let out = compute_normalize_library_row(&NormalizeLibraryRowInput {
            row: Some(json!({
                "id": " abc ",
                "ts": "2026-03-04T00:00:00.000Z",
                "objective": " Ship lane ",
                "objective_id": " BL-215 ",
                "signature": "  reduce drift safely ",
                "target": "directive",
                "impact": "high",
                "certainty": 1.2,
                "filter_stack": ["risk_guard", "  "],
                "outcome_trit": 2,
                "result": "OK",
                "maturity_band": "Developing",
                "principle_id": " p1 ",
                "session_id": " s1 "
            })),
        });
        let row = out.row.as_object().expect("row object");
        assert_eq!(row.get("id").and_then(|v| v.as_str()).unwrap_or(""), "abc");
        assert_eq!(
            row.get("target").and_then(|v| v.as_str()).unwrap_or(""),
            "directive"
        );
        assert_eq!(
            row.get("impact").and_then(|v| v.as_str()).unwrap_or(""),
            "high"
        );
        assert_eq!(
            row.get("outcome_trit")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            1
        );
    }

