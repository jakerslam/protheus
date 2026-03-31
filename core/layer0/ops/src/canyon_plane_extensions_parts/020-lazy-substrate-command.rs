pub(super) fn lazy_substrate_command(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        24,
    )
    .to_ascii_lowercase();
    let path = lazy_substrate_path(root);
    let mut state = read_object(&path);
    let (rules, graph_errors, graph_path) = load_substrate_adapter_rules(root);
    let available_adapters = rules.iter().map(|row| row.id.as_str()).collect::<Vec<_>>();
    let current_feature_set = state
        .get("feature_set")
        .and_then(Value::as_str)
        .unwrap_or("minimal")
        .to_ascii_lowercase();
    if state.is_empty() {
        state.insert("default_features".to_string(), json!([]));
        state.insert("loaded_adapters".to_string(), json!([]));
        state.insert("available_adapters".to_string(), json!(available_adapters));
        state.insert(
            "feature_set".to_string(),
            Value::String("minimal".to_string()),
        );
        state.insert(
            "adapter_graph_path".to_string(),
            Value::String(graph_path.clone()),
        );
    }

    let mut errors = Vec::<String>::new();
    match op.as_str() {
        "enable" => {
            let feature_set = clean(
                parsed
                    .flags
                    .get("feature-set")
                    .map(String::as_str)
                    .unwrap_or("minimal"),
                64,
            )
            .to_ascii_lowercase();
            let default_features = if feature_set == "full-substrate" {
                json!(["full-substrate"])
            } else {
                json!([])
            };
            state.insert("feature_set".to_string(), Value::String(feature_set));
            state.insert("default_features".to_string(), default_features);
            state.insert("updated_at".to_string(), Value::String(now_iso()));
        }
        "load" => {
            let adapter = clean(
                parsed
                    .flags
                    .get("adapter")
                    .map(String::as_str)
                    .unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            if adapter.is_empty() {
                return Err("adapter_required".to_string());
            }
            let known = rules.iter().any(|row| row.id == adapter);
            if strict && !known {
                errors.push("adapter_unknown".to_string());
            }
            if strict {
                let feature_set = state
                    .get("feature_set")
                    .and_then(Value::as_str)
                    .unwrap_or(current_feature_set.as_str())
                    .to_ascii_lowercase();
                let allowed = rules
                    .iter()
                    .find(|row| row.id == adapter)
                    .map(|row| row.feature_sets.iter().any(|set| set == &feature_set))
                    .unwrap_or(false);
                if !allowed {
                    errors.push("adapter_not_enabled_for_feature_set".to_string());
                }
            }
            let mut loaded = state
                .get("loaded_adapters")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if !loaded
                .iter()
                .any(|row| row.as_str() == Some(adapter.as_str()))
            {
                loaded.push(Value::String(adapter.clone()));
            }
            state.insert("loaded_adapters".to_string(), Value::Array(loaded));
            state.insert("updated_at".to_string(), Value::String(now_iso()));
        }
        "status" => {}
        _ => return Err("lazy_substrate_op_invalid".to_string()),
    }

    let loaded_count = state
        .get("loaded_adapters")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let feature_count = state
        .get("default_features")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let graph_valid = graph_errors.is_empty();
    let graph_errors_for_payload = graph_errors.clone();
    if strict && !graph_valid {
        errors.extend(graph_errors);
    }
    if strict {
        let active_feature_set = state
            .get("feature_set")
            .and_then(Value::as_str)
            .unwrap_or("minimal")
            .to_ascii_lowercase();
        let loaded = state
            .get("loaded_adapters")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for row in loaded {
            let Some(adapter_id) = row.as_str() else {
                continue;
            };
            let Some(rule) = rules.iter().find(|rule| rule.id == adapter_id) else {
                errors.push(format!("loaded_adapter_missing_graph_rule:{adapter_id}"));
                continue;
            };
            if !rule
                .feature_sets
                .iter()
                .any(|set| set == &active_feature_set)
            {
                errors.push(format!("loaded_adapter_feature_set_violation:{adapter_id}"));
            }
            if rule.feature_gate.trim().is_empty() {
                errors.push(format!("loaded_adapter_missing_feature_gate:{adapter_id}"));
            }
        }
    }
    let size_saved_bytes = if feature_count == 0 {
        4_194_304u64.saturating_sub((loaded_count as u64) * 262_144)
    } else {
        0
    };

    let state_value = Value::Object(state.clone());
    let payload = json!({
        "ok": !strict || errors.is_empty(),
        "type": "canyon_plane_lazy_substrate",
        "lane": LANE_ID,
        "ts": now_iso(),
        "strict": strict,
        "op": op,
        "state": state_value,
        "adapter_graph": {
            "path": graph_path,
            "rules_loaded": rules.len(),
            "valid": graph_valid,
            "errors": graph_errors_for_payload
        },
        "size_saved_bytes": size_saved_bytes,
        "errors": errors,
        "claim_evidence": [{
            "id": "V7-CANYON-002.2",
            "claim": "substrate_adapters_default_empty_and_load_on_explicit_request",
            "evidence": {
                "loaded_count": loaded_count,
                "feature_count": feature_count,
                "size_saved_bytes": size_saved_bytes
            }
        }]
    });
    write_json(&path, &state_value)?;
    Ok(payload)
}
