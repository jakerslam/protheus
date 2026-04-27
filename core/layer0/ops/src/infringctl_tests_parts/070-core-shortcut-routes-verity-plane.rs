
fn assert_core_route(cmd: &str, rest: &[&str], expected_script: &str, expected_args: &[&str]) {
    let rest = rest.iter().map(|token| (*token).to_string()).collect::<Vec<_>>();
    let route = resolve_core_shortcuts(cmd, &rest).expect("route");
    assert_eq!(route.script_rel, expected_script);
    assert_eq!(
        route.args,
        expected_args
            .iter()
            .map(|token| (*token).to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn core_shortcut_routes_verity_default_to_status_on_verity_plane() {
    assert_core_route("verity", &[], "core://verity-plane", &["status"]);
}

#[test]
fn core_shortcut_routes_verity_drift_to_verity_plane_drift_status() {
    assert_core_route(
        "verity",
        &["drift", "--limit=5"],
        "core://verity-plane",
        &["drift-status", "--limit=5"],
    );
}

#[test]
fn core_shortcut_routes_top_level_dream_to_autonomy_controller() {
    assert_core_route(
        "dream",
        &["--hand-id=agent-1"],
        "core://autonomy-controller",
        &["dream", "--hand-id=agent-1"],
    );
}

#[test]
fn core_shortcut_routes_top_level_compact_to_autonomy_controller() {
    assert_core_route(
        "compact",
        &["reactive", "--strict=1"],
        "core://autonomy-controller",
        &["compact", "reactive", "--strict=1"],
    );
}

#[test]
fn core_shortcut_routes_top_level_proactive_daemon_to_autonomy_controller() {
    assert_core_route(
        "proactive_daemon",
        &["cycle", "--auto=1"],
        "core://autonomy-controller",
        &["proactive_daemon", "cycle", "--auto=1"],
    );
}

#[test]
fn core_shortcut_routes_top_level_speculate_to_autonomy_controller() {
    assert_core_route(
        "speculate",
        &["status"],
        "core://autonomy-controller",
        &["speculate", "status"],
    );
}

#[test]
fn core_shortcut_routes_dashboard_ui_serve_to_daemon_control_start_with_flag_normalization() {
    let route = resolve_core_shortcuts(
        "dashboard-ui",
        &[
            "serve".to_string(),
            "--host=0.0.0.0".to_string(),
            "--port=4310".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://daemon-control");
    assert_eq!(
        route.args,
        vec![
            "start",
            "--dashboard-host=0.0.0.0",
            "--dashboard-port=4310",
            "--dashboard-open=0"
        ]
    );
}

#[test]
fn core_shortcut_routes_dashboard_alias_to_daemon_control_start() {
    let route = resolve_core_shortcuts("dashboard", &[]).expect("route");
    assert_eq!(route.script_rel, "core://daemon-control");
    assert_eq!(route.args, vec!["start", "--dashboard-open=0"]);
}

#[test]
fn core_shortcut_routes_dashboard_alias_respects_explicit_open_opt_in() {
    let route = resolve_core_shortcuts("dashboard", &["--dashboard-open=1".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://daemon-control");
    assert_eq!(route.args, vec!["start", "--dashboard-open=1"]);
}

#[test]
fn core_shortcut_routes_doctor_to_install_doctor_domain() {
    let route = resolve_core_shortcuts("doctor", &[]).expect("route");
    assert_eq!(route.script_rel, "core://install-doctor");
    assert_eq!(route.args, vec!["doctor"]);
}

#[test]
fn core_shortcut_routes_verify_install_to_install_doctor_domain() {
    let route = resolve_core_shortcuts("verify-install", &["--json=1".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://install-doctor");
    assert_eq!(route.args, vec!["verify-install", "--json=1"]);
}

#[test]
fn core_shortcut_routes_help_to_command_list_core_domain() {
    let route = resolve_core_shortcuts("help", &[]).expect("route");
    assert_eq!(route.script_rel, "core://command-list");
    assert_eq!(route.args, vec!["--mode=help"]);
}

#[test]
fn core_shortcut_routes_completion_to_core_domain() {
    let route = resolve_core_shortcuts("completion", &[]).expect("route");
    assert_eq!(route.script_rel, "core://completion");
    assert_eq!(route.args, vec!["--help"]);
}

#[test]
fn core_shortcut_routes_repl_to_core_domain() {
    let route = resolve_core_shortcuts("repl", &[]).expect("route");
    assert_eq!(route.script_rel, "core://repl");
    assert!(route.args.is_empty());
}

#[test]
fn core_shortcut_routes_version_and_update_to_core_version_domain() {
    let version = resolve_core_shortcuts("version", &[]).expect("version route");
    assert_eq!(version.script_rel, "core://version-cli");
    assert_eq!(version.args, vec!["version"]);

    let version_alias = resolve_core_shortcuts("--version", &[]).expect("version alias route");
    assert_eq!(version_alias.script_rel, "core://version-cli");
    assert_eq!(version_alias.args, vec!["version"]);

    let update = resolve_core_shortcuts("update", &["--json=1".to_string()]).expect("update route");
    assert_eq!(update.script_rel, "core://version-cli");
    assert_eq!(update.args, vec!["update", "--json=1"]);
}

#[test]
fn core_shortcut_routes_health_and_job_submit_to_infring_control_plane() {
    let health = resolve_core_shortcuts("health", &[]).expect("health route");
    assert_eq!(health.script_rel, "core://infring-control-plane");
    assert_eq!(health.args, vec!["status"]);

    let job = resolve_core_shortcuts("job-submit", &["--id=lane-1".to_string()])
        .expect("job-submit route");
    assert_eq!(job.script_rel, "core://infring-control-plane");
    assert_eq!(job.args, vec!["run", "--id=lane-1"]);
}

#[test]
fn core_shortcut_routes_stack_default_to_context_stacks_list() {
    let route = resolve_core_shortcuts("stack", &[]).expect("route");
    assert_eq!(route.script_rel, "core://context-stacks");
    assert_eq!(route.args, vec!["list"]);
}

#[test]
fn core_shortcut_routes_context_stacks_passthrough_subcommand() {
    let route = resolve_core_shortcuts(
        "context-stacks",
        &["create".to_string(), "--stack-id=demo".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://context-stacks");
    assert_eq!(route.args, vec!["create", "--stack-id=demo"]);
}

#[test]
fn core_shortcut_routes_workspace_search_default_to_list() {
    let route = resolve_core_shortcuts("workspace-search", &[]).expect("route");
    assert_eq!(route.script_rel, "core://workspace-file-search");
    assert_eq!(route.args, vec!["list"]);
}

#[test]
fn core_shortcut_routes_workspace_files_passthrough_subcommand() {
    let route = resolve_core_shortcuts(
        "workspace-files",
        &["search".to_string(), "--q=router".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://workspace-file-search");
    assert_eq!(route.args, vec!["search", "--q=router"]);
}

#[test]
fn core_shortcut_routes_batch_query_default_to_status() {
    let route = resolve_core_shortcuts("batch-query", &[]).expect("route");
    assert_eq!(route.script_rel, "core://batch-query");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_batch_alias_passthrough_to_batch_query_domain() {
    let route = resolve_core_shortcuts(
        "batch",
        &[
            "query".to_string(),
            "--source=web".to_string(),
            "--query=tool hit rate".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://batch-query");
    assert_eq!(
        route.args,
        vec!["query", "--source=web", "--query=tool hit rate"]
    );
}

#[test]
fn tier1_route_contracts_resolve_to_expected_core_targets() {
    for row in crate::command_list_kernel::tier1_route_contracts() {
        let rest = row
            .rest
            .iter()
            .map(|token| token.to_string())
            .collect::<Vec<_>>();
        let route = resolve_core_shortcuts(row.cmd, &rest).expect("tier1 contract route");
        assert_eq!(
            route.script_rel, row.expected_script,
            "tier1 route mismatch for {}",
            row.cmd
        );
    }
}

#[test]
fn tier1_runtime_entrypoints_align_with_install_fallback_manifest() {
    let mut expected = crate::command_list_kernel::tier1_runtime_entrypoints();
    expected.sort_unstable();
    let mut fallback = INSTALL_RUNTIME_FALLBACK_ENTRYPOINTS
        .iter()
        .map(|row| (*row).to_string())
        .collect::<Vec<_>>();
    fallback.sort_unstable();
    assert_eq!(expected, fallback);
}
