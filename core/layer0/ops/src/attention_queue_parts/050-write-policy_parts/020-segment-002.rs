        let receipts = read_jsonl(&dir.path().join("local/state/attention/receipts.jsonl"));
        let last = receipts.last().expect("last receipt");
        assert_eq!(
            last.get("decision").and_then(Value::as_str),
            Some("dropped_backpressure")
        );
        assert_eq!(
            last.get("backpressure_policy_action").and_then(Value::as_str),
            Some("drop")
        );
        assert_eq!(
            last.get("backpressure_threshold_soft").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            last.get("backpressure_threshold_hard").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            last.get("backpressure_level_before").and_then(Value::as_str),
            Some("critical")
        );
    }

    #[test]
    fn enqueue_orders_queue_by_band_then_priority_then_score() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");
        let low = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "external_item",
            "severity": "info",
            "summary": "low priority item",
            "attention_key": "prio-low"
        });
        let mid = json!({
            "ts": now_iso(),
            "source": "memory_ambient",
            "source_type": "memory_event",
            "severity": "warn",
            "summary": "mid priority item",
            "attention_key": "prio-mid",
            "importance": {
                "score": 0.74
            }
        });
        let high = json!({
            "ts": now_iso(),
            "source": "spine",
            "source_type": "infra_outage_state",
            "severity": "critical",
            "summary": "conduit bridge timeout degraded",
            "attention_key": "prio-high"
        });
        assert_eq!(enqueue_event(dir.path(), &low), 0);
        assert_eq!(enqueue_event(dir.path(), &mid), 0);
        assert_eq!(enqueue_event(dir.path(), &high), 0);

        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 3);
        assert_eq!(
            queue[0].get("attention_key").and_then(Value::as_str),
            Some("prio-high")
        );
        assert_eq!(
            queue[1].get("attention_key").and_then(Value::as_str),
            Some("prio-mid")
        );
        assert_eq!(
            queue[2].get("attention_key").and_then(Value::as_str),
            Some("prio-low")
        );
    }

    #[test]
    fn enqueue_assigns_tiered_queue_lanes() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");
        let background = json!({
            "ts": now_iso(),
            "source": "ops_logs",
            "source_type": "receipt_timeline",
            "severity": "info",
            "summary": "routine timeline heartbeat",
            "attention_key": "lane-background"
        });
        let critical = json!({
            "ts": now_iso(),
            "source": "security",
            "source_type": "integrity_fault",
            "severity": "critical",
            "summary": "critical policy failure",
            "attention_key": "lane-critical"
        });
        assert_eq!(enqueue_event(dir.path(), &background), 0);
        assert_eq!(enqueue_event(dir.path(), &critical), 0);
        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 2);
        assert_eq!(
            queue[0].get("queue_lane").and_then(Value::as_str),
            Some("critical")
        );
        assert_eq!(
            queue[1].get("queue_lane").and_then(Value::as_str),
            Some("background")
        );
    }

    #[test]
    fn enqueue_attaches_importance_and_initiative_metadata() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");
        let event = json!({
            "ts": now_iso(),
            "source": "security",
            "source_type": "integrity_fault",
            "severity": "critical",
            "summary": "security_global_gate_failed conduit timeout",
            "attention_key": "importance-meta"
        });
        assert_eq!(enqueue_event(dir.path(), &event), 0);
        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 1);
        let row = &queue[0];
        let score = row.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        assert!(score >= 0.95);
        assert_eq!(row.get("band").and_then(Value::as_str), Some("p0"));
        assert_eq!(
            row.get("initiative_action").and_then(Value::as_str),
            Some("persistent_until_ack")
        );
        assert!(row.get("importance").map(Value::is_object).unwrap_or(false));
    }

    #[test]
    fn next_ack_and_drain_progress_cursor() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");

        let first = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "external_item",
            "severity": "warn",
            "summary": "first",
            "attention_key": "cursor-1"
        });
        let second = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "eye_run_failed",
            "severity": "critical",
            "summary": "second",
            "attention_key": "cursor-2"
        });
        assert_eq!(enqueue_event(dir.path(), &first), 0);
        assert_eq!(enqueue_event(dir.path(), &second), 0);

        let next_code = run(
            dir.path(),
            &[
                "next".to_string(),
                "--consumer=cockpit".to_string(),
                "--limit=1".to_string(),
            ],
        );
        assert_eq!(next_code, 0);

        let contract = load_contract(dir.path());
        let queue = read_jsonl(&contract.queue_path);
        assert_eq!(queue.len(), 2);
        let token = cursor_token_for_event(&contract, "cockpit", 0, &queue[0]);

