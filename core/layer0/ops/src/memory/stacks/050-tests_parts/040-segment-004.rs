    #[test]
    fn node_spike_backpressure_stays_bounded_and_never_drops_critical() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[("stack-id", "pressure"), ("system-prompt", "system"), ("stable-nodes", "a,b")],
            ),
        );
        assert_true_key(&create, "ok");
        for _ in 0..2 {
            let out = context_stacks_node_spike(
                tmp.path(),
                &parsed(
                    &["node-spike"],
                    &[
                        ("stack-id", "pressure"),
                        ("node-id", "n-pressure"),
                        ("delta", "0.95"),
                        ("external-trigger", "true"),
                        ("queue-limit", "2"),
                    ],
                ),
            );
            assert_true_key(&out, "ok");
        }
        let overflow = context_stacks_node_spike(
            tmp.path(),
            &parsed(
                &["node-spike"],
                &[
                    ("stack-id", "pressure"),
                    ("node-id", "n-pressure"),
                    ("delta", "0.95"),
                    ("external-trigger", "true"),
                    ("queue-limit", "2"),
                ],
            ),
        );
        assert_true_key(&overflow, "ok");
        let queue_depth = overflow
            .pointer("/queue/depth_after")
            .and_then(Value::as_u64)
            .unwrap_or(99);
        assert!(queue_depth <= 2);
        let critical_dropped = overflow
            .pointer("/metrics/critical_dropped")
            .and_then(Value::as_u64)
            .unwrap_or(1);
        assert_eq!(critical_dropped, 0);
        let critical_journaled = overflow
            .pointer("/metrics/critical_journaled")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert!(critical_journaled >= 1, "critical overflow should journal instead of dropping");
    }

    #[test]
    fn contract_verify_enforces_snapshot_provider_batch_and_scheduler_contracts() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[
                    ("stack-id", "contract"),
                    ("system-prompt", "system"),
                    ("stable-nodes", "alpha,beta"),
                ],
            ),
        );
        assert_true_key(&create, "ok");
        let out = context_stacks_contract_verify(
            tmp.path(),
            &parsed(&["contract-verify"], &[("stack-id", "contract")]),
        );
        assert_true_key(&out, "ok");
        assert_eq!(
            out.pointer("/contracts/semantic_snapshot_stable_head_contract_ok")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/contracts/provider_snapshot_disposable_contract_ok")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/contracts/render_fingerprint_mode_contract_ok")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/contracts/strict_two_lane_batch_contract_ok")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/contracts/scheduler_cache_edge_contract_ok")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/contracts/manifest_active_delta_tail_contract_ok")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/contracts/delta_tail_typed_merge_contract_ok")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/contracts/delta_tail_promotion_contract_ok")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

