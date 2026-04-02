const PROMPT_CACHE_INDEX_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/prompt_cache_index.json";

fn prompt_cache_now_epoch_ms() -> i64 {
    let now = std::time::SystemTime::now();
    now.duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
struct PromptOptimizationPlan {
    system_prompt: String,
    session_messages: Vec<Value>,
    user_message: String,
    assistant_prefill: String,
    metadata: Value,
}

fn prompt_cache_index_path(root: &Path) -> PathBuf {
    root.join(PROMPT_CACHE_INDEX_REL)
}

fn load_prompt_cache_index(root: &Path) -> Value {
    read_json(&prompt_cache_index_path(root)).unwrap_or_else(|| {
        json!({
            "type": "infring_dashboard_prompt_cache_index",
            "updated_at": crate::now_iso(),
            "lanes": {}
        })
    })
}

fn save_prompt_cache_index(root: &Path, mut value: Value) {
    if !value.is_object() {
        value = json!({});
    }
    value["type"] = json!("infring_dashboard_prompt_cache_index");
    value["updated_at"] = json!(crate::now_iso());
    write_json_pretty(&prompt_cache_index_path(root), &value);
}

fn tokenized_lower(text: &str, max_tokens: usize) -> Vec<String> {
    clean_text(text, 16_000)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 3)
        .take(max_tokens)
        .map(|token| token.to_string())
        .collect::<Vec<_>>()
}

fn message_overlap_ratio(messages: &[(String, String)], user_message: &str) -> f64 {
    let user_tokens = tokenized_lower(user_message, 32);
    if user_tokens.is_empty() {
        return 0.0;
    }
    let corpus = messages
        .iter()
        .rev()
        .take(12)
        .map(|(_, text)| clean_text(text, 2_400).to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let overlap = user_tokens
        .iter()
        .filter(|token| corpus.contains(token.as_str()))
        .count();
    overlap as f64 / user_tokens.len() as f64
}

fn summarize_message_window(rows: &[(String, String)], max_lines: usize) -> String {
    let mut lines = Vec::<String>::new();
    for (role, text) in rows.iter().rev().take(max_lines).rev() {
        let role_clean = clean_text(role, 24).to_ascii_lowercase();
        let snippet = clean_text(text, 180);
        if role_clean.is_empty() || snippet.is_empty() {
            continue;
        }
        lines.push(format!("- {}: {}", role_clean, snippet));
    }
    clean_text(&lines.join("\n"), 2_400)
}

fn infer_output_contract(user_message: &str) -> (&'static str, &'static str, &'static str) {
    let lowered = clean_text(user_message, 2_000).to_ascii_lowercase();
    if lowered.contains("json")
        || lowered.contains("schema")
        || lowered.contains("structured output")
    {
        return ("json", "{", "Respond with valid JSON only.");
    }
    if lowered.contains("list")
        || lowered.contains("steps")
        || lowered.contains("bullet")
        || lowered.contains("checklist")
    {
        return ("markdown_list", "-", "Respond as concise markdown bullet points.");
    }
    ("plain_text", "", "Respond concisely with only what is needed.")
}

fn build_structured_system_prompt(
    system_prompt: &str,
    cache_lane: &str,
    stable_hash: &str,
    output_contract: &str,
    output_instruction: &str,
    summary_block: Option<&str>,
) -> String {
    let mut sections = Vec::<String>::new();
    let instructions = clean_chat_text(system_prompt, 9_000);
    if !instructions.trim().is_empty() {
        sections.push(format!(
            "<instructions>\n{}\n</instructions>",
            instructions
        ));
    }
    sections.push(format!(
        "<cache_control lane=\"{}\" stable_hash=\"{}\" breakpoint=\"system_instructions\" />",
        clean_text(cache_lane, 64),
        clean_text(stable_hash, 32)
    ));
    sections.push(format!(
        "<output_format type=\"{}\">{}</output_format>",
        clean_text(output_contract, 40),
        clean_text(output_instruction, 320)
    ));
    if let Some(summary) = summary_block {
        if !summary.trim().is_empty() {
            sections.push(format!("<context_summary>\n{}\n</context_summary>", summary));
        }
    }
    clean_chat_text(&sections.join("\n\n"), 12_000)
}

fn compute_prompt_cache_hit(
    root: &Path,
    cache_lane: &str,
    provider_id: &str,
    model_name: &str,
    stable_hash: &str,
) -> (bool, u64) {
    let mut index = load_prompt_cache_index(root);
    if index.get("lanes").is_none() || !index.get("lanes").map(Value::is_object).unwrap_or(false) {
        index["lanes"] = json!({});
    }
    let lane = clean_text(cache_lane, 80);
    let provider = normalize_provider_id(provider_id);
    let model = clean_text(model_name, 240);
    let key = format!("{provider}::{model}");
    let now_ts = prompt_cache_now_epoch_ms();
    let mut cache_hit = false;
    let hit_count;

    if index["lanes"].get(&lane).is_none()
        || !index["lanes"]
            .get(&lane)
            .map(Value::is_object)
            .unwrap_or(false)
    {
        index["lanes"][lane.clone()] = json!({});
    }
    if index["lanes"][&lane].get(&key).is_none()
        || !index["lanes"][&lane]
            .get(&key)
            .map(Value::is_object)
            .unwrap_or(false)
    {
        index["lanes"][lane.clone()][key.clone()] = json!({});
    }
    let entry = &mut index["lanes"][&lane][&key];
    let previous_hash = clean_text(
        entry.get("stable_hash").and_then(Value::as_str).unwrap_or(""),
        80,
    );
    if !previous_hash.is_empty() && previous_hash == clean_text(stable_hash, 80) {
        cache_hit = true;
        hit_count = entry
            .get("hit_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1);
    } else {
        hit_count = 1;
    }
    *entry = json!({
        "stable_hash": clean_text(stable_hash, 80),
        "hit_count": hit_count,
        "last_seen_ms": now_ts,
    });
    save_prompt_cache_index(root, index);
    (cache_hit, hit_count)
}

fn optimize_prompt_request(
    root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    session_messages: &[Value],
    user_message: &str,
) -> PromptOptimizationPlan {
    let mut rows = content_from_message_rows(session_messages);
    let user_clean = clean_chat_text(user_message, 16_000);
    let mut summary_applied = false;
    let mut context_cleared = false;
    let mut summarized_turns = 0usize;
    let mut summary_block = String::new();

    if rows.len() > 14 {
        let keep_recent = 8usize;
        let split_at = rows.len().saturating_sub(keep_recent);
        let older = rows[..split_at].to_vec();
        summary_block = summarize_message_window(&older, 8);
        summarized_turns = older.len();
        if !summary_block.is_empty() {
            summary_applied = true;
        }
        rows = rows[split_at..].to_vec();
    }

    let overlap_ratio = message_overlap_ratio(&rows, &user_clean);
    if rows.len() > 6 && overlap_ratio < 0.08 {
        rows = rows
            .into_iter()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        context_cleared = true;
    }

    let (output_contract, prefill, output_instruction) = infer_output_contract(&user_clean);
    let cache_lane =
        crate::model_router::prompt_cache_lane_for_route("default", output_contract, false);
    let stable_hash = crate::deterministic_receipt_hash(&json!({
        "cache_lane": cache_lane,
        "provider": normalize_provider_id(provider_id),
        "model": clean_text(model_name, 240),
        "system_prompt": clean_chat_text(system_prompt, 8_000),
        "output_contract": output_contract,
    }));
    let (cache_hit, cache_hit_count) =
        compute_prompt_cache_hit(root, &cache_lane, provider_id, model_name, &stable_hash);
    let structured_system_prompt = build_structured_system_prompt(
        system_prompt,
        &cache_lane,
        &stable_hash[..16.min(stable_hash.len())],
        output_contract,
        output_instruction,
        if summary_applied {
            Some(&summary_block)
        } else {
            None
        },
    );

    let prepared_messages = rows
        .into_iter()
        .map(|(role, text)| json!({"role": role, "text": clean_chat_text(&text, 8_000)}))
        .collect::<Vec<_>>();

    let metadata = json!({
        "cache_control": {
            "lane": cache_lane,
            "stable_hash": clean_text(&stable_hash, 80),
            "cache_hit": cache_hit,
            "cache_hit_count": cache_hit_count,
            "breakpoint": "system_instructions",
        },
        "context": {
            "summary_applied": summary_applied,
            "summarized_turns": summarized_turns,
            "context_cleared": context_cleared,
            "overlap_ratio": overlap_ratio,
        },
        "output_contract": {
            "type": output_contract,
            "assistant_prefill": prefill,
        }
    });

    PromptOptimizationPlan {
        system_prompt: structured_system_prompt,
        session_messages: prepared_messages,
        user_message: user_clean,
        assistant_prefill: clean_chat_text(prefill, 160),
        metadata,
    }
}
