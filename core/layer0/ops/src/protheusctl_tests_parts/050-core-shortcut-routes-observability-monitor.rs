
#[test]
fn core_shortcut_routes_observability_monitor_to_observability_plane() {
    let route = resolve_core_shortcuts(
        "observability",
        &[
            "monitor".to_string(),
            "--severity=high".to_string(),
            "--message=latency_spike".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://observability-plane");
    assert_eq!(
        route.args,
        vec!["monitor", "--severity=high", "--message=latency_spike"]
    );
}

#[test]
fn core_shortcut_routes_observability_selfhost_status_without_forced_deploy() {
    let route = resolve_core_shortcuts(
        "observability",
        &["selfhost".to_string(), "status".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://observability-plane");
    assert_eq!(route.args, vec!["selfhost", "status"]);
}

#[test]
fn core_shortcut_routes_observability_enable_acp_provenance() {
    let route = resolve_core_shortcuts(
        "observability",
        &[
            "enable".to_string(),
            "acp-provenance".to_string(),
            "--visibility-mode=meta".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://observability-plane");
    assert_eq!(
        route.args,
        vec!["acp-provenance", "--op=enable", "--visibility-mode=meta"]
    );
}

#[test]
fn core_shortcut_routes_schedule_to_persist_plane() {
    let route = resolve_core_shortcuts(
        "schedule",
        &[
            "--op=upsert".to_string(),
            "--job=nightly".to_string(),
            "--cron=0 2 * * *".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://persist-plane");
    assert_eq!(
        route.args,
        vec![
            "schedule",
            "--op=upsert",
            "--job=nightly",
            "--cron=0 2 * * *"
        ]
    );
}

#[test]
fn core_shortcut_routes_mobile_to_persist_plane() {
    let route = resolve_core_shortcuts(
        "mobile",
        &["--op=publish".to_string(), "--session-id=phone".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://persist-plane");
    assert_eq!(
        route.args,
        vec!["mobile-cockpit", "--op=publish", "--session-id=phone"]
    );
}

#[test]
fn core_shortcut_routes_mobile_daemon_enable_to_persist_plane() {
    let route = resolve_core_shortcuts(
        "mobile",
        &[
            "daemon".to_string(),
            "enable".to_string(),
            "--platform=android".to_string(),
            "--edge-backend=bitnet".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://persist-plane");
    assert_eq!(
        route.args,
        vec![
            "mobile-daemon",
            "--op=enable",
            "--platform=android",
            "--edge-backend=bitnet"
        ]
    );
}

#[test]
fn core_shortcut_routes_connector_add_to_persist_plane() {
    let route = resolve_core_shortcuts("connector", &["add".to_string(), "slack".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://persist-plane");
    assert_eq!(
        route.args,
        vec!["connector", "--op=add", "--provider=slack"]
    );
}

#[test]
fn core_shortcut_routes_cowork_delegate_to_persist_plane() {
    let route = resolve_core_shortcuts(
        "cowork",
        &[
            "delegate".to_string(),
            "--task=ship-batch16".to_string(),
            "--parent=ops-lead".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://persist-plane");
    assert_eq!(
        route.args,
        vec![
            "cowork",
            "--op=delegate",
            "--task=ship-batch16",
            "--parent=ops-lead"
        ]
    );
}

#[test]
fn core_shortcut_routes_app_run_code_engineer_to_app_plane() {
    let route = resolve_core_shortcuts(
        "app",
        &[
            "run".to_string(),
            "code-engineer".to_string(),
            "build".to_string(),
            "an".to_string(),
            "agent".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://app-plane");
    assert_eq!(
        route.args,
        vec!["run", "--app=code-engineer", "--prompt=build an agent"]
    );
}

#[test]
fn core_shortcut_routes_app_run_chat_ui_to_app_plane() {
    let route = resolve_core_shortcuts(
        "app",
        &[
            "run".to_string(),
            "chat-ui".to_string(),
            "--session-id=s1".to_string(),
            "hello".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://app-plane");
    assert_eq!(
        route.args,
        vec!["run", "--app=chat-ui", "--session-id=s1", "--message=hello"]
    );
}

#[test]
fn core_shortcut_routes_top_level_chat_starter_history_action() {
    let route = resolve_core_shortcuts(
        "chat-starter",
        &["history".to_string(), "--session-id=s1".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://app-plane");
    assert_eq!(
        route.args,
        vec!["history", "--app=chat-starter", "--session-id=s1"]
    );
}

#[test]
fn core_shortcut_routes_top_level_chat_starter_plain_message_to_run() {
    let route = resolve_core_shortcuts(
        "chat-starter",
        &[
            "hello".to_string(),
            "from".to_string(),
            "shortcut".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://app-plane");
    assert_eq!(
        route.args,
        vec!["run", "--app=chat-starter", "--message=hello from shortcut"]
    );
}

#[test]
fn core_shortcut_routes_top_level_chat_ui_switch_provider_action() {
    let route = resolve_core_shortcuts(
        "chat-ui",
        &[
            "switch-provider".to_string(),
            "--provider=anthropic".to_string(),
            "--model=claude-sonnet".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://app-plane");
    assert_eq!(
        route.args,
        vec![
            "switch-provider",
            "--app=chat-ui",
            "--provider=anthropic",
            "--model=claude-sonnet"
        ]
    );
}

#[test]
fn core_shortcut_routes_build_goal_to_app_plane() {
    let route = resolve_core_shortcuts(
        "build",
        &[
            "ship".to_string(),
            "a".to_string(),
            "receipted".to_string(),
            "api".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://app-plane");
    assert_eq!(
        route.args,
        vec![
            "build",
            "--app=code-engineer",
            "--goal=ship a receipted api"
        ]
    );
}

#[test]
fn core_shortcut_routes_snowball_start_to_core_plane() {
    let route = resolve_core_shortcuts(
        "snowball",
        &[
            "start".to_string(),
            "--cycle-id=s17".to_string(),
            "--drops=core-hardening,app-refine".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://snowball-plane");
    assert_eq!(
        route.args,
        vec![
            "start",
            "--cycle-id=s17",
            "--drops=core-hardening,app-refine"
        ]
    );
}

#[test]
fn core_shortcut_routes_snowball_regress_alias_to_melt_refine() {
    let route = resolve_core_shortcuts(
        "snowball",
        &[
            "regress".to_string(),
            "--cycle-id=s35".to_string(),
            "--regression-pass=0".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://snowball-plane");
    assert_eq!(
        route.args,
        vec!["melt-refine", "--cycle-id=s35", "--regression-pass=0"]
    );
}

#[test]
fn core_shortcut_routes_orchestrate_agency_to_company_plane() {
    let route = resolve_core_shortcuts(
        "orchestrate",
        &["agency".to_string(), "research".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://company-plane");
    assert_eq!(route.args, vec!["orchestrate-agency", "--team=research"]);
}

#[test]
fn core_shortcut_routes_browser_snapshot_to_vbrowser_plane() {
    let route = resolve_core_shortcuts(
        "browser",
        &[
            "snapshot".to_string(),
            "--session-id=snap-1".to_string(),
            "--refs=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://vbrowser-plane");
    assert_eq!(
        route.args,
        vec!["snapshot", "--session-id=snap-1", "--refs=1"]
    );
}

#[test]
fn core_shortcut_routes_hand_new_to_autonomy_controller() {
    let route = resolve_core_shortcuts(
        "hand",
        &[
            "new".to_string(),
            "--hand-id=alpha".to_string(),
            "--template=researcher".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://autonomy-controller");
    assert_eq!(
        route.args,
        vec!["hand-new", "--hand-id=alpha", "--template=researcher"]
    );
}

#[test]
fn core_shortcut_routes_hands_enable_scheduled_to_assimilation_controller() {
    let route = resolve_core_shortcuts(
        "hands",
        &[
            "enable".to_string(),
            "scheduled".to_string(),
            "--strict=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://assimilation-controller");
    assert_eq!(
        route.args,
        vec!["scheduled-hands", "--op=enable", "--strict=1"]
    );
}

#[test]
fn core_shortcut_routes_oracle_to_network_protocol() {
    let route = resolve_core_shortcuts(
        "oracle",
        &[
            "query".to_string(),
            "--provider=polymarket".to_string(),
            "--event=btc".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://network-protocol");
    assert_eq!(
        route.args,
        vec!["oracle-query", "--provider=polymarket", "--event=btc"]
    );
}

#[test]
fn core_shortcut_routes_truth_weight_to_network_protocol() {
    let route = resolve_core_shortcuts(
        "truth",
        &["weight".to_string(), "--market=pm:btc-100k".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://network-protocol");
    assert_eq!(route.args, vec!["truth-weight", "--market=pm:btc-100k"]);
}

#[test]
fn core_shortcut_routes_agent_ephemeral_to_autonomy_controller() {
    let route = resolve_core_shortcuts(
        "agent",
        &[
            "run".to_string(),
            "--ephemeral".to_string(),
            "--goal=triage".to_string(),
            "--domain=research".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://autonomy-controller");
    assert_eq!(
        route.args,
        vec!["ephemeral-run", "--goal=triage", "--domain=research"]
    );
}

#[test]
fn core_shortcut_routes_agent_trunk_status_to_autonomy_controller() {
    let route = resolve_core_shortcuts(
        "agent",
        &[
            "status".to_string(),
            "--trunk".to_string(),
            "--strict=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://autonomy-controller");
    assert_eq!(route.args, vec!["trunk-status", "--strict=1"]);
}

#[test]
fn local_fail_closed_signal_blocks_dispatch() {
    let _guard = env_guard();
    std::env::set_var("PROTHEUS_CTL_SECURITY_GATE_DISABLED", "0");
    std::env::set_var("PROTHEUS_CTL_SECURITY_COVENANT_VIOLATION", "1");
    let root = PathBuf::from(".");
    let verdict = evaluate_dispatch_security(
        &root,
        "client/runtime/systems/ops/protheus_control_plane.js",
        &[],
    );
    assert!(!verdict.ok);
    assert!(verdict.reason.contains("fail_closed"));
    std::env::remove_var("PROTHEUS_CTL_SECURITY_COVENANT_VIOLATION");
    std::env::remove_var("PROTHEUS_CTL_SECURITY_GATE_DISABLED");
}

