fn selected_model_param_count_billion(
    root: &Path,
    snapshot: &Value,
    provider_hint: &str,
    model_hint: &str,
) -> i64 {
    let provider_seed = clean_text(provider_hint, 80);
    let model_seed = clean_text(model_hint, 200);
    if model_seed.is_empty() {
        return 0;
    }
    let (resolved_provider, resolved_model) =
        split_model_ref(&model_seed, &provider_seed, &model_seed);
    let provider_key = clean_text(&resolved_provider, 80).to_ascii_lowercase();
    let model_key = clean_text(&resolved_model, 200).to_ascii_lowercase();
    if model_key.is_empty() {
        return 0;
    }
    let mut requested_refs = HashSet::<String>::new();
    requested_refs.insert(model_key.clone());
    if let Some(last) = model_key.rsplit('/').next() {
        if !last.is_empty() {
            requested_refs.insert(last.to_string());
        }
    }
    if !provider_key.is_empty() && provider_key != "auto" {
        requested_refs.insert(format!("{provider_key}/{model_key}"));
        if let Some(last) = model_key.rsplit('/').next() {
            if !last.is_empty() {
                requested_refs.insert(format!("{provider_key}/{last}"));
            }
        }
    }

    let mut best = 0_i64;
    for provider_row in crate::dashboard_provider_runtime::provider_rows(root, snapshot) {
        let row_provider = clean_text(
            provider_row
                .get("id")
                .or_else(|| provider_row.get("provider"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        if !provider_key.is_empty() && provider_key != "auto" && row_provider != provider_key {
            continue;
        }
        let profiles = provider_row
            .get("model_profiles")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (name, profile) in profiles {
            let profile_model = clean_text(&name, 200).to_ascii_lowercase();
            if profile_model.is_empty() {
                continue;
            }
            let profile_refs = [
                profile_model.clone(),
                if row_provider.is_empty() {
                    profile_model.clone()
                } else {
                    format!("{}/{}", row_provider, profile_model)
                },
            ];
            if !profile_refs
                .iter()
                .any(|candidate| requested_refs.contains(candidate))
            {
                continue;
            }
            let params = parse_i64_loose(profile.get("param_count_billion"))
                .max(parse_i64_loose(profile.get("params_billion")));
            if params > best {
                best = params;
            }
        }
    }
    if best > 0 {
        return best;
    }

    let catalog_rows = crate::dashboard_model_catalog::catalog_payload(root, snapshot)
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in catalog_rows {
        let row_provider = clean_text(
            row.get("provider").and_then(Value::as_str).unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        if !provider_key.is_empty() && provider_key != "auto" && row_provider != provider_key {
            continue;
        }
        let row_model = clean_text(
            row.get("model")
                .or_else(|| row.get("id"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            200,
        )
        .to_ascii_lowercase();
        if row_model.is_empty() || !requested_refs.contains(&row_model) {
            continue;
        }
        let params = parse_i64_loose(row.get("params_billion"))
            .max(parse_i64_loose(row.get("param_count_billion")));
        if params > best {
            best = params;
        }
    }
    best
}

fn selected_model_supports_self_naming(
    root: &Path,
    snapshot: &Value,
    provider_hint: &str,
    model_hint: &str,
) -> bool {
    selected_model_param_count_billion(root, snapshot, provider_hint, model_hint) >= 80
}

fn parse_manifest_fields(manifest_toml: &str) -> HashMap<String, String> {
    let mut out = HashMap::<String, String>::new();
    let mut in_model = false;
    for line in manifest_toml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = trimmed.trim_matches(|ch| ch == '[' || ch == ']');
            in_model = section.eq_ignore_ascii_case("model");
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = clean_text(k, 80).to_ascii_lowercase();
            let mut value = v.trim().trim_matches('"').to_string();
            value = clean_text(&value, 400);
            if value.is_empty() {
                continue;
            }
            if key == "name" {
                out.insert("name".to_string(), value.clone());
            } else if key == "role" {
                out.insert("role".to_string(), value.clone());
            } else if in_model && key == "provider" {
                out.insert("provider".to_string(), value.clone());
            } else if in_model && key == "model" {
                out.insert("model".to_string(), value.clone());
            }
        }
    }
    out
}

fn make_agent_id(root: &Path, suggested_name: &str) -> String {
    let profiles = profiles_map(root);
    let contracts = contracts_map(root);
    let mut used = HashSet::<String>::new();
    for key in profiles.keys() {
        used.insert(clean_agent_id(key));
    }
    for key in contracts.keys() {
        used.insert(clean_agent_id(key));
    }
    let hint = clean_text(suggested_name, 80)
        .to_ascii_lowercase()
        .replace(' ', "-");
    let hint_suffix = if hint == "agent" {
        String::new()
    } else if let Some(rest) = hint
        .strip_prefix("agent-")
        .or_else(|| hint.strip_prefix("agent_"))
    {
        clean_agent_id(rest.trim_matches(|ch| ch == '-' || ch == '_'))
    } else {
        clean_agent_id(&hint)
    };
    let direct = clean_agent_id(&hint);
    if !direct.is_empty() && !used.contains(&direct) {
        return direct;
    }
    let hash_seed = json!({"hint": hint, "ts": crate::now_iso(), "nonce": Utc::now().timestamp_nanos_opt().unwrap_or_default()});
    let hash = crate::deterministic_receipt_hash(&hash_seed);
    let mut base = format!("agent-{}", hash.chars().take(12).collect::<String>());
    if !hint_suffix.is_empty() && hint_suffix.len() <= 18 {
        base = format!(
            "agent-{}-{}",
            hint_suffix,
            hash.chars().take(5).collect::<String>()
        );
    }
    let mut candidate = clean_agent_id(&base);
    if candidate.is_empty() {
        candidate = format!("agent-{}", hash.chars().take(12).collect::<String>());
    }
    if !used.contains(&candidate) {
        return candidate;
    }
    for idx in 2..5000 {
        let next = format!("{candidate}-{idx}");
        if !used.contains(&next) {
            return next;
        }
    }
    format!(
        "agent-{}",
        crate::deterministic_receipt_hash(&json!({"fallback": crate::now_iso()}))
            .chars()
            .take(14)
            .collect::<String>()
    )
}

fn contract_with_runtime_fields(contract: &Value) -> Value {
    let mut out = if contract.is_object() {
        contract.clone()
    } else {
        json!({})
    };
    let status = clean_text(
        out.get("status")
            .and_then(Value::as_str)
            .unwrap_or("active"),
        40,
    );
    let termination_condition = clean_text(
        out.get("termination_condition")
            .and_then(Value::as_str)
            .unwrap_or("task_or_timeout"),
        80,
    )
    .to_ascii_lowercase();
    let auto_terminate_allowed = out
        .get("auto_terminate_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let idle_terminate_allowed = out
        .get("idle_terminate_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let non_expiring = matches!(termination_condition.as_str(), "manual" | "task_complete")
        || (!auto_terminate_allowed && !idle_terminate_allowed);
    if non_expiring {
        if out
            .get("expires_at")
            .and_then(Value::as_str)
            .map(|v| v.trim().is_empty())
            .unwrap_or(true)
        {
            out["expires_at"] = Value::String(String::new());
        }
        out["remaining_ms"] = Value::Null;
        return out;
    }
    let now = Utc::now();
    let created = out
        .get("created_at")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339_utc)
        .unwrap_or(now);
    let expiry_seconds = out
        .get("expiry_seconds")
        .and_then(Value::as_i64)
        .unwrap_or(3600)
        .clamp(1, 31 * 24 * 60 * 60);
    let expires = out
        .get("expires_at")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339_utc)
        .unwrap_or_else(|| created + chrono::Duration::seconds(expiry_seconds));
    if out
        .get("expires_at")
        .and_then(Value::as_str)
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
    {
        out["expires_at"] = Value::String(expires.to_rfc3339());
    }
    let mut remaining = (expires.timestamp_millis() - now.timestamp_millis()).max(0);
    if status.eq_ignore_ascii_case("terminated") {
        remaining = 0;
    }
    out["remaining_ms"] = Value::from(remaining);
    out
}

fn collab_agents_map(snapshot: &Value) -> HashMap<String, Value> {
    let mut out = HashMap::<String, Value>::new();
    let rows = snapshot
        .pointer("/collab/dashboard/agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in rows {
        let id = clean_agent_id(row.get("shadow").and_then(Value::as_str).unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        out.insert(id, row);
    }
    out
}

fn collab_runtime_active(row: Option<&Value>) -> bool {
    row.and_then(|value| value.get("status").and_then(Value::as_str))
        .map(|status| {
            status.eq_ignore_ascii_case("active") || status.eq_ignore_ascii_case("running")
        })
        .unwrap_or(false)
}

fn session_summary_map(root: &Path, snapshot: &Value) -> HashMap<String, Value> {
    let mut out = HashMap::<String, Value>::new();
    let snapshot_rows = snapshot
        .pointer("/agents/session_summaries/rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in snapshot_rows {
        let agent_id = clean_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            continue;
        }
        out.insert(agent_id, row);
    }
    let state_rows = crate::dashboard_agent_state::session_summaries(root, 500)
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in state_rows {
        let agent_id = clean_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            continue;
        }
        out.insert(agent_id, row);
    }
    out
}

fn session_summary_rows(root: &Path, snapshot: &Value) -> Vec<Value> {
    let mut rows = session_summary_map(root, snapshot)
        .into_values()
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn first_string(value: Option<&Value>, key: &str) -> String {
    clean_text(
        value
            .and_then(|row| row.get(key).and_then(Value::as_str))
            .unwrap_or(""),
        240,
    )
}

