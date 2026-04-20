        state.byzantine_test_mode = true;
        let ok = spawn_single(&mut state, None, "task", 5, &options)
            .expect("byzantine should pass in test mode");
        assert_eq!(
            ok.get("report")
                .and_then(|v| v.get("corrupted"))
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn detailed_metrics_emit_breakdown_fields() {
        let mut state = SwarmState::default();
        let options = spawn_options();
        let ok = spawn_single(&mut state, None, "task", 5, &options).expect("spawn ok");
        let metrics = ok.get("metrics").cloned().unwrap_or(Value::Null);
        assert!(metrics.get("queue_wait_ms").is_some());
        assert!(metrics.get("execution_time_ms").is_some());
        assert!(metrics.get("total_latency_ms").is_some());
    }

    #[test]
    fn token_budget_fail_hard_enforced() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.token_budget = Some(100);
        options.budget_exhaustion_action = BudgetAction::FailHard;
        let err = spawn_single(
            &mut state,
            None,
            "write detailed and exhaustive analysis with examples and references",
            5,
            &options,
        )
        .expect_err("budget should hard fail");
        assert!(
            err.contains("token_budget_exceeded"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn adaptive_task_scaling_applies_for_small_budget() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.token_budget = Some(200);
        options.adaptive_complexity = true;
        options.budget_exhaustion_action = BudgetAction::AllowWithWarning;

        let ok = spawn_single(&mut state, None, "Analyze file", 5, &options).expect("spawn ok");
        let report_task = ok
            .get("report")
            .and_then(|value| value.get("task"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            report_task.contains("ultra-concise"),
            "expected scaled task annotation, got: {report_task}"
        );
    }

    #[test]
    fn sessions_send_allows_sibling_delivery() {
        let mut state = SwarmState::default();
        let parent = spawn_single(&mut state, None, "parent", 8, &spawn_options())
            .expect("parent spawn")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("parent id")
            .to_string();
        let child_a = spawn_single(&mut state, Some(&parent), "child-a", 8, &spawn_options())
            .expect("child-a")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("child-a id")
            .to_string();
        let child_b = spawn_single(&mut state, Some(&parent), "child-b", 8, &spawn_options())
            .expect("child-b")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("child-b id")
            .to_string();

        let delivered = send_session_message(
            &mut state,
            &child_a,
            &child_b,
            "hello",
            DeliveryGuarantee::AtMostOnce,
            false,
            DEFAULT_MESSAGE_TTL_MS,
        )
        .expect("sibling message should be delivered");
        assert_eq!(
            delivered
                .get("recipient_session_id")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            Some(child_b.clone())
        );
        let inbox = state.mailboxes.get(&child_b).expect("mailbox").unread.len();
        assert_eq!(inbox, 1);
    }

    #[test]
    fn exactly_once_dedupes_repeat_messages() {
        let mut state = SwarmState::default();
        let a = spawn_single(&mut state, None, "sender", 8, &spawn_options())
            .expect("sender spawn")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("sender id")
            .to_string();
        let b = spawn_single(&mut state, None, "receiver", 8, &spawn_options())
            .expect("receiver spawn")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("receiver id")
            .to_string();

        let first = send_session_message(
            &mut state,
            "coordinator",
            &b,
            "dedupe-message",
            DeliveryGuarantee::ExactlyOnce,
            false,
            DEFAULT_MESSAGE_TTL_MS,
        )
        .expect("first send");
        let second = send_session_message(
            &mut state,
            "coordinator",
            &b,
            "dedupe-message",
            DeliveryGuarantee::ExactlyOnce,
            false,
            DEFAULT_MESSAGE_TTL_MS,
        )
        .expect("second send");
        assert_eq!(
            second.get("dedupe_hit").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            first.get("message_id").and_then(Value::as_str),
            second.get("message_id").and_then(Value::as_str)
        );
        let inbox_len = state.mailboxes.get(&b).expect("mailbox").unread.len();
        assert_eq!(inbox_len, 1, "exactly-once must avoid duplicate inbox rows");
        assert!(state.sessions.contains_key(&a));
    }

    #[test]
    fn wildcard_label_match_supports_prefix_suffix_patterns() {
        assert!(wildcard_matches(
            "swarm-test-7-*",
            "swarm-test-7-agent-fast"
        ));
        assert!(wildcard_matches("*agent-fast", "swarm-test-7-agent-fast"));
        assert!(!wildcard_matches(
            "swarm-test-8-*",
            "swarm-test-7-agent-fast"
        ));
    }

    #[test]
    fn auto_publish_results_are_queryable_and_consensus_detects_mismatch() {
        let mut state = SwarmState::default();
        let mut first = spawn_options();
        first.auto_publish_results = true;
        first.result_value = Some(5050.0);
        first.agent_label = Some("swarm-test-7-het-agent-fast".to_string());
        first.role = Some("calculator".to_string());
        spawn_single(&mut state, None, "calc-fast", 8, &first).expect("first spawn");

