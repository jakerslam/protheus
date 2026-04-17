mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn routing_policy_requires_signature_on_update() {
        let root = tempfile::tempdir().expect("tempdir");
        let rejected = update_routing_policy(root.path(), &json!({"mode":"simulation"}));
        assert_eq!(
            rejected.get("ok").and_then(Value::as_bool),
            Some(false),
            "unsigned routing updates must fail closed"
        );
        assert_eq!(
            rejected.get("error").and_then(Value::as_str),
            Some("routing_policy_signature_required")
        );
    }

    #[test]
    fn routing_policy_update_and_fallback_chain_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let updated = update_routing_policy(
            root.path(),
            &json!({
                "signature": "sig:test-routing-v1",
                "retry": {"max_attempts_per_route": 3, "max_total_attempts": 6},
                "fallback_chain": [
                    {"provider":"moonshot","model":"kimi-k2.5"},
                    {"provider":"openrouter","model":"deepseek/deepseek-chat-v3-0324:free"}
                ]
            }),
        );
        assert_eq!(updated.get("ok").and_then(Value::as_bool), Some(true));
        let chain = routing_fallback_chain(root.path(), "openai", "gpt-5");
        assert!(chain.len() >= 3);
        assert_eq!(
            chain[0].get("provider").and_then(Value::as_str),
            Some("openai")
        );
        assert_eq!(
            chain[1].get("provider").and_then(Value::as_str),
            Some("moonshot")
        );
    }

    #[test]
    fn virtual_key_budget_is_fail_closed() {
        let root = tempfile::tempdir().expect("tempdir");
        let upsert = upsert_virtual_key(
            root.path(),
            "team-alpha",
            &json!({
                "provider": "openai",
                "model": "gpt-5",
                "team_id": "alpha",
                "budget_limit_usd": 0.000001,
                "rate_limit_rpm": 100
            }),
        );
        assert_eq!(upsert.get("ok").and_then(Value::as_bool), Some(true));
        let reserve = reserve_virtual_key_slot(root.path(), "team-alpha");
        assert_eq!(reserve.get("ok").and_then(Value::as_bool), Some(true));
        let spend = record_virtual_key_usage(root.path(), "team-alpha", 0.01);
        assert_eq!(spend.get("ok").and_then(Value::as_bool), Some(true));
        let blocked = reserve_virtual_key_slot(root.path(), "team-alpha");
        assert_eq!(blocked.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            blocked.get("error").and_then(Value::as_str),
            Some("virtual_key_budget_exceeded")
        );
    }

    #[test]
    fn invoke_chat_emits_routing_trace_and_cost_estimate() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = invoke_chat(
            root.path(),
            "openai",
            "gpt-5",
            "You are a helper",
            &[],
            "hello",
        )
        .expect("invoke chat");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(out
            .get("routing_trace")
            .and_then(|row| row.get("attempts"))
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("cost_usd")
            .and_then(Value::as_f64)
            .map(|value| value >= 0.0)
            .unwrap_or(false));
        assert!(out
            .get("prompt_optimization")
            .and_then(|row| row.get("cache_control"))
            .and_then(|row| row.get("lane"))
            .and_then(Value::as_str)
            .map(|lane| !lane.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn optimize_prompt_request_tracks_cache_hits_and_context_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut rows = Vec::<Value>::new();
        for idx in 0..20usize {
            rows.push(json!({
                "role": if idx % 2 == 0 { "user" } else { "assistant" },
                "text": format!("long conversation turn {idx}: {}", "detail ".repeat(40))
            }));
        }
        let first = optimize_prompt_request(
            root.path(),
            "openai",
            "gpt-5",
            "You are helpful.",
            &rows,
            "Return JSON schema for concise cache test output.",
        );
        let second = optimize_prompt_request(
            root.path(),
            "openai",
            "gpt-5",
            "You are helpful.",
            &rows,
            "Return JSON schema for concise cache test output.",
        );
        assert_eq!(
            first
                .metadata
                .pointer("/cache_control/cache_hit")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            second
                .metadata
                .pointer("/cache_control/cache_hit")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            second
                .metadata
                .pointer("/context/summary_applied")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            second
                .metadata
                .pointer("/output_contract/type")
                .and_then(Value::as_str),
            Some("json")
        );
        assert_eq!(second.assistant_prefill, "{");
    }

    #[test]
    fn ensure_model_profile_backfills_metadata_for_new_model_ref() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = ensure_model_profile(root.path(), "moonshot", "kimi-k2.5-preview");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let profile = out.get("profile").cloned().unwrap_or_else(|| json!({}));
        assert!(
            profile
                .get("context_window")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 131_072
        );
        assert!(
            profile
                .get("max_output_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0
        );
    }

    #[test]
    fn ensure_model_profile_keeps_openrouter_namespaced_model_ids() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = ensure_model_profile(root.path(), "openrouter", "moonshotai/kimi-k2.5");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("model").and_then(Value::as_str), Some("moonshotai/kimi-k2.5"));
    }

    #[test]
    fn discover_models_auto_mode_returns_summary_payload() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = discover_models(root.path(), "__auto__");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("input_kind").and_then(Value::as_str),
            Some("auto_discovery")
        );
        assert!(
            out.get("model_count")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn missing_model_errors_are_not_retryable() {
        assert!(!is_retryable_model_error(
            "model backend unavailable: model 'llama3.2:latest' not found"
        ));
        assert!(!is_retryable_model_error("ollama: no such model"));
    }

    #[test]
    fn fallback_chain_skips_unavailable_local_models() {
        let root = tempfile::tempdir().expect("tempdir");
        let updated = update_routing_policy(
            root.path(),
            &json!({
                "signature": "sig:test-local-fallback-skip",
                "fallback_chain": [
                    {"provider":"ollama","model":"definitely-missing-local-model-xyz"}
                ]
            }),
        );
        assert_eq!(updated.get("ok").and_then(Value::as_bool), Some(true));
        let chain = routing_fallback_chain(root.path(), "openai", "gpt-5");
        let has_missing_local = chain.iter().any(|row| {
            row.get("provider").and_then(Value::as_str) == Some("ollama")
                && row.get("model").and_then(Value::as_str)
                    == Some("definitely-missing-local-model-xyz")
        });
        assert!(
            !has_missing_local,
            "unavailable local fallback models should be filtered out"
        );
    }

    #[test]
    fn invoke_chat_auto_route_emits_decision_and_inference_receipt() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = add_custom_model(root.path(), "openai", "gpt-4o-mini", 128_000, 4_096);
        let _ = save_provider_key(root.path(), "openai", "sk-test-openai");
        let out = invoke_chat(
            root.path(),
            "auto",
            "auto",
            "You are a coding assistant.",
            &[],
            "Write a concise code review checklist.",
        )
        .expect("auto route invoke");
        assert!(
            out.get("provider")
                .and_then(Value::as_str)
                .map(|row| !row.is_empty() && row != "auto")
                .unwrap_or(false),
            "auto route should resolve a concrete provider"
        );
        assert!(
            out.get("model")
                .and_then(Value::as_str)
                .map(|row| !row.is_empty() && row != "auto")
                .unwrap_or(false),
            "auto route should resolve a concrete model"
        );
        assert!(
            out.get("auto_route_decision")
                .and_then(Value::as_object)
                .is_some(),
            "resolved auto route decision should be visible in response"
        );
        assert!(
            out.get("response_hash")
                .and_then(Value::as_str)
                .map(|row| !row.is_empty())
                .unwrap_or(false),
            "response hash should be attached for deterministic receipts"
        );
        let receipts = fs::read_to_string(root.path().join(PROVIDER_INFERENCE_RECEIPTS_REL))
            .expect("provider inference receipts");
        assert!(
            receipts.contains("\"type\":\"infring_provider_inference_receipt\"")
                && receipts.contains("\"provider\"")
                && receipts.contains("\"response_hash\""),
            "inference receipts should be persisted with provider and response hash fields"
        );
    }
}
