fn execute_workflow_visual_bridge_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let eval_pass_rate = payload_f64(payload, "eval_pass_rate", 0.9);
    if strict && profile.id == "V6-WORKFLOW-029.4" && eval_pass_rate < 0.5 {
        return Err("workflow_visual_bridge_eval_gate_failed".to_string());
    }
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "graph_nodes": payload_u64(payload, "graph_nodes", 8),
        "prompt_chain_steps": payload_u64(payload, "prompt_chain_steps", 3),
        "retrieval_latency_ms": payload_f64(payload, "retrieval_latency_ms", 80.0),
        "eval_pass_rate": eval_pass_rate,
        "cold_start_guard_ms": payload_f64(payload, "cold_start_guard_ms", 3000.0),
        "state_path": state_rel
    });
    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "workflow_visual_bridge_lane_maps_canvas_prompt_rag_and_eval_runtime_surfaces_with_receipts",
            "evidence": {
                "eval_pass_rate": eval_pass_rate,
                "retrieval_latency_ms": payload_f64(payload, "retrieval_latency_ms", 80.0)
            }
        })],
        artifacts: vec![state_rel],
    })
}

fn source_path_from_payload(root: &Path, payload: &Value, key: &str, fallback: &str) -> PathBuf {
    let raw = payload_string(payload, key, fallback);
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn copy_file_if_present(
    root: &Path,
    source: &Path,
    destination: &Path,
    apply: bool,
) -> Result<Option<Value>, String> {
    if !source.exists() || !source.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(source).map_err(|err| format!("assimilation_read_failed:{err}"))?;
    if apply {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("assimilation_dir_create_failed:{err}"))?;
        }
        fs::write(destination, &bytes).map_err(|err| format!("assimilation_write_failed:{err}"))?;
    }
    Ok(Some(json!({
        "source": lane_utils::rel_path(root, source),
        "destination": lane_utils::rel_path(root, destination),
        "bytes": bytes.len(),
        "sha256": sha256_hex(&bytes),
    })))
}

fn copy_tree_files_if_present(
    root: &Path,
    source_root: &Path,
    destination_root: &Path,
    apply: bool,
) -> Result<Vec<Value>, String> {
    if !source_root.exists() || !source_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut copied = Vec::<Value>::new();
    let mut stack = vec![source_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let read =
            fs::read_dir(&dir).map_err(|err| format!("assimilation_read_dir_failed:{err}"))?;
        let mut entries = Vec::<PathBuf>::new();
        for entry in read.flatten() {
            entries.push(entry.path());
        }
        entries.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
        for entry_path in entries {
            if entry_path.is_dir() {
                stack.push(entry_path);
                continue;
            }
            if !entry_path.is_file() {
                continue;
            }
            let rel = entry_path
                .strip_prefix(source_root)
                .map_err(|err| format!("assimilation_strip_prefix_failed:{err}"))?;
            let destination = destination_root.join(rel);
            if let Some(row) = copy_file_if_present(root, &entry_path, &destination, apply)? {
                copied.push(row);
            }
        }
    }

    copied.sort_by(|a, b| {
        let a_source = a.get("source").and_then(Value::as_str).unwrap_or_default();
        let b_source = b.get("source").and_then(Value::as_str).unwrap_or_default();
        a_source.cmp(b_source)
    });
    Ok(copied)
}

fn read_json_if_exists(path: &Path) -> Option<Value> {
    if !path.exists() || !path.is_file() {
        return None;
    }
    lane_utils::read_json(path)
}

fn openclaw_seed_to_model_artifacts(seed_manifest: &Value) -> Vec<Value> {
    seed_manifest
        .get("artifacts")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let id = row.get("id").and_then(Value::as_str)?.trim();
                    let provider = row.get("provider").and_then(Value::as_str)?.trim();
                    let model = row.get("model").and_then(Value::as_str)?.trim();
                    if id.is_empty() || provider.is_empty() || model.is_empty() {
                        return None;
                    }
                    let required = row
                        .get("required")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let auto_pull = !row.get("present").and_then(Value::as_bool).unwrap_or(true);
                    Some(json!({
                        "id": id,
                        "provider": provider,
                        "model": model,
                        "required": required,
                        "auto_pull": auto_pull,
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_model_parameter_billions(model_name: &str) -> Option<f32> {
    let lower = model_name.to_lowercase();
    let bytes = lower.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] != b'b' {
            continue;
        }
        if i == 0 {
            continue;
        }
        let mut start = i;
        while start > 0 {
            let c = bytes[start - 1] as char;
            if c.is_ascii_digit() || c == '.' {
                start -= 1;
            } else {
                break;
            }
        }
        if start < i {
            let raw = &lower[start..i];
            if let Ok(value) = raw.parse::<f32>() {
                if value.is_finite() && value > 0.0 {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn openclaw_seed_to_llm_models(seed_manifest: &Value) -> Vec<ModelMetadata> {
    let mut models = Vec::<ModelMetadata>::new();
    let Some(rows) = seed_manifest.get("artifacts").and_then(Value::as_array) else {
        return models;
    };

    for row in rows {
        let id = row.get("id").and_then(Value::as_str).unwrap_or("").trim();
        let provider = row
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .trim();
        let model = row
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if id.is_empty() || model.is_empty() {
            continue;
        }
        let provider_lower = provider.to_lowercase();
        let model_lower = model.to_lowercase();
        let runtime_kind = if provider_lower.contains("ollama")
            || provider_lower.contains("local")
            || provider_lower.contains("lmstudio")
        {
            ModelRuntimeKind::LocalApi
        } else {
            ModelRuntimeKind::CloudApi
        };

        let mut entry = ModelMetadata::new(id, provider, model, runtime_kind);
        entry.parameter_billions = parse_model_parameter_billions(model);
        entry.context_tokens = row
            .get("context_tokens")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .or(Some(
                if matches!(runtime_kind, ModelRuntimeKind::CloudApi) {
                    128_000
                } else {
                    32_768
                },
            ));
        if matches!(runtime_kind, ModelRuntimeKind::CloudApi) {
            entry.pricing_input_per_1m_usd = row
                .get("pricing_input_per_1m_usd")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .or(Some(entry.parameter_billions.unwrap_or(7.0).max(1.0) * 0.2));
            entry.pricing_output_per_1m_usd = row
                .get("pricing_output_per_1m_usd")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .or(Some(
                    entry.parameter_billions.unwrap_or(7.0).max(1.0) * 0.35,
                ));
        } else {
            entry.hardware_vram_gb = row
                .get("hardware_vram_gb")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .or(Some(
                    (entry.parameter_billions.unwrap_or(4.0).max(1.0) * 1.15).round(),
                ));
        }

        let mut specialties = vec![ModelSpecialty::General];
        if model_lower.contains("coder") || model_lower.contains("code") {
            specialties.push(ModelSpecialty::Coding);
        }
        if model_lower.contains("reason") || model_lower.contains("think") {
            specialties.push(ModelSpecialty::Reasoning);
        }
        if entry.context_tokens.unwrap_or(0) >= 64_000 {
            specialties.push(ModelSpecialty::LongContext);
        }
        if model_lower.contains("mini")
            || model_lower.contains("small")
            || model_lower.contains("4b")
            || model_lower.contains("3b")
        {
            specialties.push(ModelSpecialty::FastResponse);
        }
        entry.specialties = specialties;
        models.push(entry);
    }

    models
}

fn llm_model_to_json(model: &ModelMetadata) -> Value {
    let runtime_kind = match model.runtime_kind {
        ModelRuntimeKind::CloudApi => "cloud_api",
        ModelRuntimeKind::LocalApi => "local_api",
        ModelRuntimeKind::LocalPath => "local_path",
    };
    let specialties = model
        .specialties
        .iter()
        .map(|item| match item {
            ModelSpecialty::General => "general",
            ModelSpecialty::Coding => "coding",
            ModelSpecialty::Reasoning => "reasoning",
            ModelSpecialty::LongContext => "long_context",
            ModelSpecialty::FastResponse => "fast_response",
        })
        .map(|v| Value::String(v.to_string()))
        .collect::<Vec<_>>();
    json!({
        "id": model.id,
        "provider": model.provider,
        "name": model.name,
        "runtime_kind": runtime_kind,
        "context_tokens": model.context_tokens,
        "parameter_billions": model.parameter_billions,
        "pricing_input_per_1m_usd": model.pricing_input_per_1m_usd,
        "pricing_output_per_1m_usd": model.pricing_output_per_1m_usd,
        "hardware_vram_gb": model.hardware_vram_gb,
        "specialties": specialties,
        "power_score_1_to_5": model.power_score_1_to_5,
        "cost_score_1_to_5": model.cost_score_1_to_5
    })
}
