#[test]
fn persona_blocked_path_fails_closed_before_security_core() {
    let _guard = env_guard();
    std::env::set_var("PROTHEUS_CTL_SECURITY_GATE_DISABLED", "0");
    std::env::set_var(
        "PROTHEUS_CTL_PERSONA_BLOCKED_PATHS",
        "client/runtime/systems/ops/protheus_control_plane.js",
    );
    let root = PathBuf::from(".");
    let verdict = evaluate_dispatch_security(
        &root,
        "client/runtime/systems/ops/protheus_control_plane.js",
        &[],
    );
    assert!(!verdict.ok);
    assert!(verdict
        .reason
        .contains(PERSONA_DISPATCH_SECURITY_GATE_CHECK_ID));
    assert!(verdict.reason.contains("blocked_dispatch_path"));
    std::env::remove_var("PROTHEUS_CTL_PERSONA_BLOCKED_PATHS");
    std::env::remove_var("PROTHEUS_CTL_SECURITY_GATE_DISABLED");
}

#[test]
fn requested_lens_arg_supports_inline_and_pair_forms() {
    let inline = requested_lens_arg(&["--lens=guardian".to_string()]);
    assert_eq!(inline.as_deref(), Some("guardian"));

    let paired = requested_lens_arg(&["--persona-lens".to_string(), "operator".to_string()]);
    assert_eq!(paired.as_deref(), Some("operator"));
}

#[test]
fn command_center_boundary_allows_core_session_route() {
    let route = Route {
        script_rel: "core://command-center-session".to_string(),
        args: vec!["resume".to_string(), "session-1".to_string()],
        forward_stdin: false,
    };
    assert!(enforce_command_center_boundary("session", &route).is_ok());
}

#[test]
fn command_center_boundary_rejects_client_red_legion_authority() {
    let route = Route {
        script_rel: "client/runtime/systems/red_legion/command_center.ts".to_string(),
        args: vec!["resume".to_string(), "session-1".to_string()],
        forward_stdin: false,
    };
    let err = enforce_command_center_boundary("session", &route).expect_err("must reject");
    assert!(err.contains("red_legion_client_authority_forbidden"));
}

#[test]
fn command_center_boundary_rejects_non_core_session_route() {
    let route = Route {
        script_rel: "client/runtime/systems/ops/protheusd.js".to_string(),
        args: vec!["status".to_string()],
        forward_stdin: false,
    };
    let err = enforce_command_center_boundary("session", &route).expect_err("must reject");
    assert!(err.contains("session_route_must_be_core_authoritative"));
}

#[test]
fn session_route_supports_extended_lifecycle_commands() {
    let route = Route {
        script_rel: "core://command-center-session".to_string(),
        args: vec!["kill".to_string(), "session-9".to_string()],
        forward_stdin: false,
    };
    assert!(enforce_command_center_boundary("session", &route).is_ok());
}

#[test]
fn node_missing_fallback_supports_help_surface() {
    let route = Route {
        script_rel: "client/runtime/systems/ops/protheus_command_list.js".to_string(),
        args: vec!["--mode=help".to_string()],
        forward_stdin: false,
    };
    assert_eq!(node_missing_fallback(Path::new("."), &route, true), Some(0));
}

#[test]
fn node_missing_fallback_supports_version_surface() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let base = std::env::temp_dir().join(format!("protheusctl_version_fallback_{nonce}"));
    fs::create_dir_all(&base).expect("mkdir");
    fs::write(base.join("package.json"), r#"{"version":"9.9.9-test"}"#).expect("package");
    assert_eq!(
        workspace_package_version(&base).as_deref(),
        Some("9.9.9-test")
    );
    let route = Route {
        script_rel: "client/runtime/systems/ops/protheus_version_cli.js".to_string(),
        args: vec!["version".to_string()],
        forward_stdin: false,
    };
    assert_eq!(node_missing_fallback(&base, &route, true), Some(0));
    let _ = fs::remove_dir_all(base);
}

#[test]
fn node_missing_fallback_supports_version_surface_ts_route() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let base = std::env::temp_dir().join(format!("protheusctl_version_fallback_ts_{nonce}"));
    fs::create_dir_all(&base).expect("mkdir");
    fs::write(base.join("package.json"), r#"{"version":"1.2.3-ts"}"#).expect("package");
    let route = Route {
        script_rel: "client/runtime/systems/ops/protheus_version_cli.ts".to_string(),
        args: vec!["version".to_string()],
        forward_stdin: false,
    };
    assert_eq!(node_missing_fallback(&base, &route, true), Some(0));
    let _ = fs::remove_dir_all(base);
}

#[test]
fn node_missing_fallback_supports_update_surface() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let base = std::env::temp_dir().join(format!("protheusctl_update_fallback_{nonce}"));
    fs::create_dir_all(&base).expect("mkdir");
    fs::write(base.join("package.json"), r#"{"version":"2.4.6-update"}"#).expect("package");
    let route = Route {
        script_rel: "client/runtime/systems/ops/protheus_version_cli.ts".to_string(),
        args: vec!["update".to_string()],
        forward_stdin: false,
    };
    assert_eq!(node_missing_fallback(&base, &route, true), Some(0));
    let _ = fs::remove_dir_all(base);
}

#[test]
fn node_missing_fallback_is_none_for_non_fallback_routes() {
    let route = Route {
        script_rel: "client/runtime/systems/ops/protheus_diagram.js".to_string(),
        args: vec!["status".to_string()],
        forward_stdin: false,
    };
    assert_eq!(node_missing_fallback(Path::new("."), &route, false), None);
}

#[test]
fn run_node_script_falls_back_when_command_list_script_is_missing() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let base = std::env::temp_dir().join(format!("protheusctl_missing_script_fallback_{nonce}"));
    fs::create_dir_all(base.join("client/runtime/systems/ops")).expect("mkdir");

    let status = run_node_script(
        &base,
        "client/runtime/systems/ops/protheus_command_list.js",
        &["--mode=list".to_string()],
        false,
    );
    assert_eq!(status, 0, "expected fallback command list to succeed");

    let _ = fs::remove_dir_all(base);
}

#[test]
fn run_node_script_dist_mode_fails_closed_on_ts_only_entrypoint() {
    let _guard = env_guard();
    std::env::remove_var("PROTHEUS_RUNTIME_MODE");
    std::env::remove_var("PROTHEUS_RUNTIME_MODE_STATE_PATH");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let base = std::env::temp_dir().join(format!("protheusctl_dist_mode_mismatch_{nonce}"));
    fs::create_dir_all(base.join("client/runtime/systems/ops")).expect("mkdir");
    fs::create_dir_all(base.join("local/state/ops")).expect("state");
    fs::write(
        base.join("client/runtime/systems/ops/protheus_command_list.ts"),
        "export {};",
    )
    .expect("ts entry");
    fs::write(
        base.join("local/state/ops/runtime_mode.json"),
        r#"{"mode":"dist"}"#,
    )
    .expect("runtime mode");

    let status = run_node_script(
        &base,
        "client/runtime/systems/ops/protheus_command_list.js",
        &["--mode=list".to_string()],
        false,
    );
    assert_eq!(status, 1, "dist mode must not execute ts fallback");

    let _ = fs::remove_dir_all(base);
    std::env::remove_var("PROTHEUS_RUNTIME_MODE");
    std::env::remove_var("PROTHEUS_RUNTIME_MODE_STATE_PATH");
}

#[test]
fn runtime_missing_entrypoints_dist_mode_requires_js_assets() {
    let _guard = env_guard();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let base = std::env::temp_dir().join(format!("protheusctl_runtime_manifest_dist_{nonce}"));
    fs::create_dir_all(base.join("client/runtime/config")).expect("config");
    fs::create_dir_all(base.join("client/runtime/systems/ops")).expect("ops");
    fs::write(
        base.join("client/runtime/config/install_runtime_manifest_v1.txt"),
        "client/runtime/systems/ops/protheus_command_list.js\n",
    )
    .expect("manifest");
    fs::write(
        base.join("client/runtime/systems/ops/protheus_command_list.ts"),
        "export {};",
    )
    .expect("ts entry");

    let source_missing = runtime_missing_entrypoints_for_mode(&base, "source");
    assert!(
        source_missing.is_empty(),
        "source mode should allow js/ts fallback"
    );
    let dist_missing = runtime_missing_entrypoints_for_mode(&base, "dist");
    assert_eq!(
        dist_missing,
        vec!["client/runtime/systems/ops/protheus_command_list.js".to_string()],
        "dist mode must require bundled js entrypoint"
    );

    let _ = fs::remove_dir_all(base);
}

#[test]
fn dispatch_security_embedded_engine_passes_without_cargo_manifest() {
    let _guard = env_guard();
    std::env::set_var("PROTHEUS_CTL_SECURITY_GATE_DISABLED", "0");
    std::env::set_var("PROTHEUS_CTL_SECURITY_DISABLE_CARGO_FALLBACK", "1");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let base = std::env::temp_dir().join(format!("protheusctl_embedded_security_{nonce}"));
    fs::create_dir_all(&base).expect("mkdir");

    let verdict =
        evaluate_dispatch_security(&base, "core://daemon-control", &["status".to_string()]);
    assert!(
        verdict.ok,
        "embedded security engine should avoid cargo/rustup dependency: {}",
        verdict.reason
    );

    let _ = fs::remove_dir_all(base);
    std::env::remove_var("PROTHEUS_CTL_SECURITY_DISABLE_CARGO_FALLBACK");
    std::env::remove_var("PROTHEUS_CTL_SECURITY_GATE_DISABLED");
}

#[test]
fn core_domain_nexus_tool_label_routes_web_domains_to_web_search() {
    assert_eq!(
        core_domain_nexus_tool_label("web-conduit", &[]),
        "web_search"
    );
    assert_eq!(
        core_domain_nexus_tool_label(
            "web-conduit",
            &[
                "--provider-plugin-id=brave".to_string(),
                "--contract=webSearchProviders".to_string(),
            ],
        ),
        "web_search"
    );
}

#[test]
fn core_domain_nexus_tool_label_routes_memory_domains_to_memory_lane() {
    assert_eq!(
        core_domain_nexus_tool_label("continuity-runtime", &[]),
        "batch_query"
    );
    assert_eq!(
        core_domain_nexus_tool_label(
            "continuity-runtime",
            &["--contract=memoryEmbeddingProviders".to_string()],
        ),
        "batch_query"
    );
}
