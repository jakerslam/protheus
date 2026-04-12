fn maybe_set_web_conduit_enabled(policy: &mut Value, enabled: bool, changes: &mut Vec<String>) {
    let current = policy
        .pointer("/web_conduit/enabled")
        .and_then(Value::as_bool);
    if current.is_none() {
        let web_conduit = ensure_child_value(policy, "web_conduit");
        ensure_object_value(web_conduit).insert("enabled".to_string(), Value::Bool(enabled));
        changes.push(format!("Set /web_conduit/enabled to {enabled}."));
    }
}

fn archive_legacy_value(policy: &mut Value, family: &str, key: &str, value: Value) {
    let entry = ensure_legacy_archive_entry(policy, family, key);
    *entry = value;
}

fn migrate_legacy_search_config(policy: &mut Value, root: &Path, changes: &mut Vec<String>) {
    let search_snapshot = policy.pointer("/tools/web/search").cloned();
    let Some(search) = search_snapshot.and_then(|row| row.as_object().cloned()) else {
        return;
    };
    let mut remaining = serde_json::Map::<String, Value>::new();
    let mut selected_provider: Option<String> = None;
    for (key, value) in search {
        match key.as_str() {
            "enabled" => {
                if let Some(enabled) = value.as_bool() {
                    maybe_set_web_conduit_enabled(policy, enabled, changes);
                } else {
                    remaining.insert(key, value);
                }
            }
            "provider" => {
                if let Some(raw) = value.as_str() {
                    if let Some(normalized) = normalize_search_provider_alias(policy, raw) {
                        selected_provider = Some(normalized.clone());
                        changes.push(format!(
                            "Moved tools.web.search.provider -> /web_conduit/search_provider_order ({normalized})."
                        ));
                    } else {
                        archive_legacy_value(
                            policy,
                            "search",
                            "unsupported_provider",
                            Value::String(clean_text(raw, 80)),
                        );
                        changes.push(format!(
                            "Archived unsupported legacy web search provider \"{}\".",
                            clean_text(raw, 80)
                        ));
                    }
                }
            }
            "apiKey" => {
                if let Some(raw) = value.as_str() {
                    let cleaned = clean_text(raw, 600);
                    if !cleaned.is_empty() {
                        let entry = ensure_search_provider_config_entry(policy, "serperdev");
                        ensure_object_value(entry)
                            .insert("api_key".to_string(), Value::String(cleaned));
                        selected_provider.get_or_insert_with(|| "serperdev".to_string());
                        changes.push(
                            "Moved tools.web.search.apiKey -> /web_conduit/search_provider_config/serperdev/api_key."
                                .to_string(),
                        );
                    }
                }
            }
            "apiKeyEnv" => {
                if let Some(raw) = value.as_str() {
                    let cleaned = clean_text(raw, 160);
                    if !cleaned.is_empty() {
                        let entry = ensure_search_provider_config_entry(policy, "serperdev");
                        ensure_object_value(entry)
                            .insert("api_key_env".to_string(), Value::String(cleaned));
                        selected_provider.get_or_insert_with(|| "serperdev".to_string());
                        changes.push(
                            "Moved tools.web.search.apiKeyEnv -> /web_conduit/search_provider_config/serperdev/api_key_env."
                                .to_string(),
                        );
                    }
                }
            }
            other => {
                if let Some(section) = value.as_object() {
                    if let Some(normalized) = normalize_search_provider_alias(policy, other) {
                        let mut consumed = false;
                        if let Some(raw) = section.get("apiKey").and_then(Value::as_str) {
                            let cleaned = clean_text(raw, 600);
                            if !cleaned.is_empty() {
                                let entry =
                                    ensure_search_provider_config_entry(policy, &normalized);
                                ensure_object_value(entry)
                                    .insert("api_key".to_string(), Value::String(cleaned));
                                selected_provider
                                    .get_or_insert_with(|| normalized.clone());
                                changes.push(format!(
                                    "Moved tools.web.search.{other}.apiKey -> /web_conduit/search_provider_config/{normalized}/api_key."
                                ));
                                consumed = true;
                            }
                        }
                        if let Some(raw) = section.get("apiKeyEnv").and_then(Value::as_str) {
                            let cleaned = clean_text(raw, 160);
                            if !cleaned.is_empty() {
                                let entry =
                                    ensure_search_provider_config_entry(policy, &normalized);
                                ensure_object_value(entry).insert(
                                    "api_key_env".to_string(),
                                    Value::String(cleaned),
                                );
                                selected_provider
                                    .get_or_insert_with(|| normalized.clone());
                                changes.push(format!(
                                    "Moved tools.web.search.{other}.apiKeyEnv -> /web_conduit/search_provider_config/{normalized}/api_key_env."
                                ));
                                consumed = true;
                            }
                        }
                        let mut leftover = section.clone();
                        leftover.remove("apiKey");
                        leftover.remove("apiKeyEnv");
                        if !leftover.is_empty() {
                            archive_legacy_value(policy, "search", other, Value::Object(leftover));
                            changes.push(format!(
                                "Archived unsupported legacy search config at tools.web.search.{other}."
                            ));
                        } else if !consumed {
                            archive_legacy_value(policy, "search", other, Value::Object(section.clone()));
                            changes.push(format!(
                                "Archived legacy search provider shape at tools.web.search.{other}."
                            ));
                        }
                    } else {
                        archive_legacy_value(policy, "search", other, Value::Object(section.clone()));
                        changes.push(format!(
                            "Archived unsupported legacy search provider config tools.web.search.{other}."
                        ));
                    }
                } else {
                    remaining.insert(key, value);
                }
            }
        }
    }
    if let Some(provider) = selected_provider {
        let mut order = vec![provider];
        order.extend(normalized_search_policy_order(root, policy));
        set_search_provider_order(policy, &dedupe_strings(order));
    }
    if remaining.is_empty() {
        remove_legacy_tools_key(policy, "search");
    } else {
        let tools = ensure_child_value(policy, "tools");
        let web = ensure_child_value(tools, "web");
        ensure_object_value(web).insert("search".to_string(), Value::Object(remaining));
    }
}

fn migrate_legacy_fetch_config(policy: &mut Value, root: &Path, changes: &mut Vec<String>) {
    let fetch_snapshot = policy.pointer("/tools/web/fetch").cloned();
    let Some(fetch) = fetch_snapshot.and_then(|row| row.as_object().cloned()) else {
        return;
    };
    let mut remaining = serde_json::Map::<String, Value>::new();
    let mut selected_provider: Option<String> = None;
    for (key, value) in fetch {
        match key.as_str() {
            "provider" => {
                if let Some(raw) = value.as_str() {
                    if let Some(normalized) = normalize_fetch_provider_alias(policy, raw) {
                        selected_provider = Some(normalized.clone());
                        changes.push(format!(
                            "Moved tools.web.fetch.provider -> /web_conduit/fetch_provider_order ({normalized})."
                        ));
                    } else {
                        archive_legacy_value(
                            policy,
                            "fetch",
                            "unsupported_provider",
                            Value::String(clean_text(raw, 80)),
                        );
                        changes.push(format!(
                            "Archived unsupported legacy web fetch provider \"{}\".",
                            clean_text(raw, 80)
                        ));
                    }
                }
            }
            "firecrawl" => {
                archive_legacy_value(policy, "fetch", "firecrawl", value);
                changes.push(
                    "Archived unsupported legacy fetch provider config tools.web.fetch.firecrawl."
                        .to_string(),
                );
            }
            other => {
                remaining.insert(other.to_string(), value);
            }
        }
    }
    if let Some(provider) = selected_provider {
        let mut order = vec![provider];
        order.extend(normalized_fetch_policy_order(root, policy));
        set_fetch_provider_order(policy, &dedupe_strings(order));
    }
    if remaining.is_empty() {
        remove_legacy_tools_key(policy, "fetch");
    } else {
        let tools = ensure_child_value(policy, "tools");
        let web = ensure_child_value(tools, "web");
        ensure_object_value(web).insert("fetch".to_string(), Value::Object(remaining));
    }
}

fn read_json_required(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("web_conduit_read_json_failed:{err}"))?;
    serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("web_conduit_parse_json_failed:{err}"))
}

pub fn api_migrate_legacy_config(root: &Path, request: &Value) -> Value {
    let source_path = clean_text(
        request
            .get("source_path")
            .or_else(|| request.get("from_path"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    let path = if source_path.is_empty() {
        policy_path(root)
    } else {
        PathBuf::from(source_path)
    };
    let apply = request.get("apply").and_then(Value::as_bool).unwrap_or(false);
    let summary_only = request
        .get("summary_only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut normalized = match read_json_required(&path) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "web_conduit_legacy_config_migration",
                "error": err,
                "source_path": path.display().to_string()
            });
        }
    };
    let mut changes = Vec::<String>::new();
    migrate_legacy_search_config(&mut normalized, root, &mut changes);
    migrate_legacy_fetch_config(&mut normalized, root, &mut changes);
    if apply {
        if let Err(err) = write_json_atomic(&path, &normalized) {
            return json!({
                "ok": false,
                "type": "web_conduit_legacy_config_migration",
                "error": err,
                "source_path": path.display().to_string(),
                "changes": changes
            });
        }
    }
    let mut payload = json!({
        "ok": true,
        "type": "web_conduit_legacy_config_migration",
        "source_path": path.display().to_string(),
        "applied": apply,
        "changes": changes,
        "migration_contract": {
            "supports_apply": true,
            "source_path_default": policy_path(root).display().to_string(),
            "archives_unsupported_legacy_config": true,
            "commands": [
                "protheus-ops web-conduit migrate-legacy-config --source-path=<path>",
                "protheus-ops web-conduit migrate-legacy-config --source-path=<path> --apply=1"
            ]
        }
    });
    if !summary_only {
        payload["normalized_config"] = normalized;
    }
    payload
}
