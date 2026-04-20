
pub fn save_provider_key(root: &Path, provider_id: &str, key: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let secret = clean_text(key, 4096);
    if provider.is_empty() || secret.is_empty() || provider == "auto" {
        return json!({"ok": false, "error": "provider_key_invalid"});
    }
    let mut secrets = load_secrets(root);
    if secrets.get("providers").is_none()
        || !secrets
            .get("providers")
            .map(Value::is_object)
            .unwrap_or(false)
    {
        secrets["providers"] = json!({});
    }
    secrets["providers"][provider.clone()] = json!({"key": secret, "updated_at": crate::now_iso()});
    save_secrets(root, secrets);

    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["auth_status"] = json!("configured");
    row["key_prefix"] = json!(masked_prefix(key));
    row["key_last4"] = json!(masked_last4(key));
    row["key_hash"] = json!(crate::deterministic_receipt_hash(
        &json!({"provider": provider, "key": key})
    ));
    row["key_set_at"] = json!(crate::now_iso());
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    json!({
        "ok": true,
        "provider": provider,
        "auth_status": "configured",
        "switched_default": false
    })
}

pub fn remove_provider_key(root: &Path, provider_id: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let mut secrets = load_secrets(root);
    if let Some(obj) = secrets.get_mut("providers").and_then(Value::as_object_mut) {
        obj.remove(&provider);
    }
    save_secrets(root, secrets);
    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["auth_status"] = json!(if provider_is_local(&provider) {
        "configured"
    } else {
        "not_set"
    });
    row["key_prefix"] = json!("");
    row["key_last4"] = json!("");
    row["key_hash"] = json!("");
    row["key_set_at"] = json!("");
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    json!({"ok": true, "provider": provider})
}

pub fn set_provider_url(root: &Path, provider_id: &str, base_url: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let cleaned = clean_text(base_url, 400);
    if provider.is_empty() || cleaned.is_empty() {
        return json!({"ok": false, "error": "provider_url_invalid"});
    }
    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["base_url"] = json!(cleaned);
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    let probe = test_provider(root, &provider);
    json!({
        "ok": probe.get("status").and_then(Value::as_str) == Some("ok"),
        "provider": provider,
        "reachable": probe.get("status").and_then(Value::as_str) == Some("ok"),
        "latency_ms": probe.get("latency_ms").cloned().unwrap_or_else(|| json!(0)),
        "error": probe.get("error").cloned().unwrap_or(Value::Null)
    })
}
