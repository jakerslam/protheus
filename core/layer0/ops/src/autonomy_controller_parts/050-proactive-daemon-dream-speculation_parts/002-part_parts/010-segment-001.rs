fn run_proactive_daemon_daemon(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let action = clean_id(
        parse_flag(argv, "action").or_else(|| parse_positional(argv, 1)),
        "status",
    );
    let auto = parse_bool(parse_flag(argv, "auto").as_deref(), false);
    let force_cycle = parse_bool(parse_flag(argv, "force").as_deref(), false);
    let tick_ms = parse_u64(parse_flag(argv, "tick-ms").as_deref(), 5000, 1000, 60_000);
    let jitter_ms = parse_u64(
        parse_flag(argv, "jitter-ms").as_deref(),
        400,
        0,
        tick_ms.min(5_000),
    );
    let window_sec = parse_u64(parse_flag(argv, "window-sec").as_deref(), 900, 10, 86_400);
    let max_messages = parse_u64(parse_flag(argv, "max-proactive").as_deref(), 2, 1, 64);
    let blocking_budget_ms = parse_u64(
        parse_flag(argv, "block-budget-ms").as_deref(),
        15_000,
        50,
        120_000,
    );
    let dream_idle_ms = parse_u64(
        parse_flag(argv, "dream-idle-ms").as_deref(),
        6 * 60 * 60 * 1000,
        60_000,
        30 * 24 * 60 * 60 * 1000,
    );
    let dream_max_without_ms = parse_u64(
        parse_flag(argv, "dream-max-without-ms").as_deref(),
        24 * 60 * 60 * 1000,
        60_000,
        60 * 24 * 60 * 60 * 1000,
    );
    let policy_tier = clean(
        parse_flag(argv, "policy-tier")
            .unwrap_or_else(|| {
                std::env::var("PROACTIVE_DAEMON_POLICY_TIER")
                    .unwrap_or_else(|_| "observe".to_string())
            }),
        32,
    );
    let enabled_tool_surfaces = parse_tool_surfaces(
        parse_flag(argv, "tool-surfaces").or_else(|| parse_flag(argv, "tools")),
    );
    let brief_mode = parse_bool(parse_flag(argv, "brief").as_deref(), true);
    let now_ms = now_epoch_ms();

    let mut state = read_json(&proactive_daemon_state_path(root))
        .unwrap_or_else(proactive_daemon_default_state);
    ensure_proactive_daemon_state_shape(&mut state);
    state["heartbeat"]["tick_ms"] = json!(tick_ms);
    state["heartbeat"]["jitter_ms"] = json!(jitter_ms);
    state["proactive"]["window_sec"] = json!(window_sec);
    state["proactive"]["max_messages"] = json!(max_messages);
    state["proactive"]["brief_mode"] = json!(brief_mode);
    state["budgets"]["blocking_ms"] = json!(blocking_budget_ms);
    state["tool_surfaces"]["policy_tier"] = json!(policy_tier);
    state["tool_surfaces"]["enabled"] = Value::Array(
        enabled_tool_surfaces
            .iter()
            .cloned()
            .map(Value::String)
            .collect(),
    );
    state["dream"]["max_idle_ms"] = json!(dream_idle_ms);
    state["dream"]["max_without_dream_ms"] = json!(dream_max_without_ms);
    rollover_proactive_window(&mut state, now_ms);
    purge_expired_isolation_quarantine(&mut state, now_ms);

