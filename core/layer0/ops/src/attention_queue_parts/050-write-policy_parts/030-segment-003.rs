        let ack_code = run(
            dir.path(),
            &[
                "ack".to_string(),
                "--consumer=cockpit".to_string(),
                "--through-index=0".to_string(),
                format!("--cursor-token={token}"),
            ],
        );
        assert_eq!(ack_code, 0);

        let cursor_state = load_cursor_state(&contract.cursor_state_path);
        assert_eq!(read_consumer_offset(&cursor_state, "cockpit"), 1);

        let drain_code = run(
            dir.path(),
            &[
                "drain".to_string(),
                "--consumer=cockpit".to_string(),
                "--limit=10".to_string(),
            ],
        );
        assert_eq!(drain_code, 0);

        let cursor_state_after = load_cursor_state(&contract.cursor_state_path);
        assert_eq!(read_consumer_offset(&cursor_state_after, "cockpit"), 2);
    }

    #[test]
    fn compact_trims_acked_prefix_and_rebases_offsets() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");

        for idx in 0..5 {
            let event = json!({
                "ts": now_iso(),
                "source": "dashboard_chat",
                "source_type": "chat_turn",
                "severity": "info",
                "summary": format!("compact-event-{idx}"),
                "attention_key": format!("compact-key-{idx}")
            });
            assert_eq!(enqueue_event(dir.path(), &event), 0);
        }

        let contract = load_contract(dir.path());
        let (rows, _) = load_active_queue(&contract);
        assert_eq!(rows.len(), 5);
        let token = cursor_token_for_event(&contract, "dashboard-cockpit", 3, &rows[3]);

        assert_eq!(
            run(
                dir.path(),
                &[
                    "ack".to_string(),
                    "--consumer=dashboard-cockpit".to_string(),
                    "--through-index=3".to_string(),
                    format!("--cursor-token={token}"),
                ],
            ),
            0
        );

        assert_eq!(
            run(
                dir.path(),
                &[
                    "compact".to_string(),
                    "--retain=1".to_string(),
                    "--min-acked=2".to_string(),
                ],
            ),
            0
        );

        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 2);

        let cursor_state =
            load_cursor_state(&dir.path().join("local/state/attention/cursor_state.json"));
        assert_eq!(read_consumer_offset(&cursor_state, "dashboard-cockpit"), 1);
    }

    #[test]
    fn status_reports_backpressure_threshold_contract() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 8, "warn");

        let event = json!({
            "ts": now_iso(),
            "source": "dashboard_chat",
            "source_type": "chat_turn",
            "severity": "info",
            "summary": "status backpressure",
            "attention_key": "status-bp"
        });
        assert_eq!(enqueue_event(dir.path(), &event), 0);
        let contract = load_contract(dir.path());
        assert_eq!(contract.backpressure_soft_watermark, 6);
        assert_eq!(contract.backpressure_hard_watermark, 8);
        let snapshot = backpressure_snapshot(6, &contract);
        assert_eq!(snapshot.get("level").and_then(Value::as_str), Some("high"));
        assert_eq!(
            snapshot
                .pointer("/thresholds/soft_watermark")
                .and_then(Value::as_u64),
            Some(6)
        );
        assert_eq!(
            snapshot
                .pointer("/thresholds/hard_watermark")
                .and_then(Value::as_u64),
            Some(8)
        );
    }

    #[test]
    fn ack_rejects_bad_token() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");
        let event = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "external_item",
            "severity": "warn",
            "summary": "first",
            "attention_key": "cursor-bad-token"
        });
        assert_eq!(enqueue_event(dir.path(), &event), 0);
        let code = run(
            dir.path(),
            &[
                "ack".to_string(),
                "--consumer=cockpit".to_string(),
                "--through-index=0".to_string(),
                "--cursor-token=invalid".to_string(),
            ],
        );
        assert_eq!(code, 2);

        let contract = load_contract(dir.path());
        let cursor_state = load_cursor_state(&contract.cursor_state_path);
        assert_eq!(read_consumer_offset(&cursor_state, "cockpit"), 0);
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 32,
            .. ProptestConfig::default()
        })]

        #[test]
        fn queue_depth_never_exceeds_contract_limit(
            max_depth in 1usize..12,
            info_events in 1usize..64
        ) {
            let dir = tempdir().expect("tempdir");
            write_policy(dir.path(), max_depth, "critical");
            for idx in 0..info_events {
                let event = json!({
                    "ts": now_iso(),
                    "source": "proptest",
                    "source_type": "queue_depth",
                    "severity": "info",
                    "summary": format!("queue-depth-{idx}"),
                    "attention_key": format!("prop-depth-{idx}")
                });
                let _ = enqueue_event(dir.path(), &event);
            }
            let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
            prop_assert!(queue.len() <= max_depth);
        }

