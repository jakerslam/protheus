        let pushed = compute_push_tier_event(&PushTierEventInput {
            scope_map: Some(json!({"tactical": []})),
            target: Some("directive".to_string()),
            ts: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert_eq!(
            pushed
                .map
                .as_object()
                .and_then(|m| m.get("directive"))
                .and_then(|v| v.as_array())
                .map(|rows| rows.len())
                .unwrap_or(0),
            1
        );

        let added = compute_add_tier_event(&AddTierEventInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            metric: Some("live_apply_attempts".to_string()),
            target: Some("belief".to_string()),
            ts: Some("2026-03-04T12:00:00.000Z".to_string()),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert!(value_path(
            Some(&added.state),
            &["scopes", "1.7", "live_apply_attempts", "belief"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let inc_attempt = compute_increment_live_apply_attempt(&IncrementLiveApplyAttemptInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            target: Some("identity".to_string()),
            now_iso: Some("2026-03-04T12:02:00.000Z".to_string()),
        });
        assert!(value_path(
            Some(&inc_attempt.state),
            &["scopes", "1.7", "live_apply_attempts", "identity"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let inc_success = compute_increment_live_apply_success(&IncrementLiveApplySuccessInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            target: Some("identity".to_string()),
            now_iso: Some("2026-03-04T12:03:00.000Z".to_string()),
        });
        assert!(value_path(
            Some(&inc_success.state),
            &["scopes", "1.7", "live_apply_successes", "identity"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let inc_abort =
            compute_increment_live_apply_safe_abort(&IncrementLiveApplySafeAbortInput {
                file_path: Some(tier_path.to_string_lossy().to_string()),
                policy: Some(policy.clone()),
                target: Some("identity".to_string()),
                now_iso: Some("2026-03-04T12:04:00.000Z".to_string()),
            });
        assert!(value_path(
            Some(&inc_abort.state),
            &["scopes", "1.7", "live_apply_safe_aborts", "identity"]
        )
        .and_then(|v| v.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

        let shadow = compute_update_shadow_trial_counters(&UpdateShadowTrialCountersInput {
            file_path: Some(tier_path.to_string_lossy().to_string()),
            policy: Some(policy.clone()),
            session: Some(json!({"mode":"test","apply_requested": false,"target":"directive"})),
            result: Some("success".to_string()),
            destructive: Some(false),
            now_iso: Some("2026-03-04T12:05:00.000Z".to_string()),
        });
        assert!(shadow.state.is_some());

        let upsert = compute_upsert_first_principle_lock(&UpsertFirstPrincipleLockInput {
            file_path: Some(lock_path.to_string_lossy().to_string()),
            session: Some(json!({
                "objective_id":"BL-246",
                "objective":"Guard principle quality",
                "target":"directive",
                "maturity_band":"mature"
            })),
            principle: Some(json!({"id":"fp_guard","confidence":0.91})),
            now_iso: Some("2026-03-04T12:06:00.000Z".to_string()),
        });
        assert!(value_path(Some(&upsert.state), &["locks", upsert.key.as_str()]).is_some());

        let check = compute_check_first_principle_downgrade(&CheckFirstPrincipleDowngradeInput {
            file_path: Some(lock_path.to_string_lossy().to_string()),
            policy: Some(policy),
            session: Some(json!({
                "objective_id":"BL-246",
                "objective":"Guard principle quality",
                "target":"directive",
                "maturity_band":"developing"
            })),
            confidence: Some(0.5),
            now_iso: Some("2026-03-04T12:07:00.000Z".to_string()),
        });
        assert!(!check.allowed);
        assert_eq!(
            check.reason.as_deref().unwrap_or(""),
            "first_principle_downgrade_blocked_lower_maturity"
        );
    }

    #[test]
    fn helper_primitives_batch17_match_contract() {
        let temp_root = std::env::temp_dir().join("inv_batch17");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(temp_root.join("events"));
        let _ = fs::create_dir_all(temp_root.join("simulation"));
        let _ = fs::create_dir_all(temp_root.join("red_team"));

        let library_path = temp_root.join("library.jsonl");
        let receipts_path = temp_root.join("receipts.jsonl");
        let active_sessions_path = temp_root.join("active_sessions.json");
        let fp_latest_path = temp_root.join("first_principles_latest.json");
        let fp_history_path = temp_root.join("first_principles_history.jsonl");
        let fp_lock_path = temp_root.join("first_principles_lock.json");

