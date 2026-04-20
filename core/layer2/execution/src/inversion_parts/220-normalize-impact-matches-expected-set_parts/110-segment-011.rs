        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a1",
                "ts":"2026-03-04T00:00:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.9,
                "filter_stack":["drift_guard"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a2",
                "ts":"2026-03-04T00:10:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.88,
                "filter_stack":["drift_guard","identity_guard"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a3",
                "ts":"2026-03-04T00:20:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.86,
                "filter_stack":["drift_guard","fallback_pathing"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"a4",
                "ts":"2026-03-04T00:30:00.000Z",
                "objective":"Reduce drift safely",
                "objective_id":"BL-263",
                "signature":"drift guard stable",
                "signature_tokens":["drift","guard","stable"],
                "target":"directive",
                "impact":"high",
                "certainty":0.84,
                "filter_stack":["drift_guard","constraint_reframe"],
                "outcome_trit":-1,
                "result":"fail",
                "maturity_band":"developing"
            })),
        });
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(library_path.to_string_lossy().to_string()),
            row: Some(json!({
                "id":"ok1",
                "ts":"2026-03-04T01:00:00.000Z",
                "objective":"Ship safely",
                "signature":"safe lane pass",
                "signature_tokens":["safe","lane","pass"],
                "target":"directive",
                "impact":"high",
                "certainty":0.92,
                "filter_stack":["safe_path"],
                "outcome_trit":1,
                "result":"success",
                "maturity_band":"mature"
            })),
        });

        let detect = compute_detect_immutable_axiom_violation(
            &DetectImmutableAxiomViolationInput {
                policy: Some(json!({
                    "immutable_axioms": {
                        "enabled": true,
                        "axioms": [{
                            "id":"safety_guard",
                            "patterns":["drift guard"],
                            "regex":["drift\\s+guard"],
                            "intent_tags":["safety"],
                            "signals":{"action_terms":["drift"],"subject_terms":["guard"],"object_terms":[]},
                            "min_signal_groups": 1
                        }]
                    }
                })),
                decision_input: Some(json!({
                    "objective":"Need drift guard policy",
                    "signature":"drift guard now",
                    "filters":["constraint_reframe"],
                    "intent_tags":["safety"]
                })),
            },
        );
        assert_eq!(detect.hits, vec!["safety_guard".to_string()]);

        let maturity = compute_maturity_score(&ComputeMaturityScoreInput {
            state: Some(json!({
                "stats": {
                    "total_tests": 20,
                    "passed_tests": 15,
                    "destructive_failures": 2
                }
            })),
            policy: Some(json!({
                "maturity": {
                    "target_test_count": 40,
                    "score_weights": {"pass_rate":0.5,"non_destructive_rate":0.3,"experience":0.2},
                    "bands": {"novice":0.25,"developing":0.45,"mature":0.65,"seasoned":0.82}
                }
            })),
        });
        assert_eq!(maturity.band, "seasoned".to_string());
        assert!((maturity.score - 0.745).abs() < 0.000001);

        let candidates = compute_select_library_candidates(&SelectLibraryCandidatesInput {
            policy: Some(json!({
                "library": {
                    "min_similarity_for_reuse": 0.2,
                    "token_weight": 0.6,
                    "trit_weight": 0.3,
                    "target_weight": 0.1
                }
            })),
            query: Some(json!({
                "signature_tokens":["drift","guard","stable"],
                "trit_vector":[-1],
                "target":"directive"
            })),
            file_path: Some(library_path.to_string_lossy().to_string()),
        });
        assert!(!candidates.candidates.is_empty());

        let lane = compute_parse_lane_decision(&ParseLaneDecisionInput {
            args: Some(json!({"brain_lane":"right"})),
        });
        assert_eq!(lane.selected_lane, "right".to_string());
        assert_eq!(lane.source, "arg".to_string());

