
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


