
fn run_auth_save(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let provider = clean_id(
        parsed
            .flags
            .get("provider")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("domain").map(String::as_str)),
        "default",
    );
    let profile = clean_id(
        parsed
            .flags
            .get("profile")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("user").map(String::as_str)),
        "default",
    );
    let username = clean(
        parsed
            .flags
            .get("username")
            .cloned()
            .unwrap_or_else(|| "user".to_string()),
        120,
    );
    let secret = parsed.flags.get("secret").cloned().unwrap_or_default();
    if strict && secret.trim().is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_auth_save",
            "lane": "core/layer0/ops",
            "error": "secret_required"
        });
    }
    let encrypted = match encrypt_secret(root, &secret) {
        Some(v) => v,
        None => {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "vbrowser_plane_auth_save",
                "lane": "core/layer0/ops",
                "error": "encrypt_failed"
            });
        }
    };
    let mut vault = load_auth_vault(root);
    if !vault.get("profiles").and_then(Value::as_array).is_some() {
        vault["profiles"] = Value::Array(Vec::new());
    }
    let mut profiles = vault
        .get("profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    profiles.retain(|row| {
        row.get("provider").and_then(Value::as_str) != Some(provider.as_str())
            || row.get("profile").and_then(Value::as_str) != Some(profile.as_str())
    });
    let entry = json!({
        "provider": provider,
        "profile": profile,
        "username": username,
        "secret": encrypted,
        "updated_at": crate::now_iso()
    });
    profiles.push(entry.clone());
    vault["profiles"] = Value::Array(profiles.clone());
    write_auth_vault(root, &vault);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_auth_save",
        "lane": "core/layer0/ops",
        "entry": {
            "provider": provider,
            "profile": profile,
            "username": username
        },
        "profiles_total": profiles.len(),
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-002.4",
                "claim": "auth_profiles_are_saved_in_encrypted_vault_for_reuse",
                "evidence": {"provider": provider, "profile": profile}
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}
