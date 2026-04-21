
#[test]
fn trim_context_pool_compacts_older_verbose_tool_transcripts() {
    let tool_heavy_a = format!(
        "From web retrieval: {}",
        "alpha benchmark tokens ".repeat(220)
    );
    let tool_heavy_b = format!("Terminal output {}", "stdout ".repeat(220));
    let tool_heavy_c = format!("Web benchmark synthesis: {}", "ops latency ".repeat(220));
    let rows = vec![
        json!({"role":"user","text":"compare frameworks"}),
        json!({"role":"assistant","text":tool_heavy_a.clone()}),
        json!({"role":"assistant","text":tool_heavy_b.clone()}),
        json!({"role":"assistant","text":tool_heavy_c.clone()}),
        json!({"role":"user","text":"summarize findings"}),
    ];
    let compacted = trim_context_pool(&rows, 500_000);
    assert_eq!(compacted.len(), rows.len());

    let first_tool_text = message_text(&compacted[1]).to_ascii_lowercase();
    assert!(first_tool_text.contains("compacted"));
    assert!(first_tool_text.len() < tool_heavy_a.to_ascii_lowercase().len());

    let second_tool_text = message_text(&compacted[2]).to_ascii_lowercase();
    let third_tool_text = message_text(&compacted[3]).to_ascii_lowercase();
    assert!(second_tool_text.contains("terminal output"));
    assert!(third_tool_text.contains("web benchmark synthesis"));
}

#[test]
fn trim_context_pool_compacts_older_image_heavy_tool_transcripts() {
    let image_heavy_a = format!(
        "{{\"tool\":\"file_read\",\"content_base64\":\"{}\"}}",
        "a".repeat(2800)
    );
    let image_heavy_b = format!(
        "{{\"tool\":\"file_read\",\"content_base64\":\"{}\"}}",
        "b".repeat(2800)
    );
    let image_heavy_c = format!(
        "{{\"tool\":\"file_read\",\"content_base64\":\"{}\"}}",
        "c".repeat(2800)
    );
    let rows = vec![
        json!({"role":"assistant","text":image_heavy_a.clone()}),
        json!({"role":"assistant","text":image_heavy_b.clone()}),
        json!({"role":"assistant","text":image_heavy_c.clone()}),
        json!({"role":"user","text":"summarize file contents"}),
    ];
    let compacted = trim_context_pool(&rows, 500_000);
    assert_eq!(compacted.len(), rows.len());

    let first_text = message_text(&compacted[0]).to_ascii_lowercase();
    let second_text = message_text(&compacted[1]).to_ascii_lowercase();
    let third_text = message_text(&compacted[2]).to_ascii_lowercase();
    assert!(first_text.contains("screenshot taken"));
    assert!(first_text.len() < image_heavy_a.to_ascii_lowercase().len());
    assert!(second_text.contains("content_base64"));
    assert!(third_text.contains("content_base64"));
}

#[test]
fn trim_context_pool_compacts_older_image_base64_tool_transcripts() {
    let image_heavy_a = format!(
        "{{\"tool\":\"computer\",\"image_base64\":\"{}\"}}",
        "x".repeat(3000)
    );
    let image_heavy_b = format!(
        "{{\"tool\":\"computer\",\"image_base64\":\"{}\"}}",
        "y".repeat(3000)
    );
    let image_heavy_c = format!(
        "{{\"tool\":\"computer\",\"image_base64\":\"{}\"}}",
        "z".repeat(3000)
    );
    let rows = vec![
        json!({"role":"assistant","text":image_heavy_a.clone()}),
        json!({"role":"assistant","text":image_heavy_b.clone()}),
        json!({"role":"assistant","text":image_heavy_c.clone()}),
        json!({"role":"user","text":"summarize image findings"}),
    ];
    let compacted = trim_context_pool(&rows, 500_000);
    assert_eq!(compacted.len(), rows.len());

    let first_text = message_text(&compacted[0]).to_ascii_lowercase();
    let second_text = message_text(&compacted[1]).to_ascii_lowercase();
    let third_text = message_text(&compacted[2]).to_ascii_lowercase();
    assert!(first_text.contains("screenshot taken"));
    assert!(first_text.len() < image_heavy_a.to_ascii_lowercase().len());
    assert!(second_text.contains("image_base64"));
    assert!(third_text.contains("image_base64"));
}

#[test]
fn trim_context_pool_compacts_older_image_url_data_uri_tool_transcripts() {
    let image_heavy_a = format!(
        "{{\"tool\":\"computer\",\"image_url\":\"data:image/png;base64,{}\"}}",
        "u".repeat(3000)
    );
    let image_heavy_b = format!(
        "{{\"tool\":\"computer\",\"image_url\":\"data:image/png;base64,{}\"}}",
        "v".repeat(3000)
    );
    let image_heavy_c = format!(
        "{{\"tool\":\"computer\",\"image_url\":\"data:image/png;base64,{}\"}}",
        "w".repeat(3000)
    );
    let rows = vec![
        json!({"role":"assistant","text":image_heavy_a.clone()}),
        json!({"role":"assistant","text":image_heavy_b.clone()}),
        json!({"role":"assistant","text":image_heavy_c.clone()}),
        json!({"role":"user","text":"summarize image findings"}),
    ];
    let compacted = trim_context_pool(&rows, 500_000);
    assert_eq!(compacted.len(), rows.len());

    let first_text = message_text(&compacted[0]).to_ascii_lowercase();
    let second_text = message_text(&compacted[1]).to_ascii_lowercase();
    let third_text = message_text(&compacted[2]).to_ascii_lowercase();
    assert!(first_text.contains("screenshot taken"));
    assert!(first_text.len() < image_heavy_a.to_ascii_lowercase().len());
    assert!(second_text.contains("image_url"));
    assert!(third_text.contains("image_url"));
}

#[test]
fn trim_context_pool_compacts_older_camel_case_image_payload_tool_transcripts() {
    let image_heavy_a = format!(
        "{{\"tool\":\"computer\",\"contentBase64\":\"{}\"}}",
        "r".repeat(3000)
    );
    let image_heavy_b = format!(
        "{{\"tool\":\"computer\",\"imageData\":\"data:image/png;base64,{}\"}}",
        "s".repeat(3000)
    );
    let image_heavy_c = format!(
        "{{\"tool\":\"computer\",\"contentBase64\":\"{}\"}}",
        "t".repeat(3000)
    );
    let rows = vec![
        json!({"role":"assistant","text":image_heavy_a.clone()}),
        json!({"role":"assistant","text":image_heavy_b.clone()}),
        json!({"role":"assistant","text":image_heavy_c.clone()}),
        json!({"role":"user","text":"summarize image findings"}),
    ];
    let compacted = trim_context_pool(&rows, 500_000);
    assert_eq!(compacted.len(), rows.len());

    let first_text = message_text(&compacted[0]).to_ascii_lowercase();
    let second_text = message_text(&compacted[1]).to_ascii_lowercase();
    let third_text = message_text(&compacted[2]).to_ascii_lowercase();
    assert!(first_text.contains("screenshot taken"));
    assert!(first_text.len() < image_heavy_a.to_ascii_lowercase().len());
    assert!(second_text.contains("imagedata"));
    assert!(third_text.contains("contentbase64"));
}

#[test]
fn context_command_emergency_compacts_before_saturation() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    let _ = update_profile_patch(
        root.path(),
        &agent_id,
        &json!({"context_window": 512, "context_window_tokens": 512}),
    );
    write_agent_session_messages(
        root.path(),
        &agent_id,
        synthetic_session_messages(120, "context-pressure", "alpha", 80),
    );

    let context = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/command"),
        br#"{"command":"context","silent":true,"active_context_target_tokens":512,"active_context_min_recent_messages":4,"auto_compact_threshold_ratio":0.95,"auto_compact_target_ratio":0.45}"#,
        &json!({"ok": true}),
    )
    .expect("context command");
    assert_eq!(context.status, 200);
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/emergency_compact/triggered")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/emergency_compact/removed_messages")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0,
        true
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/emergency_compact/after_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX)
            < context
                .payload
                .pointer("/context_pool/emergency_compact/before_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(i64::MIN),
        true
    );
}

#[test]
fn message_ignores_unrelated_passive_memory_when_term_index_is_missing() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    let attention_path = root
        .path()
        .join("client/runtime/local/state/attention/queue.jsonl");
    if let Some(parent) = attention_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let unrelated = json!({
        "ts": crate::now_iso(),
        "source": format!("agent:{agent_id}"),
        "source_type": "passive_memory_turn",
        "severity": "info",
        "summary": "SQL-Data-Exploration Data Exploration in SQL for Covid-19 Data Project Overview Data Source Tools Used",
        "raw_event": {
            "agent_id": agent_id,
            "memory_kind": "passive_turn",
            "user_text": "legacy row without indexed terms",
            "assistant_text": "legacy row without indexed terms"
        }
    });
    let encoded = serde_json::to_string(&unrelated).expect("encode attention row");
    std::fs::write(&attention_path, format!("{encoded}\n")).expect("write attention queue");

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"code me a reverse linked list"}"#,
        &json!({"ok": true}),
    )
    .expect("message");
    assert_eq!(response.status, 200);
    let text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        !text.contains("sql-data-exploration"),
        "unrelated passive-memory project summary should never leak into response text"
    );
    assert!(
        !text.contains("project overview"),
        "template-section drift should be filtered before prompt assembly"
    );
    assert!(
        !text.contains("covid-19"),
        "legacy unrelated context row should not steer coding request replies"
    );
}

#[test]
fn context_defaults_to_minimum_recent_window_floor() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    let context = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/command"),
        br#"{"command":"context","silent":true}"#,
        &json!({"ok": true}),
    )
    .expect("context command");
    assert_eq!(context.status, 200);
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/min_recent_messages")
            .and_then(Value::as_u64),
        Some(28),
    );
}

#[test]
fn memory_recall_prefers_active_session_earliest_turn_for_first_chat_queries() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    let _ = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"first-marker-alpha: we discussed memory continuity"}"#,
        &json!({"ok": true}),
    )
    .expect("seed first");
    let _ = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"later-marker-beta: we then discussed tool routing"}"#,
        &json!({"ok": true}),
    )
    .expect("seed second");
    let recall = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"can you remember our first chat?"}"#,
        &json!({"ok": true}),
    )
    .expect("recall");
    let text = recall
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(text.contains("first-marker-alpha"));
}
