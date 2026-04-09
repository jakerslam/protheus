fn tool_used(payload: &Value, tool_name: &str) -> bool {
    payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("name")
                    .and_then(Value::as_str)
                    .map(|name| name == tool_name)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn tool_names(payload: &Value) -> Vec<String> {
    payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    row.get("name")
                        .and_then(Value::as_str)
                        .map(|name| name.to_string())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn record_gauntlet_task(
    results: &mut Vec<(String, bool, String)>,
    task: &str,
    pass: bool,
    detail: impl Into<String>,
) {
    results.push((task.to_string(), pass, detail.into()));
}

fn response_status(response: Option<&CompatApiResponse>) -> u16 {
    response.map(|row| row.status).unwrap_or(0)
}

fn response_payload(response: Option<&CompatApiResponse>) -> Value {
    response
        .map(|row| row.payload.clone())
        .unwrap_or_else(|| json!({}))
}

#[test]
fn agent_capability_gauntlet_20_difficult_tasks() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let _ = fs::create_dir_all(root.path().join("notes"));
    let _ = fs::create_dir_all(root.path().join("src").join("api"));
    let _ = fs::write(root.path().join("notes/plan.txt"), "ship it");
    let _ = fs::write(
        root.path().join("src").join("api").join("security_gate.rs"),
        "pub fn gate() {}",
    );

    let snapshot = json!({
        "ok": true,
        "runtime_sync": {
            "summary": {
                "queue_depth": 0,
                "conduit_signals": 1,
                "backpressure_level": "normal"
            }
        },
        "health": {
            "alerts": {
                "count": 1,
                "checks": [
                    {"id": "queue_depth", "severity": "warning", "message": "queue pressure drift"}
                ]
            }
        }
    });

    let mut results = Vec::<(String, bool, String)>::new();

    // 01) Create a primary agent that will execute the gauntlet.
    let parent_create = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Gauntlet Parent","role":"director"}"#,
        &snapshot,
    );
    let parent_status = response_status(parent_create.as_ref());
    let parent_id = parent_create
        .as_ref()
        .and_then(|row| row.payload.get("agent_id").and_then(Value::as_str))
        .map(|row| clean_text(row, 180))
        .unwrap_or_default();
    record_gauntlet_task(
        &mut results,
        "01_create_parent_agent",
        parent_status == 200 && !parent_id.is_empty(),
        format!("status={parent_status} agent_id={parent_id}"),
    );

    // 02) Read a workspace file through conversational slash routing.
    let file_read = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        br#"{"message":"/file notes/plan.txt"}"#,
        &snapshot,
    );
    let file_status = response_status(file_read.as_ref());
    let file_payload = response_payload(file_read.as_ref());
    let file_response = clean_text(
        file_payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or(""),
        400,
    );
    let file_read_natural = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        br#"{"message":"read file notes/plan.txt and show me full contents"}"#,
        &snapshot,
    );
    let file_natural_status = response_status(file_read_natural.as_ref());
    let file_natural_payload = response_payload(file_read_natural.as_ref());
    let file_natural_response = clean_text(
        file_natural_payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or(""),
        400,
    );
    record_gauntlet_task(
        &mut results,
        "02_file_read_routes",
        file_status == 200
            && tool_used(&file_payload, "file_read")
            && file_response.contains("ship it")
            && file_natural_status == 200
            && tool_used(&file_natural_payload, "file_read")
            && file_natural_response.contains("ship it"),
        format!(
            "slash_status={file_status} natural_status={file_natural_status} slash={file_response} natural={file_natural_response}"
        ),
    );

    // 03) Export a folder tree through conversational slash routing.
    let folder_export = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        br#"{"message":"/folder notes"}"#,
        &snapshot,
    );
    let folder_status = response_status(folder_export.as_ref());
    let folder_payload = response_payload(folder_export.as_ref());
    record_gauntlet_task(
        &mut results,
        "03_folder_export_slash_route",
        folder_status == 200 && tool_used(&folder_payload, "folder_export"),
        format!("status={folder_status}"),
    );

    // 04) Fetch a live URL through governed web conduit routing.
    let browse = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        br#"{"message":"/browse https://example.com"}"#,
        &snapshot,
    );
    let browse_status = response_status(browse.as_ref());
    let browse_payload = response_payload(browse.as_ref());
    record_gauntlet_task(
        &mut results,
        "04_web_fetch_routing",
        matches!(browse_status, 200 | 400) && tool_used(&browse_payload, "web_fetch"),
        format!("status={browse_status}"),
    );

    // 05) Run natural-language web search intent without slash command.
    let web_search = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        br#"{"message":"search the web for websocket reconnect jitter handling"}"#,
        &snapshot,
    );
    let web_search_status = response_status(web_search.as_ref());
    let web_search_payload = response_payload(web_search.as_ref());
    let search_routed = tool_used(&web_search_payload, "web_search")
        || tool_used(&web_search_payload, "batch_query");
    let search_tools = tool_names(&web_search_payload).join(",");
    record_gauntlet_task(
        &mut results,
        "05_web_search_natural_intent",
        matches!(web_search_status, 200 | 400) && search_routed,
        format!("status={web_search_status} tools={search_tools}"),
    );

    // 06) Persist semantic memory via slash memory set.
    let memory_set = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        br#"{"message":"/memory set fact.launch_code \"aurora-7\""}"#,
        &snapshot,
    );
    let memory_set_status = response_status(memory_set.as_ref());
    let memory_set_payload = response_payload(memory_set.as_ref());
    record_gauntlet_task(
        &mut results,
        "06_memory_set",
        memory_set_status == 200 && tool_used(&memory_set_payload, "memory_kv_set"),
        format!("status={memory_set_status}"),
    );

    // 07) Query semantic memory using natural conversational retrieval.
    let memory_query = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        br#"{"message":"what did we decide about launch code aurora?"}"#,
        &snapshot,
    );
    let memory_query_status = response_status(memory_query.as_ref());
    let memory_query_payload = response_payload(memory_query.as_ref());
    let memory_query_response = clean_text(
        memory_query_payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or(""),
        400,
    )
    .to_ascii_lowercase();
    record_gauntlet_task(
        &mut results,
        "07_memory_semantic_query",
        memory_query_status == 200
            && tool_used(&memory_query_payload, "memory_semantic_query")
            && (memory_query_response.contains("aurora")
                || memory_query_response.contains("launch")),
        format!("status={memory_query_status} response={memory_query_response}"),
    );

    // 08) Inspect active context telemetry with pruning controls.
    let context = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/command"),
        br#"{"command":"context","silent":true}"#,
        &snapshot,
    );
    let context_status = response_status(context.as_ref());
    let context_payload = response_payload(context.as_ref());
    record_gauntlet_task(
        &mut results,
        "08_context_command_surface",
        context_status == 200
            && context_payload.get("ok").and_then(Value::as_bool) == Some(true)
            && context_payload
                .pointer("/context_pool/pre_generation_pruning_enabled")
                .and_then(Value::as_bool)
                == Some(true),
        format!("status={context_status}"),
    );

    // 09) Trigger emergency auto-compaction under high context pressure.
    let _ = update_profile_patch(
        root.path(),
        &parent_id,
        &json!({"context_window": 512, "context_window_tokens": 512}),
    );
    let session_path =
        state_path(root.path(), AGENT_SESSIONS_DIR_REL).join(format!("{parent_id}.json"));
    let noisy_messages = (0..80)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("context-bloat-{idx} {}", "alpha ".repeat(40)),
                "ts": crate::now_iso()
            })
        })
        .collect::<Vec<_>>();
    write_json(
        &session_path,
        &json!({
            "agent_id": parent_id,
            "active_session_id": "default",
            "sessions": [
                {
                    "session_id": "default",
                    "updated_at": crate::now_iso(),
                    "messages": noisy_messages
                }
            ]
        }),
    );
    let compact_message = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        br#"{"message":"run context compaction","active_context_target_tokens":512,"active_context_min_recent_messages":4,"auto_compact_threshold_ratio":0.95,"auto_compact_target_ratio":0.45}"#,
        &snapshot,
    );
    let compact_status = response_status(compact_message.as_ref());
    let compact_payload = response_payload(compact_message.as_ref());
    record_gauntlet_task(
        &mut results,
        "09_emergency_context_compaction",
        compact_status == 200
            && compact_payload
                .pointer("/context_pool/emergency_compact/triggered")
                .and_then(Value::as_bool)
                == Some(true)
            && compact_payload
                .pointer("/context_pool/emergency_compact/removed_messages")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0,
        format!("status={compact_status}"),
    );

    // 10) Spawn subagents with governance constraints (budget/depth/circuit breaker).
    let spawn = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        &parent_id,
        None,
        "spawn_subagents",
        &json!({
            "count": 8,
            "objective": "Parallelize large architecture analysis",
            "merge_strategy": "voting",
            "budget_tokens": 1_000_000,
            "confirm": true,
            "approval_note": "user requested bounded spawn for analysis"
        }),
    );
    let effective_count = spawn
        .get("effective_count")
        .and_then(Value::as_u64)
        .unwrap_or(999);
    let degraded = spawn
        .pointer("/circuit_breakers/degraded")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    record_gauntlet_task(
        &mut results,
        "10_spawn_governance_circuit_breaker",
        effective_count <= 1
            && degraded
            && spawn
                .pointer("/directive/merge_strategy")
                .and_then(Value::as_str)
                == Some("voting"),
        format!("effective_count={effective_count} degraded={degraded}"),
    );

    // 11) Create descendant child agent bound to parent.
    let child_create = handle(
        root.path(),
        "POST",
        "/api/agents",
        format!(
            "{{\"name\":\"Gauntlet Child\",\"role\":\"analyst\",\"parent_agent_id\":\"{}\"}}",
            parent_id
        )
        .as_bytes(),
        &snapshot,
    );
    let child_status = response_status(child_create.as_ref());
    let child_id = child_create
        .as_ref()
        .and_then(|row| row.payload.get("agent_id").and_then(Value::as_str))
        .map(|row| clean_text(row, 180))
        .unwrap_or_default();
    record_gauntlet_task(
        &mut results,
        "11_create_descendant_agent",
        child_status == 200 && !child_id.is_empty(),
        format!("status={child_status} child_id={child_id}"),
    );

    // 12) Parent can manage descendant lifecycle.
    let allowed_manage = handle_with_headers(
        root.path(),
        "POST",
        &format!("/api/agents/{child_id}/stop"),
        br#"{}"#,
        &[("X-Actor-Agent-Id", parent_id.as_str())],
        &snapshot,
    );
    let allowed_status = response_status(allowed_manage.as_ref());
    let allowed_ok = allowed_manage
        .as_ref()
        .and_then(|row| row.payload.get("ok").and_then(Value::as_bool))
        .unwrap_or(false);
    record_gauntlet_task(
        &mut results,
        "12_parent_manages_descendant",
        allowed_status == 200 && allowed_ok,
        format!("status={allowed_status} ok={allowed_ok}"),
    );

    // 13) Non-parent actor is blocked from managing another tree.
    let sibling_create = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Gauntlet Sibling","role":"analyst"}"#,
        &snapshot,
    );
    let sibling_id = sibling_create
        .as_ref()
        .and_then(|row| row.payload.get("agent_id").and_then(Value::as_str))
        .map(|row| clean_text(row, 180))
        .unwrap_or_default();
    let denied_manage = handle_with_headers(
        root.path(),
        "POST",
        &format!("/api/agents/{child_id}/start"),
        br#"{}"#,
        &[("X-Actor-Agent-Id", sibling_id.as_str())],
        &snapshot,
    );
    let denied_status = response_status(denied_manage.as_ref());
    let denied_error = denied_manage
        .as_ref()
        .and_then(|row| row.payload.get("error").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();
    record_gauntlet_task(
        &mut results,
        "13_non_parent_manage_blocked",
        denied_status == 403 && denied_error == "agent_manage_forbidden",
        format!("status={denied_status} error={denied_error}"),
    );

    // 14) Schedule a cron task through command surface.
    let cron_schedule = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/command"),
        br#"{"command":"cron","args":"schedule 10m follow up on workflow completion"}"#,
        &snapshot,
    );
    let cron_schedule_status = response_status(cron_schedule.as_ref());
    let cron_schedule_payload = response_payload(cron_schedule.as_ref());
    let cron_job_id = clean_text(
        cron_schedule_payload
            .pointer("/result/job/id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    record_gauntlet_task(
        &mut results,
        "14_cron_schedule_command",
        cron_schedule_status == 200
            && cron_schedule_payload.get("tool").and_then(Value::as_str) == Some("cron_schedule")
            && !cron_job_id.is_empty(),
        format!("status={cron_schedule_status} job_id={cron_job_id}"),
    );

    // 15) List cron jobs through command surface.
    let cron_list = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/command"),
        br#"{"command":"cron","args":"list"}"#,
        &snapshot,
    );
    let cron_list_status = response_status(cron_list.as_ref());
    let cron_list_payload = response_payload(cron_list.as_ref());
    let cron_rows = cron_list_payload
        .pointer("/result/jobs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    record_gauntlet_task(
        &mut results,
        "15_cron_list_command",
        cron_list_status == 200 && !cron_rows.is_empty(),
        format!("status={cron_list_status} jobs={}", cron_rows.len()),
    );

    // 16) Undo last turn with receipted rollback.
    let rollback = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        &parent_id,
        None,
        "session_rollback_last_turn",
        &json!({}),
    );
    let removed_count = rollback
        .get("removed_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let rollback_ok = rollback.get("ok").and_then(Value::as_bool).unwrap_or(false);
    record_gauntlet_task(
        &mut results,
        "16_session_rollback",
        rollback_ok && removed_count >= 1,
        format!("ok={rollback_ok} removed={removed_count}"),
    );

    // 17) Archived agents remain searchable in conversation index.
    let archived_agent_id = "agent-gauntlet-archived";
    let _ = crate::dashboard_agent_state::upsert_profile(
        root.path(),
        archived_agent_id,
        &json!({"name":"Archived Atlas", "identity":{"emoji":"🛰️"}}),
    );
    let _ = crate::dashboard_agent_state::append_turn(
        root.path(),
        archived_agent_id,
        "Please patch websocket reconnect jitter and bottom scroll bounce",
        "I can patch reconnect jitter and scroll bounce.",
    );
    let _ = crate::dashboard_agent_state::archive_agent(root.path(), archived_agent_id, "test");
    let _ = crate::dashboard_agent_state::upsert_contract(
        root.path(),
        archived_agent_id,
        &json!({"status":"terminated","termination_reason":"user_archive"}),
    );
    let archived_search = handle(
        root.path(),
        "GET",
        "/api/search/conversations?q=reconnect%20jitter&limit=5",
        &[],
        &snapshot,
    );
    let archived_status = response_status(archived_search.as_ref());
    let archived_payload = response_payload(archived_search.as_ref());
    let archived_found = archived_payload
        .get("results")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("agent_id").and_then(Value::as_str) == Some(archived_agent_id)
                    && row.get("archived").and_then(Value::as_bool) == Some(true)
            })
        })
        .unwrap_or(false);
    record_gauntlet_task(
        &mut results,
        "17_archived_search_visibility",
        archived_status == 200 && archived_found,
        format!("status={archived_status} found={archived_found}"),
    );

    // 18) Latent tool discovery maps natural intent + workspace hints.
    let profile = json!({
        "workspace_dir": root.path().to_string_lossy().to_string()
    });
    let hints = workspace_file_hints_for_message(
        root.path(),
        Some(&profile),
        "I'm worried about security in this API module",
        5,
    );
    let latent =
        latent_tool_candidates_for_message("Please audit the security of this API code", &hints);
    let latent_tools = latent
        .iter()
        .filter_map(|row| row.get("tool").and_then(Value::as_str))
        .collect::<Vec<_>>();
    record_gauntlet_task(
        &mut results,
        "18_latent_tool_discovery",
        !hints.is_empty()
            && latent_tools.contains(&"terminal_exec")
            && latent_tools.contains(&"file_read"),
        format!("hints={} latent={}", hints.len(), latent_tools.join(",")),
    );

    // 19) Terminal tooling runs without signoff and still enforces command policy deny rules.
    let terminal_allow = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        &parent_id,
        None,
        "terminal_exec",
        &json!({"command":"echo hi"}),
    );
    let terminal_deny = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        &parent_id,
        None,
        "terminal_exec",
        &json!({"command":"git reset --hard HEAD"}),
    );
    let no_signoff_gate = terminal_allow.get("error").and_then(Value::as_str)
        != Some("tool_explicit_signoff_required");
    let denied_by_policy = terminal_deny
        .pointer("/permission_gate/verdict")
        .and_then(Value::as_str)
        == Some("deny")
        && terminal_deny.get("blocked").and_then(Value::as_bool) == Some(true);
    record_gauntlet_task(
        &mut results,
        "19_terminal_policy_gate",
        no_signoff_gate && denied_by_policy,
        format!("no_signoff_gate={no_signoff_gate} denied_by_policy={denied_by_policy}"),
    );

    // 20) Proactive operational surfaces stay available (alerts + continuity).
    let alerts = handle(root.path(), "GET", "/api/telemetry/alerts", &[], &snapshot);
    let continuity = handle(root.path(), "GET", "/api/continuity", &[], &snapshot);
    let alerts_ok = alerts
        .as_ref()
        .map(|row| {
            row.status == 200 && row.payload.get("ok").and_then(Value::as_bool) == Some(true)
        })
        .unwrap_or(false);
    let continuity_ok = continuity
        .as_ref()
        .map(|row| {
            row.status == 200 && row.payload.get("ok").and_then(Value::as_bool) == Some(true)
        })
        .unwrap_or(false);
    record_gauntlet_task(
        &mut results,
        "20_alerts_and_continuity_surfaces",
        alerts_ok && continuity_ok,
        format!("alerts_ok={alerts_ok} continuity_ok={continuity_ok}"),
    );

    assert_eq!(results.len(), 20, "gauntlet must run exactly 20 tasks");
    let failed = results
        .iter()
        .filter(|(_, pass, _)| !*pass)
        .map(|(task, _, detail)| format!("{task} -> {detail}"))
        .collect::<Vec<_>>();
    assert!(
        failed.is_empty(),
        "agent gauntlet failures:\n{}",
        failed.join("\n")
    );
}
