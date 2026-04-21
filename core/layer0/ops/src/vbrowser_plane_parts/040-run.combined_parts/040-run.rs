
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let raw_command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(raw_command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = canonical_vbrowser_command(&raw_command).to_string();

    let strict = parse_bool(parsed.flags.get("strict"), true);
    let conduit = if command != "status" {
        Some(conduit_enforcement(root, &parsed, strict, &command))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": vbrowser_receipt_type(&command),
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "session-start" => run_session_start(root, &parsed, strict),
        "session-control" => run_session_control(root, &parsed, strict),
        "goto" => run_goto(root, &parsed, strict),
        "navback" => run_navback(root, &parsed, strict),
        "wait" => run_wait(root, &parsed, strict),
        "scroll" => run_scroll(root, &parsed, strict),
        "click" => run_click(root, &parsed, strict),
        "type" => run_type(root, &parsed, strict),
        "automate" => run_automate(root, &parsed, strict),
        "key-input" => run_key_input(root, &parsed, strict),
        "privacy-guard" => run_privacy_guard(root, &parsed, strict),
        "snapshot" => run_snapshot(root, &parsed, strict),
        "screenshot" => run_screenshot(root, &parsed, strict),
        "action-policy" => run_action_policy(root, &parsed, strict),
        "auth-save" => run_auth_save(root, &parsed, strict),
        "auth-login" => run_auth_login(root, &parsed, strict),
        "native" => run_native(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "vbrowser_plane_error",
            "error": "unknown_command",
            "command": command,
            "requested_command": raw_command
        }),
    };
    if command == "status" {
        print_json(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_defaults() {
        let parsed = crate::parse_args(&["status".to_string()]);
        assert_eq!(session_id(&parsed), "browser-session");
    }

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["start".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "session-start");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn alias_commands_are_canonicalized_for_claims() {
        assert_eq!(canonical_vbrowser_command("open"), "session-start");
        assert_eq!(canonical_vbrowser_command("navigate"), "goto");
        assert_eq!(canonical_vbrowser_command("control"), "session-control");
        assert_eq!(canonical_vbrowser_command("keys"), "key-input");
        assert_eq!(canonical_vbrowser_command("privacy"), "privacy-guard");
    }

    #[test]
    fn alias_bypass_emits_action_specific_receipt_type() {
        let root = tempfile::tempdir().expect("tempdir");
        let exit = run(
            root.path(),
            &[
                "open".to_string(),
                "--strict=1".to_string(),
                "--bypass=1".to_string(),
            ],
        );
        assert_eq!(exit, 1);
        let latest = crate::v8_kernel::read_json(&state_root(root.path()).join("latest.json"))
            .expect("latest vbrowser receipt");
        assert_eq!(
            latest.get("type").and_then(Value::as_str),
            Some("vbrowser_plane_session_start")
        );
        assert_eq!(
            latest
                .pointer("/conduit_enforcement/action")
                .and_then(Value::as_str),
            Some("session-start")
        );
    }
}

