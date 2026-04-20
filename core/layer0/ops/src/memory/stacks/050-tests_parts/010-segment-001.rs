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

