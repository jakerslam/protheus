    #[test]
    fn hybrid_retrieve_combines_vector_and_edge_scores() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[
                    ("stack-id", "hybrid"),
                    ("system-prompt", "system"),
                    ("stable-nodes", "r1,r2"),
                ],
            ),
        );
        assert_true_key(&create, "ok");
        let out = context_stacks_hybrid_retrieve(
            tmp.path(),
            &parsed(
                &["hybrid-retrieve"],
                &[
                    ("stack-id", "hybrid"),
                    ("query", "find strongest relation"),
                    (
                        "vector-json",
                        "[{\"id\":\"a\",\"score\":0.90},{\"id\":\"b\",\"score\":0.55}]",
                    ),
                    (
                        "edges-json",
                        "[{\"id\":\"a\",\"edge_confidence\":0.20},{\"id\":\"b\",\"edge_confidence\":0.95}]",
                    ),
                    ("top-k", "2"),
                ],
            ),
        );
        assert_true_key(&out, "ok");
        let results = out
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(results.len(), 2);
        let top_id = results
            .first()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert_eq!(top_id, "b");
    }

    #[test]
    fn partial_merge_records_post_merge_feedback_ledger() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[
                    ("stack-id", "feedback"),
                    ("system-prompt", "system"),
                    ("stable-nodes", "a,b"),
                ],
            ),
        );
        assert_true_key(&create, "ok");
        let out = context_stacks_partial_merge(
            tmp.path(),
            &parsed(
                &["partial-merge"],
                &[
                    ("stack-id", "feedback"),
                    ("family", "ingestion_memory"),
                    ("throughput-delta", "0.12"),
                    ("memory-delta", "-0.04"),
                    ("stability-delta", "0.03"),
                    ("error-rate-delta", "-0.02"),
                    (
                        "patch-json",
                        "{\"stable_nodes_add\":[\"c\"],\"volatile_metadata_patch\":{\"stage\":\"feedback\"}}",
                    ),
                ],
            ),
        );
        assert_true_key(&out, "ok");
        assert!(out
            .get("merge_feedback_event_id")
            .and_then(Value::as_str)
            .map(|row| !row.is_empty())
            .unwrap_or(false));
        let ledger = out
            .get("skill_performance_ledger")
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(ledger.get("samples").and_then(Value::as_u64), Some(1));
        let state = load_context_stacks_state(tmp.path());
        assert_eq!(state.merge_feedback_events.len(), 1);
        assert!(state.skill_performance_ledger.contains_key("ingestion_memory"));
    }

    #[test]
    fn node_spike_is_sparse_and_fires_only_when_threshold_crossed() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[("stack-id", "spike"), ("system-prompt", "system"), ("stable-nodes", "a,b")],
            ),
        );
        assert_true_key(&create, "ok");
        let quiet = context_stacks_node_spike(
            tmp.path(),
            &parsed(
                &["node-spike"],
                &[("stack-id", "spike"), ("node-id", "n1"), ("delta", "0.05"), ("staleness-seconds", "30")],
            ),
        );
        assert_true_key(&quiet, "ok");
        assert_eq!(quiet.get("should_fire").and_then(Value::as_bool), Some(false));
        let loud = context_stacks_node_spike(
            tmp.path(),
            &parsed(
                &["node-spike"],
                &[("stack-id", "spike"), ("node-id", "n1"), ("delta", "0.95"), ("staleness-seconds", "3600")],
            ),
        );
        assert_true_key(&loud, "ok");
        assert_eq!(loud.get("should_fire").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn node_spike_threshold_adapts_with_load_and_success_feedback() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[("stack-id", "adaptive"), ("system-prompt", "system"), ("stable-nodes", "a,b")],
            ),
        );
        assert_true_key(&create, "ok");
        let high_load = context_stacks_node_spike(
            tmp.path(),
            &parsed(
                &["node-spike"],
                &[
                    ("stack-id", "adaptive"),
                    ("node-id", "n-adapt"),
                    ("delta", "0.6"),
                    ("load-signal", "0.95"),
                    ("success-signal", "0.15"),
                ],
            ),
        );
        assert_true_key(&high_load, "ok");
        let t1 = high_load
            .get("threshold_after")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let low_load = context_stacks_node_spike(
            tmp.path(),
            &parsed(
                &["node-spike"],
                &[
                    ("stack-id", "adaptive"),
                    ("node-id", "n-adapt"),
                    ("delta", "0.6"),
                    ("load-signal", "0.05"),
                    ("success-signal", "0.95"),
                ],
            ),
        );
        assert_true_key(&low_load, "ok");
        let t2 = low_load
            .get("threshold_after")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        assert!(t1 > t2, "threshold should adapt downward when load drops and success rises");
    }

