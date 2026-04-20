use std::fs;
use std::io::Write;

fn context_stacks_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn context_stacks_policy_path(root: &Path) -> PathBuf {
    context_stacks_path(root, CONTEXT_STACKS_POLICY_REL)
}

fn context_stacks_state_path(root: &Path) -> PathBuf {
    context_stacks_path(root, CONTEXT_STACKS_STATE_REL)
}

fn context_stacks_receipts_path(root: &Path) -> PathBuf {
    context_stacks_path(root, CONTEXT_STACKS_RECEIPTS_REL)
}

fn context_stacks_digestion_log_path(root: &Path) -> PathBuf {
    context_stacks_path(root, CONTEXT_STACKS_DIGESTION_LOG_REL)
}

fn default_context_stacks_policy() -> ContextStacksPolicy {
    ContextStacksPolicy {
        version: "v1".to_string(),
        cache_threshold_tokens: 256,
        seed_then_fanout_min_cohort: 2,
        lookback_window_tokens: 4096,
        allow_provider_batch_lane: true,
        allow_multi_breakpoint: true,
    }
}

fn default_context_stacks_state() -> ContextStacksState {
    ContextStacksState {
        version: "v1".to_string(),
        manifests: Vec::new(),
        semantic_snapshots: Vec::new(),
        render_plans: Vec::new(),
        provider_snapshots: Vec::new(),
        delta_tails: Vec::new(),
        batch_classes: Vec::new(),
        taste_vectors: std::collections::BTreeMap::new(),
        partial_merge_events: Vec::new(),
        hybrid_retrieval_events: Vec::new(),
        node_spike_thresholds: std::collections::BTreeMap::new(),
        node_spike_events: Vec::new(),
        node_spike_queue: Vec::new(),
        node_spike_metrics: json!({
            "queue_limit": 128u64,
            "dropped_non_critical": 0u64,
            "critical_retained": 0u64,
            "critical_journaled": 0u64,
            "critical_dropped": 0u64,
            "last_overload_at": Value::Null
        }),
        merge_feedback_events: Vec::new(),
        skill_performance_ledger: std::collections::BTreeMap::new(),
        speculative_overlays: Vec::new(),
        speculative_overlay_receipts: Vec::new(),
    }
}

fn read_json_value_or(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("context_stacks_create_parent_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let encoded = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("context_stacks_encode_failed:{err}"))?;
    fs::write(&tmp, encoded).map_err(|err| format!("context_stacks_write_tmp_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("context_stacks_rename_failed:{err}"))?;
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("context_stacks_create_state_dir_failed:{err}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("context_stacks_open_receipts_failed:{err}"))?;
    let line = serde_json::to_string(row)
        .map_err(|err| format!("context_stacks_encode_receipt_failed:{err}"))?;
    writeln!(file, "{line}")
        .map_err(|err| format!("context_stacks_append_receipt_failed:{err}"))?;
    Ok(())
}

fn append_text(path: &Path, line: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("context_stacks_create_log_dir_failed:{err}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("context_stacks_open_log_failed:{err}"))?;
    writeln!(file, "{line}").map_err(|err| format!("context_stacks_append_log_failed:{err}"))?;
    Ok(())
}

fn load_or_init_json<T>(path: &Path, default: T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned + Clone,
{
    if !path.exists() {
        let encoded = serde_json::to_value(&default).unwrap_or_else(|_| serde_json::json!({}));
        let _ = write_json_atomic(path, &encoded);
        return default;
    }
    let fallback = serde_json::to_value(&default).unwrap_or_else(|_| serde_json::json!({}));
    serde_json::from_value(read_json_value_or(path, fallback)).unwrap_or(default)
}

fn load_context_stacks_policy(root: &Path) -> ContextStacksPolicy {
    load_or_init_json(
        &context_stacks_policy_path(root),
        default_context_stacks_policy(),
    )
}

fn load_context_stacks_state(root: &Path) -> ContextStacksState {
    load_or_init_json(&context_stacks_state_path(root), default_context_stacks_state())
}

fn persist_context_stacks_state(root: &Path, state: &ContextStacksState) -> Result<(), String> {
    let path = context_stacks_state_path(root);
    let encoded = serde_json::to_value(state)
        .map_err(|err| format!("context_stacks_encode_state_failed:{err}"))?;
    write_json_atomic(&path, &encoded)
}

fn append_context_stacks_receipt(root: &Path, receipt: &Value) -> Result<(), String> {
    append_jsonl(&context_stacks_receipts_path(root), receipt)
}

fn append_context_stacks_digestion_log(
    root: &Path,
    stack_id: &str,
    lines: &[String],
) -> Result<(), String> {
    let ts = now_iso();
    let mut block = Vec::<String>::new();
    block.push(format!("- ts: \"{ts}\""));
    block.push(format!("  stack_id: \"{}\"", clean(stack_id, 120)));
    block.push("  events:".to_string());
    for line in lines {
        block.push(format!("    - \"{}\"", clean(line, 600).replace('"', "'")));
    }
    append_text(&context_stacks_digestion_log_path(root), &block.join("\n"))
}
