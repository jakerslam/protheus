fn verify_session_reachable(
    state: &SwarmState,
    session_id: &str,
    timeout_ms: u64,
) -> Result<Value, String> {
    let deadline = now_epoch_ms().saturating_add(timeout_ms);
    loop {
        if state
            .sessions
            .get(session_id)
            .map(|session| session.reachable)
            .unwrap_or(false)
        {
            return Ok(json!({
                "status": "verified",
                "session_id": session_id,
                "timeout_ms": timeout_ms
            }));
        }
        if now_epoch_ms() >= deadline {
            return Err(format!("session_unreachable_timeout:{session_id}"));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn scale_task_complexity(base_task: &str, token_budget: u32) -> String {
    match token_budget {
        0..=200 => format!("{base_task} (ultra-concise, max 50 words)"),
        201..=500 => format!("{base_task} (concise, max 100 words)"),
        501..=1000 => format!("{base_task} (standard detail)"),
        1001..=5000 => format!("{base_task} (comprehensive)"),
        _ => format!("{base_task} (exhaustive analysis)"),
    }
}

fn estimate_tool_plan(task: &str, token_budget: Option<u32>) -> Vec<(String, u32)> {
    let read_tokens = 120u32.saturating_add(((task.len() as u32) / 8).min(100));
    let generate_tokens = match token_budget.unwrap_or(1200) {
        0..=200 => 40,
        201..=500 => 80,
        501..=1000 => 140,
        1001..=5000 => 300,
        _ => 600,
    };
    vec![
        ("read".to_string(), read_tokens),
        ("generate".to_string(), generate_tokens),
    ]
}
fn parse_execution_mode(argv: &[String]) -> ExecutionMode {
    let mode = parse_flag(argv, "execution-mode")
        .unwrap_or_else(|| "task".to_string())
        .trim()
        .to_ascii_lowercase();
    let cfg = PersistentAgentConfig {
        lifespan_sec: parse_u64_flag(argv, "lifespan-sec", 3600).max(1),
        check_in_interval_sec: parse_u64_flag(argv, "check-in-interval-sec", 60).max(1),
        report_mode: ReportMode::from_flag(parse_flag(argv, "report-mode")),
    };
    match mode.as_str() {
        "persistent" => ExecutionMode::Persistent(cfg),
        "background" => ExecutionMode::Background(cfg),
        _ => ExecutionMode::TaskOriented,
    }
}

fn report_mode_should_emit(report_mode: &ReportMode, anomalies: &[String], is_final: bool) -> bool {
    if is_final {
        return true;
    }
    match report_mode {
        ReportMode::Always => true,
        ReportMode::AnomaliesOnly => !anomalies.is_empty(),
        ReportMode::FinalOnly => false,
    }
}

fn collect_metrics_snapshot(
    telemetry: Option<&BudgetTelemetry>,
    check_in_count: u64,
    response_latency_ms: u64,
    active_tools: Vec<String>,
) -> MetricsSnapshot {
    let cumulative_tokens = telemetry.map(|t| t.final_usage).unwrap_or(0);
    let context_percentage = (cumulative_tokens as f64 / 8192.0).min(1.0);
    let memory_usage_mb = 1 + (check_in_count / 8);
    MetricsSnapshot {
        timestamp_ms: now_epoch_ms(),
        cumulative_tokens,
        context_percentage,
        response_latency_ms,
        memory_usage_mb,
        active_tools,
    }
}

fn detect_anomalies(timeline: &[MetricsSnapshot]) -> Vec<String> {
    if timeline.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for window in timeline.windows(2) {
        let prev = &window[0];
        let current = &window[1];
        let delta = current
            .cumulative_tokens
            .saturating_sub(prev.cumulative_tokens);
        if delta > 400 {
            out.push("token_spike".to_string());
            break;
        }
    }
    if let (Some(first), Some(last)) = (timeline.first(), timeline.last()) {
        if first.response_latency_ms > 0
            && last.response_latency_ms > first.response_latency_ms.saturating_mul(2)
            && last.response_latency_ms > 10
        {
            out.push("latency_degradation".to_string());
        }
    }
    if timeline
        .last()
        .map(|row| row.context_percentage >= 0.85)
        .unwrap_or(false)
    {
        out.push("context_bloat".to_string());
    }
    out.sort();
    out.dedup();
    out
}

fn default_spawn_options() -> SpawnOptions {
    SpawnOptions {
        verify: false,
        timeout_ms: 30_000,
        metrics_detailed: false,
        simulate_unreachable: false,
        byzantine: false,
        corruption_type: "data_falsification".to_string(),
        token_budget: None,
        token_warning_threshold: 0.8,
        budget_exhaustion_action: BudgetAction::FailHard,
        adaptive_complexity: false,
        execution_mode: ExecutionMode::TaskOriented,
        role: None,
        capabilities: Vec::new(),
        auto_publish_results: false,
        agent_label: None,
        result_value: None,
        result_text: None,
        result_confidence: 1.0,
        verification_status: "not_verified".to_string(),
    }
}

fn build_spawn_options(argv: &[String]) -> SpawnOptions {
    let metrics_detailed = parse_flag(argv, "metrics")
        .map(|value| value.eq_ignore_ascii_case("detailed"))
        .unwrap_or(false);
    let token_budget = parse_first_flag(
        argv,
        &["token-budget", "token_budget", "max-tokens", "max_tokens"],
    )
    .and_then(|value| value.trim().parse::<u32>().ok())
    .filter(|value| *value > 0);
    let token_warning_threshold =
        parse_f64_flag(argv, "token-warning-at", 0.8).clamp(0.0, 1.0) as f32;
    let mut options = default_spawn_options();
    options.verify = parse_bool_flag(argv, "verify", false);
    options.timeout_ms = (parse_f64_flag(argv, "timeout-sec", 30.0).max(0.0) * 1000.0) as u64;
    options.metrics_detailed = metrics_detailed;
    options.simulate_unreachable = parse_bool_flag(argv, "simulate-unreachable", false);
    options.byzantine = parse_bool_flag(argv, "byzantine", false);
    options.corruption_type =
        parse_flag(argv, "corruption-type").unwrap_or_else(|| "data_falsification".to_string());
    options.token_budget = token_budget;
    options.token_warning_threshold = token_warning_threshold;
    options.budget_exhaustion_action =
        BudgetAction::from_flag(parse_flag(argv, "on-budget-exhausted"));
    options.adaptive_complexity = parse_bool_flag(argv, "adaptive-complexity", false);
    options.execution_mode = parse_execution_mode(argv);
    options.role = parse_flag(argv, "role")
        .map(|value| clean_text(&value, 64))
        .filter(|value| !value.trim().is_empty());
    options.capabilities = parse_capabilities(argv);
    options.auto_publish_results = parse_bool_flag(argv, "auto-publish-results", false);
    options.agent_label = parse_flag(argv, "agent-label")
        .or_else(|| parse_flag(argv, "label"))
        .filter(|value| !value.trim().is_empty());
    options.result_value =
        parse_flag(argv, "result-value").and_then(|value| value.trim().parse::<f64>().ok());
    options.result_text = parse_flag(argv, "result-text").filter(|value| !value.trim().is_empty());
    options.result_confidence = parse_f64_flag(argv, "result-confidence", 1.0).clamp(0.0, 1.0);
    options.verification_status = parse_flag(argv, "verification-status")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "not_verified".to_string());
    options
}

fn parse_capabilities(argv: &[String]) -> Vec<String> {
    parse_flag(argv, "capabilities")
        .map(|raw| {
            raw.split(',')
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn sanitize_capability_label(raw: &str) -> Option<String> {
    let lowered = raw
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '/'], "_")
        .replace('.', "_");
    if lowered.is_empty() {
        return None;
    }
    lowered
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
        .then_some(lowered)
}

fn default_role_capability_envelope(role: &str) -> Vec<String> {
    match role {
        "coordinator" => vec![
            "delegate".to_string(),
            "audit".to_string(),
            "summarize".to_string(),
        ],
        "validator" => vec!["validate".to_string(), "audit".to_string()],
        "generator" => vec!["generate".to_string(), "relay".to_string()],
        "filter" => vec!["filter".to_string()],
        "summarizer" => vec!["summarize".to_string()],
        _ => vec!["execute".to_string(), "report".to_string()],
    }
}

fn resolve_spawn_role_card(options: &SpawnOptions, goal: &str) -> Result<RoleCard, String> {
    let role = options
        .role
        .clone()
        .map(|value| clean_text(&value, 64).to_ascii_lowercase().replace(' ', "_"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "worker".to_string());

    let mut capability_set = BTreeSet::new();
    for capability in &options.capabilities {
        if let Some(normalized) = sanitize_capability_label(capability) {
            capability_set.insert(normalized);
        }
    }
    if capability_set.is_empty() {
        for capability in default_role_capability_envelope(&role) {
            if let Some(normalized) = sanitize_capability_label(&capability) {
                capability_set.insert(normalized);
            }
        }
    }
    let capability_envelope = capability_set.into_iter().collect::<Vec<_>>();
    if capability_envelope.is_empty() {
        return Err("role_card_capability_envelope_required".to_string());
    }

    Ok(RoleCard {
        role,
        goal: clean_text(goal, 160),
        capability_envelope,
        source: if options.role.is_some() {
            "explicit".to_string()
        } else {
            "derived".to_string()
        },
    })
}

fn ensure_mailbox<'a>(state: &'a mut SwarmState, session_id: &str) -> &'a mut SessionMailbox {
    state
        .mailboxes
        .entry(session_id.to_string())
        .or_insert_with(|| SessionMailbox {
            session_id: session_id.to_string(),
            unread: Vec::new(),
            read: Vec::new(),
        })
}

fn session_exists(state: &SwarmState, session_id: &str) -> bool {
    state.sessions.contains_key(session_id)
}

fn next_message_id(
    state: &mut SwarmState,
    sender_session_id: &str,
    recipient_session_id: &str,
    payload: &str,
) -> String {
    state.message_sequence = state.message_sequence.saturating_add(1);
    let digest = deterministic_receipt_hash(&json!({
        "sender": sender_session_id,
        "recipient": recipient_session_id,
        "payload": payload,
        "seq": state.message_sequence,
        "ts": now_epoch_ms(),
    }));
    format!("msg-{}-{:x}", &digest[..10], state.message_sequence)
}

fn is_lineage_adjacent_allowed(
    state: &SwarmState,
    sender_session_id: &str,
    recipient_session_id: &str,
) -> bool {
    let Some(sender) = state.sessions.get(sender_session_id) else {
        return false;
    };
    let Some(recipient) = state.sessions.get(recipient_session_id) else {
        return false;
    };
    let sibling = sender.parent_id == recipient.parent_id;
    let child = recipient.parent_id.as_deref() == Some(sender_session_id);
    let parent = sender.parent_id.as_deref() == Some(recipient_session_id);
    sibling || child || parent
}

fn send_session_message(
    state: &mut SwarmState,
    sender_session_id: &str,
    recipient_session_id: &str,
    payload: &str,
    delivery: DeliveryGuarantee,
    simulate_first_attempt_fail: bool,
    ttl_ms: u64,
) -> Result<Value, String> {
    if sender_session_id != "coordinator" && !session_exists(state, sender_session_id) {
        return Err(format!("unknown_sender_session:{sender_session_id}"));
    }
    if !session_exists(state, recipient_session_id) {
        return Err(format!("unknown_recipient_session:{recipient_session_id}"));
    }
    if sender_session_id != "coordinator" && sender_session_id == recipient_session_id {
        return Err("self_delivery_blocked".to_string());
    }
    if sender_session_id != "coordinator"
        && !is_lineage_adjacent_allowed(state, sender_session_id, recipient_session_id)
    {
        return Err(format!(
            "recipient_scope_denied:sender={sender_session_id}:recipient={recipient_session_id}"
        ));
    }

    let dedupe_key = deterministic_receipt_hash(&json!({
        "sender": sender_session_id,
        "recipient": recipient_session_id,
        "payload": payload,
        "delivery": delivery.as_label(),
    }));
    if matches!(delivery, DeliveryGuarantee::ExactlyOnce) {
        if let Some(existing_message_id) = state.exactly_once_dedupe.get(&dedupe_key).cloned() {
            return Ok(json!({
                "ok": true,
                "type": "swarm_runtime_sessions_send",
                "sender_session_id": sender_session_id,
                "recipient_session_id": recipient_session_id,
                "delivery": delivery.as_label(),
                "dedupe_hit": true,
                "message_id": existing_message_id,
            }));
        }
    }

    let recipient_reachable = state
        .sessions
        .get(recipient_session_id)
        .map(|session| session.reachable)
        .unwrap_or(false);
    let attempts = if simulate_first_attempt_fail
        && matches!(
            delivery,
            DeliveryGuarantee::AtLeastOnce | DeliveryGuarantee::ExactlyOnce
        ) {
        2
    } else if !recipient_reachable
        && matches!(
            delivery,
            DeliveryGuarantee::AtLeastOnce | DeliveryGuarantee::ExactlyOnce
        )
    {
        3
    } else {
        1
    };
    let message_id = next_message_id(state, sender_session_id, recipient_session_id, payload);
    let ttl_ms = ttl_ms.max(1);
    let message = AgentMessage {
        message_id: message_id.clone(),
        sender_session_id: sender_session_id.to_string(),
        recipient_session_id: recipient_session_id.to_string(),
        payload: payload.to_string(),
        created_at: now_iso(),
        timestamp_ms: now_epoch_ms(),
        delivery: delivery.clone(),
        attempts,
        acknowledged: false,
        acked_at_ms: None,
        dedupe_key: if matches!(delivery, DeliveryGuarantee::ExactlyOnce) {
            Some(dedupe_key.clone())
        } else {
            None
        },
        ttl_ms,
        expires_at_ms: now_epoch_ms().saturating_add(ttl_ms),
        deferred: !recipient_reachable,
    };

    let mailbox_depth = state
        .mailboxes
        .get(recipient_session_id)
        .map(|mailbox| mailbox.unread.len())
        .unwrap_or(0);
    if mailbox_depth >= MAX_MAILBOX_UNREAD {
        append_dead_letter(state, message.clone(), "mailbox_backpressure", true);
        append_event(
            state,
            json!({
                "type": "swarm_dead_letter",
                "message_id": message_id,
                "sender_session_id": sender_session_id,
                "recipient_session_id": recipient_session_id,
                "reason": "mailbox_backpressure",
                "timestamp": now_iso(),
            }),
        );
        return Ok(json!({
            "ok": true,
            "type": "swarm_runtime_sessions_send",
            "message_id": message.message_id,
            "sender_session_id": message.sender_session_id,
            "recipient_session_id": message.recipient_session_id,
            "delivery": message.delivery.as_label(),
            "attempts": message.attempts,
            "dedupe_hit": false,
            "dead_lettered": true,
            "dead_letter_reason": "mailbox_backpressure",
        }));
    }
    if !recipient_reachable && matches!(delivery, DeliveryGuarantee::AtMostOnce) {
        append_dead_letter(state, message.clone(), "recipient_unreachable", true);
        return Err(format!(
            "recipient_unreachable:delivery={}:recipient={recipient_session_id}",
            delivery.as_label()
        ));
    }

    ensure_mailbox(state, recipient_session_id)
        .unread
        .push(message.clone());
    if matches!(delivery, DeliveryGuarantee::ExactlyOnce) {
        state
            .exactly_once_dedupe
            .insert(dedupe_key, message_id.clone());
    }

    append_event(
        state,
        json!({
            "type": "swarm_message_sent",
            "message_id": message_id,
            "sender_session_id": sender_session_id,
            "recipient_session_id": recipient_session_id,
            "delivery": delivery.as_label(),
            "attempts": attempts,
            "deferred": !recipient_reachable,
            "timestamp": now_iso(),
        }),
    );

    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_sessions_send",
        "message_id": message.message_id,
        "sender_session_id": message.sender_session_id,
        "recipient_session_id": message.recipient_session_id,
        "delivery": message.delivery.as_label(),
        "attempts": message.attempts,
        "dedupe_hit": false,
        "deferred": !recipient_reachable,
        "ttl_ms": ttl_ms,
    }))
}
