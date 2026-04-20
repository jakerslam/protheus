use std::cmp::min;
use std::collections::BTreeSet;

fn parse_u64_flag(parsed: &crate::ParsedArgs, key: &str, default: u64) -> u64 {
    parsed
        .flags
        .get(key)
        .and_then(|raw| clean(raw, 40).parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_usize_flag(parsed: &crate::ParsedArgs, key: &str, default: usize) -> usize {
    parsed
        .flags
        .get(key)
        .and_then(|raw| clean(raw, 40).parse::<usize>().ok())
        .unwrap_or(default)
}

fn generate_id(prefix: &str) -> String {
    let payload = format!("{prefix}:{}:{}", now_iso(), std::process::id());
    let digest = sha256_hex(&payload);
    format!("{prefix}_{}", &digest[..24])
}

fn dedupe_preserving_order(rows: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::<String>::new();
    let mut out = Vec::<String>::new();
    for row in rows {
        if seen.insert(row.clone()) {
            out.push(row);
        }
    }
    out
}

fn parse_stable_nodes(parsed: &crate::ParsedArgs) -> Vec<String> {
    let from_json = parse_json_array(parsed.flags.get("stable-nodes-json"));
    if !from_json.is_empty() {
        return dedupe_preserving_order(from_json);
    }
    dedupe_preserving_order(parse_csv(parsed.flags.get("stable-nodes")))
}

fn build_stable_head(parsed: &crate::ParsedArgs) -> StableHead {
    let system_prompt = clean(
        parsed
            .flags
            .get("system-prompt")
            .or_else(|| parsed.flags.get("prompt"))
            .map(String::as_str)
            .unwrap_or("Context stack"),
        4000,
    );
    let tools = dedupe_preserving_order(parse_csv(parsed.flags.get("tools")));
    let ordered_stable_nodes = parse_stable_nodes(parsed);
    StableHead {
        system_prompt,
        tools,
        ordered_stable_nodes,
    }
}

fn build_render_plan(
    parsed: &crate::ParsedArgs,
    semantic_snapshot_id: &str,
    existing_plan: Option<&RenderPlan>,
) -> RenderPlan {
    let provider = clean(
        parsed
            .flags
            .get("provider")
            .map(String::as_str)
            .or_else(|| existing_plan.map(|row| row.provider.as_str()))
            .unwrap_or("default"),
        120,
    );
    let model = clean(
        parsed
            .flags
            .get("model")
            .map(String::as_str)
            .or_else(|| existing_plan.map(|row| row.model.as_str()))
            .unwrap_or("default"),
        160,
    );
    let tool_choice = clean(
        parsed
            .flags
            .get("tool-choice")
            .map(String::as_str)
            .or_else(|| existing_plan.map(|row| row.tool_choice.as_str()))
            .unwrap_or("auto"),
        120,
    );
    let thinking_mode = clean(
        parsed
            .flags
            .get("thinking-mode")
            .map(String::as_str)
            .or_else(|| existing_plan.map(|row| row.thinking_mode.as_str()))
            .unwrap_or("default"),
        120,
    );
    let image_presence = clean(
        parsed
            .flags
            .get("image-presence")
            .map(String::as_str)
            .or_else(|| existing_plan.map(|row| row.image_presence.as_str()))
            .unwrap_or("none"),
        120,
    );
    let response_mode = clean(
        parsed
            .flags
            .get("response-mode")
            .map(String::as_str)
            .or_else(|| existing_plan.map(|row| row.response_mode.as_str()))
            .unwrap_or("chat"),
        120,
    );
    let cache_policy = parsed
        .flags
        .get("cache-policy")
        .map(|raw| CachePolicy::from_raw(raw))
        .or_else(|| existing_plan.map(|row| row.cache_policy))
        .unwrap_or(CachePolicy::Auto);
    let ttl_class = clean(
        parsed
            .flags
            .get("ttl-class")
            .map(String::as_str)
            .or_else(|| existing_plan.map(|row| row.ttl_class.as_str()))
            .unwrap_or("session"),
        120,
    );
    let payload = json!({
        "semantic_snapshot_id": semantic_snapshot_id,
        "provider": provider,
        "model": model,
        "tool_choice": tool_choice,
        "thinking_mode": thinking_mode,
        "image_presence": image_presence,
        "response_mode": response_mode,
        "cache_policy": cache_policy.as_str(),
        "ttl_class": ttl_class
    });
    let render_plan_id = format!(
        "render_plan_{}",
        &sha256_hex(&serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()))[..24]
    );
    RenderPlan {
        render_plan_id,
        provider,
        model,
        tool_choice,
        thinking_mode,
        image_presence,
        response_mode,
        cache_policy,
        ttl_class,
    }
}

fn derive_provider_snapshot(snapshot: &SemanticSnapshot, plan: &RenderPlan) -> ProviderSnapshot {
    ProviderSnapshot {
        render_fingerprint: render_fingerprint_for(snapshot, plan),
        semantic_snapshot_id: snapshot.semantic_snapshot_id.clone(),
        render_plan_id: plan.render_plan_id.clone(),
        provider: plan.provider.clone(),
        model: plan.model.clone(),
        serialized_prefix: provider_snapshot_serialized_prefix(snapshot, plan),
        derived_disposable: true,
        created_at: now_iso(),
    }
}

fn provider_snapshot_token_estimate(prefix: &str) -> u64 {
    let words = prefix.split_whitespace().count() as u64;
    words.max(1)
}

fn find_manifest_index(state: &ContextStacksState, stack_id: &str) -> Option<usize> {
    state
        .manifests
        .iter()
        .position(|row| row.stack_id == stack_id)
}

fn find_semantic_snapshot<'a>(
    state: &'a ContextStacksState,
    semantic_snapshot_id: &str,
) -> Option<&'a SemanticSnapshot> {
    state
        .semantic_snapshots
        .iter()
        .find(|row| row.semantic_snapshot_id == semantic_snapshot_id)
}

fn find_tail_index(state: &ContextStacksState, tail_id: &str) -> Option<usize> {
    state.delta_tails.iter().position(|row| row.tail_id == tail_id)
}

fn find_overlay_index(state: &ContextStacksState, overlay_id: &str) -> Option<usize> {
    state
        .speculative_overlays
        .iter()
        .position(|row| row.overlay_id == overlay_id)
}

fn receipt_with_common_fields(
    kind: &str,
    stack_id: &str,
    merge_outcome: &str,
    batch_id: Option<String>,
    decision: Option<&SchedulerEdgeCaseDecision>,
) -> Value {
    let mut payload = json!({
        "type": "context_stacks_receipt",
        "kind": kind,
        "ts": now_iso(),
        "stack_id": clean(stack_id, 120),
        "merge_outcome": clean(merge_outcome, 160),
        "cache_hit": false,
        "cache_creation_input_tokens": 0u64,
        "cache_read_input_tokens": 0u64,
        "batch_id": batch_id.unwrap_or_default(),
    });
    if let Some(row) = decision {
        payload["cache_hit"] = Value::Bool(row.cache_hit);
        payload["cache_creation_input_tokens"] = Value::from(row.cache_creation_input_tokens);
        payload["cache_read_input_tokens"] = Value::from(row.cache_read_input_tokens);
    }
    let receipt_id = receipt_hash(&payload);
    payload["receipt_id"] = Value::String(receipt_id);
    payload
}

fn evaluate_scheduler_edge_cases(
    policy: &ContextStacksPolicy,
    cache_policy: CachePolicy,
    prompt_tokens: u64,
    stable_prefix_tokens: u64,
    lookback_window_tokens: u64,
    fresh_cohort_size: usize,
    has_cached_provider_snapshot: bool,
) -> SchedulerEdgeCaseDecision {
    if cache_policy == CachePolicy::NoCache || prompt_tokens < policy.cache_threshold_tokens {
        return SchedulerEdgeCaseDecision {
            scheduler_mode: "no_cache".to_string(),
            cache_hit: false,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            seed_then_fanout: false,
            breakpoint_mode: None,
        };
    }

    let seed_then_fanout =
        !has_cached_provider_snapshot && fresh_cohort_size >= policy.seed_then_fanout_min_cohort;

    let breakpoint_mode = match cache_policy {
        CachePolicy::ExplicitBreakpoint => Some("explicit_breakpoint".to_string()),
        CachePolicy::MultiBreakpoint if policy.allow_multi_breakpoint => {
            Some("multi_breakpoint".to_string())
        }
        _ => None,
    };

    let outside_lookback = stable_prefix_tokens > lookback_window_tokens.max(1);
    let cache_hit = has_cached_provider_snapshot && (!outside_lookback || breakpoint_mode.is_some());
    let cache_creation_input_tokens = if seed_then_fanout || !has_cached_provider_snapshot {
        stable_prefix_tokens
    } else {
        0
    };
    let cache_read_input_tokens = if cache_hit {
        min(stable_prefix_tokens, prompt_tokens)
    } else {
        0
    };

    SchedulerEdgeCaseDecision {
        scheduler_mode: if seed_then_fanout {
            "seed_then_fanout".to_string()
        } else if outside_lookback && breakpoint_mode.is_some() {
            "breakpoint_isolated".to_string()
        } else {
            "single_shot".to_string()
        },
        cache_hit,
        cache_creation_input_tokens,
        cache_read_input_tokens,
        seed_then_fanout,
        breakpoint_mode,
    }
}

fn ensure_not_empty(value: String, fallback: &str, max_len: usize) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        clean(value, max_len)
    }
}

fn normalize_batch_class(plan: &RenderPlan, lane: BatchLane, render_fingerprint: &str) -> BatchClass {
    BatchClass {
        lane,
        provider: clean(&plan.provider, 120),
        model: clean(&plan.model, 160),
        render_fingerprint: clean(render_fingerprint, 200),
        tool_choice: clean(&plan.tool_choice, 120),
        thinking_mode: clean(&plan.thinking_mode, 120),
        image_presence: clean(&plan.image_presence, 120),
        response_mode: clean(&plan.response_mode, 120),
    }
}
