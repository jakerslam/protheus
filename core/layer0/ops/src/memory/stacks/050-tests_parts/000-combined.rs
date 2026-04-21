mod tests {
    use super::*;

    fn parsed(positional: &[&str], flags: &[(&str, &str)]) -> crate::ParsedArgs {
        let mut out = crate::ParsedArgs {
            positional: positional.iter().map(|row| (*row).to_string()).collect(),
            flags: std::collections::HashMap::new(),
        };
        for (k, v) in flags {
            out.flags.insert((*k).to_string(), (*v).to_string());
        }
        out
    }

    fn assert_true_key(payload: &Value, key: &str) {
        assert_eq!(payload.get(key).and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn semantic_snapshot_id_ignores_volatile_metadata() {
        let head = StableHead {
            system_prompt: "system".to_string(),
            tools: vec!["web".to_string()],
            ordered_stable_nodes: vec!["alpha".to_string(), "beta".to_string()],
        };
        let id_a = semantic_snapshot_id_for(&head);
        let mut snapshot_b = SemanticSnapshot {
            semantic_snapshot_id: String::new(),
            stable_head: head.clone(),
            volatile_metadata: json!({"session":"one"}),
            created_at: now_iso(),
            updated_at: now_iso(),
        };
        snapshot_b.volatile_metadata = json!({"session":"two", "cursor": 42});
        let id_b = semantic_snapshot_id_for(&snapshot_b.stable_head);
        assert_eq!(id_a, id_b);
    }

    #[test]
    fn render_fingerprint_changes_for_provider_visible_fields() {
        let snapshot = SemanticSnapshot {
            semantic_snapshot_id: "semantic_test".to_string(),
            stable_head: StableHead {
                system_prompt: "system".to_string(),
                tools: vec!["web".to_string()],
                ordered_stable_nodes: vec!["n1".to_string()],
            },
            volatile_metadata: json!({}),
            created_at: now_iso(),
            updated_at: now_iso(),
        };
        let mut base = default_render_plan("semantic_test");
        base.provider = "openai".to_string();
        base.model = "gpt-5".to_string();
        let mut changed = base.clone();
        changed.tool_choice = "required".to_string();
        let fp_a = render_fingerprint_for(&snapshot, &base);
        let fp_b = render_fingerprint_for(&snapshot, &changed);
        assert_ne!(fp_a, fp_b);
    }

    #[test]
    fn batch_class_key_is_strictly_partitioned() {
        let plan = RenderPlan {
            render_plan_id: "plan".to_string(),
            provider: "openai".to_string(),
            model: "gpt-5".to_string(),
            tool_choice: "auto".to_string(),
            thinking_mode: "default".to_string(),
            image_presence: "none".to_string(),
            response_mode: "chat".to_string(),
            cache_policy: CachePolicy::Auto,
            ttl_class: "session".to_string(),
        };
        let a = normalize_batch_class(&plan, BatchLane::LiveMicrobatch, "render_a");
        let b = normalize_batch_class(&plan, BatchLane::ProviderBatch, "render_a");
        let mut c = b.clone();
        c.tool_choice = "required".to_string();
        assert_ne!(batch_class_id_for(&a), batch_class_id_for(&b));
        assert_ne!(batch_class_id_for(&b), batch_class_id_for(&c));
    }

    #[test]
    fn scheduler_below_threshold_disables_cache_counters() {
        let policy = default_context_stacks_policy();
        let decision = evaluate_scheduler_edge_cases(
            &policy,
            CachePolicy::Auto,
            policy.cache_threshold_tokens.saturating_sub(1),
            200,
            policy.lookback_window_tokens,
            1,
            false,
        );
        assert_eq!(decision.scheduler_mode, "no_cache");
        assert!(!decision.cache_hit);
        assert_eq!(decision.cache_creation_input_tokens, 0);
        assert_eq!(decision.cache_read_input_tokens, 0);
    }

    #[test]
    fn scheduler_seed_then_fanout_for_fresh_cohort() {
        let policy = default_context_stacks_policy();
        let decision = evaluate_scheduler_edge_cases(
            &policy,
            CachePolicy::Auto,
            policy.cache_threshold_tokens + 50,
            500,
            policy.lookback_window_tokens,
            policy.seed_then_fanout_min_cohort + 1,
            false,
        );
        assert_eq!(decision.scheduler_mode, "seed_then_fanout");
        assert!(decision.seed_then_fanout);
        assert_eq!(decision.cache_creation_input_tokens, 500);
    }

    #[test]
    fn explicit_breakpoint_recovers_cache_hit_beyond_lookback() {
        let policy = default_context_stacks_policy();
        let decision = evaluate_scheduler_edge_cases(
            &policy,
            CachePolicy::ExplicitBreakpoint,
            1400,
            policy.lookback_window_tokens + 1000,
            policy.lookback_window_tokens,
            1,
            true,
        );
        assert!(decision.cache_hit);
        assert_eq!(decision.breakpoint_mode.as_deref(), Some("explicit_breakpoint"));
    }

    #[test]
    fn tail_merge_and_promote_updates_snapshot_and_clears_tail() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[
                    ("stack-id", "demo"),
                    ("system-prompt", "system"),
                    ("stable-nodes", "a,b"),
                    ("objective", "ship reliable context stacks"),
                ],
            ),
        );
        assert_true_key(&create, "ok");
        let before = create
            .pointer("/stack/semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let merge = merge_context_stack_tail(
            tmp.path(),
            &parsed(
                &["tail-merge"],
                &[
                    ("stack-id", "demo"),
                    ("merge-type", "append_working_note"),
                    ("value", "Capture provider cache counters and merge outcome."),
                ],
            ),
        );
        assert_true_key(&merge, "ok");
        let promote = promote_context_stack_tail(
            tmp.path(),
            &parsed(&["tail-promote"], &[("stack-id", "demo")]),
        );
        assert_true_key(&promote, "ok");
        let after = promote
            .get("semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert_ne!(before, after);
        let state = load_context_stacks_state(tmp.path());
        let tail = state
            .delta_tails
            .iter()
            .find(|row| row.stack_id == "demo")
            .expect("tail");
        assert!(tail.entries.is_empty());
    }

    #[test]
    fn nexus_authorization_succeeds_for_context_stacks_route() {
        let out = authorize_context_stacks_command_with_nexus_inner("list", false)
            .expect("nexus auth");
        assert_true_key(&out, "enabled");
        assert!(out
            .get("lease_id")
            .and_then(Value::as_str)
            .map(|row| !row.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn nexus_authorization_fails_closed_when_context_route_blocked() {
        let err = authorize_context_stacks_command_with_nexus_inner("list", true)
            .err()
            .unwrap_or_else(|| "missing_error".to_string());
        assert!(err.contains("lease_denied") || err.contains("delivery_denied"));
    }

    #[test]
    fn taste_tune_updates_family_weight_with_guardrails() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = context_stacks_taste_tune(
            tmp.path(),
            &parsed(
                &["taste-tune"],
                &[("family", "memory_compaction"), ("merge-lift", "0.8")],
            ),
        );
        assert_true_key(&out, "ok");
        let next = out.get("next").and_then(Value::as_f64).unwrap_or(0.0);
        assert!(next >= 0.2 && next <= 2.0, "next taste weight out of bounds");
        let state = load_context_stacks_state(tmp.path());
        let stored = state
            .taste_vectors
            .get("memory_compaction")
            .copied()
            .unwrap_or(0.0);
        assert!((stored - next).abs() < 1e-9);
    }

    #[test]
    fn partial_merge_applies_only_changed_slices() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[
                    ("stack-id", "partial"),
                    ("system-prompt", "system"),
                    ("stable-nodes", "alpha,beta"),
                ],
            ),
        );
        assert_true_key(&create, "ok");
        let out = context_stacks_partial_merge(
            tmp.path(),
            &parsed(
                &["partial-merge"],
                &[
                    ("stack-id", "partial"),
                    (
                        "patch-json",
                        "{\"stable_nodes_add\":[\"gamma\"],\"volatile_metadata_patch\":{\"stage\":\"partial\"}}",
                    ),
                ],
            ),
        );
        assert_true_key(&out, "ok");
        assert_eq!(
            out.get("mode").and_then(Value::as_str),
            Some("diff_scoped_partial")
        );
        let changed = out
            .get("changed_slices")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(changed.iter().any(|row| row.as_str() == Some("stable_head")));
        let state = load_context_stacks_state(tmp.path());
        assert!(
            !state.partial_merge_events.is_empty(),
            "partial merge events should be recorded"
        );
    }

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

    #[test]
    fn speculative_overlay_start_isolated_until_verified_merge() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[
                    ("stack-id", "overlay"),
                    ("system-prompt", "system"),
                    ("stable-nodes", "alpha,beta"),
                ],
            ),
        );
        assert_true_key(&create, "ok");
        let base_semantic_snapshot_id = create
            .pointer("/stack/semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let started = context_stacks_speculative_start(
            tmp.path(),
            &parsed(
                &["speculative-start"],
                &[
                    ("stack-id", "overlay"),
                    (
                        "patch-json",
                        "{\"stable_nodes_add\":[\"candidate-gamma\"],\"volatile_metadata_patch\":{\"phase\":\"speculative\"}}",
                    ),
                ],
            ),
        );
        assert_true_key(&started, "ok");
        let overlay_id = started
            .pointer("/overlay/overlay_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(
            !overlay_id.is_empty(),
            "speculative-start should emit an overlay id"
        );
        let proposed_semantic_snapshot_id = started
            .pointer("/overlay/proposed_semantic_snapshot/semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert_ne!(
            proposed_semantic_snapshot_id, base_semantic_snapshot_id,
            "speculative proposal should diverge from base semantic snapshot"
        );
        let state = load_context_stacks_state(tmp.path());
        let manifest = state
            .manifests
            .iter()
            .find(|row| row.stack_id == "overlay")
            .expect("manifest");
        assert_eq!(
            manifest.semantic_snapshot_id, base_semantic_snapshot_id,
            "manifest must stay canonical until merge approval"
        );
        let overlay = state
            .speculative_overlays
            .iter()
            .find(|row| row.overlay_id == overlay_id)
            .expect("overlay");
        assert_eq!(overlay.status, "active");

        let status = context_stacks_speculative_status(
            tmp.path(),
            &parsed(
                &["speculative-status"],
                &[("stack-id", "overlay"), ("overlay-id", overlay_id.as_str())],
            ),
        );
        assert_true_key(&status, "ok");
        assert_eq!(status.get("overlay_count").and_then(Value::as_u64), Some(1));
        assert!(
            status
                .get("receipt_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1,
            "speculative status should expose disposable receipt stream"
        );
    }

    #[test]
    fn speculative_overlay_merge_gate_and_single_step_rollback_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[
                    ("stack-id", "rollback"),
                    ("system-prompt", "system"),
                    ("stable-nodes", "seed"),
                ],
            ),
        );
        assert_true_key(&create, "ok");
        let base_semantic_snapshot_id = create
            .pointer("/stack/semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let started = context_stacks_speculative_start(
            tmp.path(),
            &parsed(
                &["speculative-start"],
                &[
                    ("stack-id", "rollback"),
                    ("overlay-id", "rb1"),
                    ("patch-json", "{\"stable_nodes_add\":[\"candidate\"]}"),
                ],
            ),
        );
        assert_true_key(&started, "ok");

        let blocked_merge = context_stacks_speculative_merge(
            tmp.path(),
            &parsed(&["speculative-merge"], &[("overlay-id", "rb1")]),
        );
        assert_eq!(
            blocked_merge.get("ok").and_then(Value::as_bool),
            Some(false),
            "merge must fail closed without verity approval"
        );
        assert_eq!(
            blocked_merge.get("error").and_then(Value::as_str),
            Some("speculative_merge_approval_required")
        );

        let merged = context_stacks_speculative_merge(
            tmp.path(),
            &parsed(
                &["speculative-merge"],
                &[
                    ("overlay-id", "rb1"),
                    ("verify-merge", "true"),
                    ("approval-note", "approved by verity gate"),
                ],
            ),
        );
        assert_true_key(&merged, "ok");
        let merged_semantic_snapshot_id = merged
            .get("merged_semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert_ne!(
            merged_semantic_snapshot_id, base_semantic_snapshot_id,
            "approved merge should promote speculative snapshot"
        );

        let state_after_merge = load_context_stacks_state(tmp.path());
        let manifest_after_merge = state_after_merge
            .manifests
            .iter()
            .find(|row| row.stack_id == "rollback")
            .expect("manifest");
        assert_eq!(
            manifest_after_merge.semantic_snapshot_id, merged_semantic_snapshot_id,
            "manifest should move to merged speculative snapshot"
        );

        let rollback = context_stacks_speculative_rollback(
            tmp.path(),
            &parsed(
                &["speculative-rollback"],
                &[("overlay-id", "rb1"), ("reason", "destructive_failure_simulation")],
            ),
        );
        assert_true_key(&rollback, "ok");
        assert_eq!(
            rollback.get("rollback_semantic_snapshot_id").and_then(Value::as_str),
            Some(base_semantic_snapshot_id.as_str())
        );

        let state_after_rollback = load_context_stacks_state(tmp.path());
        let manifest_after_rollback = state_after_rollback
            .manifests
            .iter()
            .find(|row| row.stack_id == "rollback")
            .expect("manifest");
        assert_eq!(
            manifest_after_rollback.semantic_snapshot_id, base_semantic_snapshot_id,
            "rollback should restore pre-merge semantic snapshot in one step"
        );
        let overlay = state_after_rollback
            .speculative_overlays
            .iter()
            .find(|row| row.overlay_id == "rb1")
            .expect("overlay");
        assert_eq!(overlay.status, "rolled_back");
        assert!(
            state_after_rollback.speculative_overlay_receipts.len() >= 3,
            "start + merge + rollback should emit auditable receipts"
        );
    }
}
