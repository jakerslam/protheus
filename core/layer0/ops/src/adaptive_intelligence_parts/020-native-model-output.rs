fn native_model_output(
    role: &str,
    model: &str,
    prompt: &str,
    context: &ContextBundle,
    resource_mode: &str,
) -> String {
    let mut keywords = BTreeSet::<String>::new();
    for row in context
        .conversation_samples
        .iter()
        .chain(context.dream_samples.iter())
    {
        for token in row
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .filter(|token| token.len() >= 4)
        {
            if keywords.len() >= 6 {
                break;
            }
            keywords.insert(token.to_ascii_lowercase());
        }
        if keywords.len() >= 6 {
            break;
        }
    }
    let keyword_list = if keywords.is_empty() {
        vec!["operator".to_string(), "context".to_string()]
    } else {
        keywords.into_iter().collect::<Vec<_>>()
    };
    let digest = sha256_hex_str(&format!(
        "{}|{}|{}|{}|{}",
        role, model, prompt, context.interaction_digest, resource_mode
    ));
    if role == "creative" {
        format!(
            "creative-hypothesis:{}\ncreative-angle:{}\ncreative-bridge:{}",
            keyword_list
                .first()
                .cloned()
                .unwrap_or_else(|| "novelty".to_string()),
            keyword_list
                .get(1)
                .cloned()
                .unwrap_or_else(|| "scenario".to_string()),
            &digest[..12]
        )
    } else {
        format!(
            "logical-step:{}\nlogical-check:{}\nlogical-constraint:{}",
            keyword_list
                .first()
                .cloned()
                .unwrap_or_else(|| "plan".to_string()),
            keyword_list
                .get(1)
                .cloned()
                .unwrap_or_else(|| "verify".to_string()),
            &digest[..12]
        )
    }
}

fn native_fallback_provider_output(
    role: &str,
    model: &str,
    prompt: &str,
    context: &ContextBundle,
    resource_mode: &str,
) -> Value {
    json!({
        "provider": "native-fallback",
        "model": model,
        "role": role,
        "output": native_model_output(role, model, prompt, context, resource_mode)
    })
}

fn run_local_model(
    _policy: &AdaptivePolicy,
    role: &str,
    model: &str,
    prompt: &str,
    context: &ContextBundle,
    resource_mode: &str,
) -> Value {
    let bin = std::env::var(LOCAL_AI_BIN_ENV).unwrap_or_else(|_| "ollama".to_string());
    if is_local_model(model) && command_exists(&bin) {
        let mut command = Command::new(&bin);
        command.arg("run").arg(ollama_model_name(model)).arg(prompt);
        match command.output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                json!({
                    "provider": "ollama",
                    "model": model,
                    "role": role,
                    "output": if stdout.is_empty() {
                        native_model_output(role, model, prompt, context, resource_mode)
                    } else {
                        stdout
                    }
                })
            }
            _ => native_fallback_provider_output(role, model, prompt, context, resource_mode),
        }
    } else {
        native_fallback_provider_output(role, model, prompt, context, resource_mode)
    }
}

fn extract_candidates(text: &str, prefix: &str) -> Vec<String> {
    let mut out = text
        .lines()
        .map(|line| clean(line, 160))
        .filter(|line| !line.is_empty())
        .take(4)
        .collect::<Vec<_>>();
    if out.is_empty() {
        out.push(format!("{prefix}:{}", clean(text, 120)));
    }
    out
}

fn connector_synthesis(
    prompt: &str,
    logical: &Value,
    creative: Option<&Value>,
    resources: &ResourceSnapshot,
    context: &ContextBundle,
) -> Value {
    let logical_text = logical
        .get("output")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let logical_candidates = extract_candidates(logical_text, "logical");
    let creative_candidates = creative
        .and_then(|row| row.get("output").and_then(Value::as_str))
        .map(|text| extract_candidates(text, "creative"))
        .unwrap_or_default();
    let proposal_count = logical_candidates.len().min(3).max(1);
    let mut proposals = Vec::<Value>::new();
    for idx in 0..proposal_count {
        let logical_line = logical_candidates
            .get(idx)
            .cloned()
            .or_else(|| logical_candidates.first().cloned())
            .unwrap_or_else(|| "logical-step".to_string());
        let creative_line = creative_candidates
            .get(idx)
            .cloned()
            .or_else(|| creative_candidates.first().cloned())
            .unwrap_or_else(|| "".to_string());
        let confidence = if resources.mode == "dual" {
            0.66 + (idx as f64 * 0.05)
        } else if resources.mode == "logical_only" {
            0.61 + (idx as f64 * 0.04)
        } else {
            0.56 + (idx as f64 * 0.03)
        };
        let action = if creative_line.is_empty() {
            logical_line.clone()
        } else {
            format!("{logical_line} | creative_extension:{creative_line}")
        };
        proposals.push(json!({
            "rank": idx + 1,
            "action": action,
            "confidence": (confidence * 100.0).round() / 100.0,
            "prompt_digest": sha256_hex_str(&format!("{}|{}", prompt, idx)),
            "context_digest": context.interaction_digest
        }));
    }
    let connector_digest = sha256_hex_str(&serde_json::to_string(&proposals).unwrap_or_default());
    json!({
        "referee": "deterministic_connector_v1",
        "proposal_count": proposals.len(),
        "connector_digest": connector_digest,
        "proposals": proposals,
        "raw_sources": {
            "logical": logical,
            "creative": creative.cloned().unwrap_or(Value::Null)
        }
    })
}

fn specialization_gain(
    role: &str,
    context: &ContextBundle,
    resources: &ResourceSnapshot,
    cycle: u64,
) -> f64 {
    let conversation_weight = context.conversation_samples.len() as f64;
    let dream_weight = context.dream_samples.len() as f64;
    let resource_bonus = if resources.mode == "dual" { 1.0 } else { 0.5 };
    let cycle_bonus = (cycle as f64).min(6.0) * 0.35;
    let raw = if role == "creative" {
        (dream_weight * 1.4) + (conversation_weight * 0.45) + resource_bonus + cycle_bonus
    } else {
        (conversation_weight * 1.1) + (dream_weight * 0.35) + resource_bonus + cycle_bonus
    };
    raw.min(9.0)
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(
        root,
        STATE_ENV,
        STATE_SCOPE,
        "adaptive_intelligence_error",
        payload,
    )
}

fn status(root: &Path, policy: &AdaptivePolicy, state: &RuntimeState) -> Value {
    let latest = read_json(&latest_path(root));
    json!({
        "ok": true,
        "type": "adaptive_intelligence_status",
        "lane": "core/layer0/ops",
        "policy": policy,
        "state": state,
        "latest_path": latest_path(root).display().to_string(),
        "latest": latest
    })
}

fn conduit(root: &Path, parsed: &crate::ParsedArgs, action: &str, strict: bool) -> Value {
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "adaptive_intelligence_conduit_enforcement",
        COMMAND_PATH,
        conduit_bypass_requested(&parsed.flags),
        "adaptive_intelligence_actions_route_through_layer0_conduit_with_fail_closed_policy",
        &[
            "V7-ADAPTIVE-001.1",
            "V7-ADAPTIVE-001.2",
            "V7-ADAPTIVE-001.3",
            "V7-ADAPTIVE-001.4",
            "V7-ADAPTIVE-001.5",
            "V7-ADAPTIVE-001.6",
        ],
    )
}

fn run_prioritize(
    root: &Path,
    policy: &AdaptivePolicy,
    parsed: &crate::ParsedArgs,
    strict: bool,
) -> Value {
    let conduit = conduit(root, parsed, "prioritize", strict);
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return json!({
            "ok": false,
            "type": "adaptive_intelligence_prioritize",
            "strict": strict,
            "conduit": conduit,
            "errors": ["conduit_bypass_rejected"]
        });
    }
    let resources = resource_snapshot(&parsed.flags, policy);
    json!({
        "ok": true,
        "type": "adaptive_intelligence_prioritize",
        "strict": strict,
        "resources": resources,
        "claim_evidence": [{
            "id": "V7-ADAPTIVE-001.4",
            "claim": "resource_aware_prioritization_runs_dual_or_logical_first_and_logs_degradation",
            "evidence": {
                "mode": resources.mode,
                "degraded": resources.degraded,
                "vram_gb": resources.vram_gb,
                "ram_gb": resources.ram_gb,
                "cpu_cores": resources.cpu_cores
            }
        }],
        "conduit": conduit
    })
}

fn run_propose(
    root: &Path,
    policy: &AdaptivePolicy,
    parsed: &crate::ParsedArgs,
    strict: bool,
) -> Value {
    let conduit = conduit(root, parsed, "propose", strict);
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return json!({
            "ok": false,
            "type": "adaptive_intelligence_propose",
            "strict": strict,
            "conduit": conduit,
            "errors": ["conduit_bypass_rejected"]
        });
    }
    let prompt = clean(
        parsed
            .flags
            .get("prompt")
            .cloned()
            .unwrap_or_else(|| "summarize operator intent".to_string()),
        600,
    );
    let context = collect_context_bundle(root, &parsed.flags);
    let resources = resource_snapshot(&parsed.flags, policy);
    let mut errors = Vec::<String>::new();
    if strict && !is_local_model(&policy.logical_model) {
        errors.push("logical_model_must_be_local".to_string());
    }
    if strict && !is_local_model(&policy.creative_model) {
        errors.push("creative_model_must_be_local".to_string());
    }
    let logical_prompt = format!(
        "persona={} bias={} prompt={} context_digest={} conversation_samples={} dream_samples={}",
        context.persona,
        context.logical_bias,
        prompt,
        context.interaction_digest,
        context.conversation_samples.join(" || "),
        context.dream_samples.join(" || ")
    );
    let creative_prompt = format!(
        "persona={} bias={} prompt={} context_digest={} dream_samples={} conversation_samples={}",
        context.persona,
        context.creative_bias,
        prompt,
        context.interaction_digest,
        context.dream_samples.join(" || "),
        context.conversation_samples.join(" || ")
    );
    let logical = run_local_model(
        policy,
        "logical",
        if resources.mode == "tiny_logical_only" {
            &policy.tiny_logical_model
        } else {
            &policy.logical_model
        },
        &logical_prompt,
        &context,
        &resources.mode,
    );
    let creative = if resources.mode == "dual" {
        Some(run_local_model(
            policy,
            "creative",
            &policy.creative_model,
            &creative_prompt,
            &context,
            &resources.mode,
        ))
    } else {
        None
    };
    let connector = connector_synthesis(&prompt, &logical, creative.as_ref(), &resources, &context);
    let connector_digest = connector
        .get("connector_digest")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let proposal = json!({
        "prompt": prompt,
        "mode": resources.mode,
        "resources": resources,
        "context": {
            "persona": context.persona,
            "logical_bias": context.logical_bias,
            "creative_bias": context.creative_bias,
            "conversation_count": context.conversation_samples.len(),
            "dream_count": context.dream_samples.len(),
            "interaction_digest": context.interaction_digest
        },
        "logical": logical,
        "creative": creative.clone().unwrap_or(Value::Null),
        "connector": connector
    })
    .with_receipt_hash();
    let _ = append_jsonl(&proposal_history_path(root), &proposal);
    let _ = append_jsonl(
        &connector_history_path(root),
        &json!({
            "ts": now_iso(),
            "type": "adaptive_intelligence_connector_row",
            "prompt_digest": sha256_hex_str(&prompt),
            "connector_digest": connector_digest,
            "proposal_receipt_hash": proposal.get("receipt_hash").cloned().unwrap_or(Value::Null)
        }),
    );
    let mut state = load_state(root, policy);
    state.updated_at = now_iso();
    state.active_mode = resources.mode.clone();
    state.local_only = policy.local_only;
    state.last_proposal_digest = proposal
        .get("receipt_hash")
        .and_then(Value::as_str)
        .map(|v| v.to_string());
    state.last_connector_digest = Some(connector_digest.clone());
    state.last_resource_mode = Some(resources.mode.clone());
    let _ = store_state(root, &state);
    json!({
        "ok": errors.is_empty(),
        "type": "adaptive_intelligence_propose",
        "strict": strict,
        "mode": resources.mode,
        "proposal": proposal,
        "claim_evidence": [
            {
                "id": "V7-ADAPTIVE-001.1",
                "claim": "dual_local_models_share_a_seed_and_run_as_parallel_creative_and_logical_profiles",
                "evidence": {
                    "seed_model": policy.seed_model,
                    "logical_model": policy.logical_model,
                    "creative_model": policy.creative_model,
                    "mode": resources.mode
                }
            },
            {
                "id": "V7-ADAPTIVE-001.3",
                "claim": "deterministic_connector_merges_dual_model_outputs_into_ranked_proposals",
                "evidence": {
                    "connector_digest": connector_digest,
                    "proposal_count": proposal.get("connector").and_then(|v| v.get("proposal_count")).cloned().unwrap_or(Value::Null)
                }
            },
            {
                "id": "V7-ADAPTIVE-001.4",
                "claim": "resource_aware_prioritization_degrades_to_logical_first_under_constraint",
                "evidence": {
                    "mode": resources.mode,
                    "degraded": resources.degraded
                }
            },
            {
                "id": "V7-ADAPTIVE-001.5",
                "claim": "dream_and_conversation_context_plus_persona_bias_feed_the_adaptive_intelligence_lane",
                "evidence": {
                    "conversation_count": context.conversation_samples.len(),
                    "dream_count": context.dream_samples.len(),
                    "persona": context.persona
                }
            }
        ],
        "errors": errors,
        "conduit": conduit
    })
}
