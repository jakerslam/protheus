fn run_auth_login(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let provider = clean_id(parsed.flags.get("provider").map(String::as_str), "default");
    let profile = clean_id(parsed.flags.get("profile").map(String::as_str), "default");
    let vault = load_auth_vault(root);
    let selected = vault
        .get("profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|row| {
            row.get("provider").and_then(Value::as_str) == Some(provider.as_str())
                && row.get("profile").and_then(Value::as_str) == Some(profile.as_str())
        });
    let Some(entry) = selected else {
        return json!({
            "ok": !strict,
            "strict": strict,
            "type": "vbrowser_plane_auth_login",
            "lane": "core/layer0/ops",
            "error": "profile_not_found",
            "provider": provider,
            "profile": profile
        });
    };
    let secret = entry
        .get("secret")
        .and_then(|v| decrypt_secret(root, v))
        .unwrap_or_default();
    if strict && secret.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_auth_login",
            "lane": "core/layer0/ops",
            "error": "decrypt_failed",
            "provider": provider,
            "profile": profile
        });
    }
    let token = sha256_hex_str(&format!("{}:{}:{}", provider, profile, secret));
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_auth_login",
        "lane": "core/layer0/ops",
        "provider": provider,
        "profile": profile,
        "session_token_hint": &token[..16],
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-002.4",
                "claim": "auth_profiles_enable_deterministic_login_without_plaintext_secret_exposure",
                "evidence": {"provider": provider, "profile": profile}
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_native(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let url = clean(
        parsed
            .flags
            .get("url")
            .cloned()
            .unwrap_or_else(|| "about:blank".to_string()),
        400,
    );
    let session = json!({
        "version": "v1",
        "session_id": sid,
        "target_url": url,
        "origin": "protheusctl-browser-native",
        "native_mode": true,
        "host_state_access": false,
        "started_at": crate::now_iso()
    });
    let path = session_state_path(root, &sid);
    let _ = write_json(&path, &session);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_native",
        "lane": "core/layer0/ops",
        "session": session,
        "artifact": {"path": path.display().to_string()},
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-002.5",
                "claim": "native_cli_browser_surface_routes_to_core_vbrowser_runtime",
                "evidence": {"session_id": sid}
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

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
                "type": "vbrowser_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "session-start" | "start" | "open" => run_session_start(root, &parsed, strict),
        "session-control" | "control" => run_session_control(root, &parsed, strict),
        "automate" => run_automate(root, &parsed, strict),
        "privacy-guard" | "privacy" => run_privacy_guard(root, &parsed, strict),
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
            "command": command
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
}

