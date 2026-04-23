
fn resolve_prompt_suggestion_model(root: &Path, agent_id: &str) -> Option<(String, String, i64)> {
    let profile = read_agent_profile(root, agent_id);
    let model_override = clean_text(
        profile
            .get("model_override")
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    let mut provider = clean_text(
        profile
            .get("model_provider")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    let mut model = clean_text(
        profile
            .get("runtime_model")
            .or_else(|| profile.get("model_name"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    if let Some((override_provider, override_model)) = parse_provider_model_ref(&model_override) {
        if provider.is_empty() {
            provider = override_provider;
        }
        if model.is_empty() || model_id_is_placeholder(&model) {
            model = override_model;
        }
    }
    if model_id_is_placeholder(&model) {
        model.clear();
    }
    if provider.is_empty() {
        provider = "auto".to_string();
    }
    if model.is_empty() {
        model = "auto".to_string();
    }

    let snapshot = read_json_file(
        &root.join("client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json"),
    )
    .unwrap_or_else(|| json!({}));
    let route_request = json!({
        "agent_id": agent_id,
        "task_type": "prompt_suggestions",
        "complexity": "general",
        "budget_mode": "balanced"
    });
    let (resolved_provider, resolved_model, _) =
        crate::dashboard_model_catalog::resolve_model_selection(
            root,
            &snapshot,
            &provider,
            &model,
            &route_request,
        );
    if resolved_provider.is_empty()
        || resolved_model.is_empty()
        || model_id_is_placeholder(&resolved_model)
    {
        return None;
    }

    let catalog = crate::dashboard_model_catalog::catalog_payload(root, &snapshot);
    let mut params_billion = catalog
        .get("models")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                clean_text(
                    row.get("provider").and_then(Value::as_str).unwrap_or(""),
                    80,
                )
                .eq_ignore_ascii_case(&resolved_provider)
                    && clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 240)
                        == resolved_model
            })
        })
        .map(|row| parse_i64_value(row.get("params_billion")))
        .unwrap_or(0);
    if params_billion <= 0 {
        params_billion = parse_param_billion_hint(&resolved_model)
            .max(parse_param_billion_hint(&model_override))
            .max(parse_i64_value(profile.get("param_count_billion")));
    }

    Some((resolved_provider, resolved_model, params_billion.max(0)))
}

fn is_template_like_suggestion(text: &str) -> bool {
    let lowered = clean_text(text, 240).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    lowered.contains("continue with")
        || lowered.contains("what should we")
        || lowered.contains("what should i")
        || lowered.contains("can you continue")
        || lowered.contains("can you verify")
        || lowered.contains("can you test")
        || lowered.contains("does compare other")
}

fn parse_model_suggestion_rows(raw: &str) -> Vec<String> {
    let value = parse_json_loose(raw).unwrap_or_else(|| Value::String(String::new()));
    let rows = if let Some(array) = value.as_array() {
        array.clone()
    } else if let Some(array) = value.get("suggestions").and_then(Value::as_array) {
        array.clone()
    } else if let Some(array) = value.get("rows").and_then(Value::as_array) {
        array.clone()
    } else {
        Vec::new()
    };
    rows.into_iter()
        .filter_map(|row| row.as_str().map(|text| sanitize_suggestion(text)))
        .filter(|row| !row.is_empty())
        .filter(|row| !is_template_like_suggestion(row))
        .collect::<Vec<_>>()
}

fn bool_env(name: &str, fallback: bool) -> bool {
    match std::env::var(name) {
        Ok(raw) => matches!(
            clean_text(&raw, 40).to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => fallback,
    }
}

fn looks_like_shell_command_line(line: &str) -> bool {
    let first = clean_text(line, 200)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        first.as_str(),
        "git"
            | "gh"
            | "cargo"
            | "npm"
            | "npx"
            | "pnpm"
            | "node"
            | "python"
            | "pytest"
            | "ls"
            | "cat"
            | "rg"
            | "grep"
            | "find"
            | "tree"
            | "curl"
            | "wget"
            | "docker"
            | "kubectl"
            | "infring"
            | "infringctl"
            | "infring-ops"
    )
}

fn collect_recent_command_candidates(
    recent_thread: &[(String, String)],
    max_rows: usize,
) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for (role, text) in recent_thread.iter().rev() {
        if role != "user" {
            continue;
        }
        for line in text.lines() {
            let normalized = clean_text(line.trim_start_matches("$ "), 220);
            if normalized.is_empty() || !looks_like_shell_command_line(&normalized) {
                continue;
            }
            if out.iter().any(|existing| existing == &normalized) {
                continue;
            }
            out.push(normalized);
            if out.len() >= max_rows.max(1) {
                return out;
            }
        }
    }
    out
}
