
fn dashboard_contract_guard_from_payload(payload: &Value) -> Value {
    let input_text = payload_string(payload, "input_text", "");
    let lowered = input_text.to_ascii_lowercase();
    let recent_messages = payload_u64(payload, "recent_messages", 0).min(2_000_000);
    let max_per_min =
        payload_u64(payload, "rogue_message_rate_max_per_min", 20).clamp(1, 1_000_000);

    let contains_any = |terms: &[&str]| -> bool { terms.iter().any(|term| lowered.contains(term)) };

    let mut reason = String::new();
    let mut detail = String::new();
    if contains_any(&["ignore", "bypass", "disable", "override"])
        && contains_any(&["contract", "safety", "receipt", "policy"])
    {
        reason = "contract_override_attempt".to_string();
        detail = "input_requested_contract_bypass".to_string();
    } else if contains_any(&["exfiltrate", "steal", "dump secrets", "leak", "data exfil"]) {
        reason = "data_exfiltration_attempt".to_string();
        detail = "input_requested_exfiltration".to_string();
    } else if contains_any(&["extend", "increase"])
        && contains_any(&["expiry", "ttl", "time to live", "contract"])
    {
        reason = "self_extension_attempt".to_string();
        detail = "input_requested_expiry_extension".to_string();
    } else if recent_messages > max_per_min {
        reason = "message_rate_spike".to_string();
        detail = format!("recent_messages={recent_messages}");
    }

    json!({
        "authority": "rust_runtime_systems",
        "policy": "V6-DASHBOARD-007.3",
        "violation": !reason.is_empty(),
        "reason": reason,
        "detail": detail,
        "recent_messages": recent_messages,
        "rogue_message_rate_max_per_min": max_per_min,
        "input_sha256": sha256_hex(input_text.as_bytes())
    })
}

fn dashboard_auto_route_from_payload(payload: &Value) -> Value {
    let input_text = payload_string(payload, "input_text", "");
    let lowered = input_text.to_ascii_lowercase();
    let token_count = payload_u64(payload, "token_count", (input_text.len() as u64 / 4).max(1))
        .clamp(1, 8_000_000);
    let has_vision = payload_bool(payload, "has_vision", false);
    let asks_speed = payload_bool(
        payload,
        "asks_speed",
        lowered.contains("fast") || lowered.contains("speed"),
    );
    let asks_cost = payload_bool(
        payload,
        "asks_cost",
        lowered.contains("cheap") || lowered.contains("cost"),
    );
    let asks_quality = payload_bool(
        payload,
        "asks_quality",
        lowered.contains("quality") || lowered.contains("best"),
    );
    let asks_long_context = payload_bool(
        payload,
        "asks_long_context",
        token_count >= 100_000 || lowered.contains("long context"),
    );

    let preferred_provider = payload_string(payload, "preferred_provider", "ollama");
    let preferred_model = payload_string(payload, "preferred_model", "llama3.2:3b");
    let fallback_provider = payload_string(payload, "fallback_provider", "cloud");
    let fallback_model = payload_string(payload, "fallback_model", "kimi2.5:cloud");

    let mut raw_candidates = payload_array(payload, "candidates");
    if raw_candidates.is_empty() {
        raw_candidates.push(json!({
            "runtime_provider": preferred_provider,
            "runtime_model": preferred_model
        }));
        raw_candidates.push(json!({
            "runtime_provider": fallback_provider,
            "runtime_model": fallback_model
        }));
    }

    let runtime_success = payload_f64(payload, "spine_success_rate", 0.90).clamp(0.0, 1.0);
    let mut scored = Vec::<Value>::new();
    for candidate in raw_candidates {
        let provider = payload_string(
            &candidate,
            "runtime_provider",
            payload_string(&candidate, "provider", "ollama").as_str(),
        );
        let model = payload_string(
            &candidate,
            "runtime_model",
            payload_string(&candidate, "model", "llama3.2:3b").as_str(),
        );
        let model_lower = model.to_ascii_lowercase();
        let (prior_latency, prior_cost, prior_success): (f64, f64, f64) =
            match provider.to_ascii_lowercase().as_str() {
                "ollama" => (120.0_f64, 0.0_f64, 0.92_f64),
                "groq" => (65.0_f64, 0.2_f64, 0.90_f64),
                "openai" => (90.0_f64, 0.55_f64, 0.95_f64),
                "anthropic" => (105.0_f64, 0.7_f64, 0.95_f64),
                "google" => (95.0_f64, 0.6_f64, 0.94_f64),
                "cloud" => (80.0_f64, 0.3_f64, 0.93_f64),
                _ => (110.0_f64, 0.45_f64, 0.90_f64),
            };
        let model_is_small = model_lower.contains("3b")
            || model_lower.contains("mini")
            || model_lower.contains("small");
        let latency_ms =
            (prior_latency * if model_is_small { 0.85_f64 } else { 1.0_f64 }).max(1.0_f64);
        let cost_per_1k =
            (prior_cost * if model_is_small { 0.7_f64 } else { 1.0_f64 }).max(0.0_f64);
        let context_window = candidate
            .get("context_window")
            .and_then(Value::as_u64)
            .unwrap_or(8192)
            .clamp(1024, 8_000_000);
        let context_score = if token_count <= context_window {
            1.0
        } else {
            (context_window as f64 / token_count as f64).clamp(0.1, 1.0)
        };
        let latency_score = 1.0 / (1.0 + (latency_ms / 120.0));
        let cost_score = 1.0 / (1.0 + cost_per_1k);
        let success_rate = ((prior_success * 0.65) + (runtime_success * 0.35)).clamp(0.2, 0.99);
        let supports_vision = candidate
            .get("supports_vision")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let vision_penalty = if has_vision && !supports_vision {
            0.55
        } else {
            0.0
        };
        let speed_weight = if asks_speed { 1.55 } else { 1.05 };
        let cost_weight = if asks_cost { 1.35 } else { 0.75 };
        let quality_weight = if asks_quality { 1.8 } else { 1.3 };
        let context_weight = if asks_long_context { 1.45 } else { 1.1 };
        let score = (latency_score * speed_weight)
            + (cost_score * cost_weight)
            + (success_rate * quality_weight)
            + (context_score * context_weight)
            - vision_penalty;
        scored.push(json!({
            "provider": provider,
            "model": model,
            "score": (score * 1_000_000.0).round() / 1_000_000.0,
            "latency_ms": latency_ms.round() as u64,
            "cost_per_1k": ((cost_per_1k * 10_000.0).round()) / 10_000.0,
            "success_rate": (success_rate * 10_000.0).round() / 10_000.0,
            "context_window": context_window,
            "supports_vision": supports_vision
        }));
    }

    scored.sort_by(|a, b| {
        let lhs = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let rhs = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        lhs.partial_cmp(&rhs).unwrap_or(std::cmp::Ordering::Equal)
    });

    let selected = scored.first().cloned().unwrap_or_else(|| {
        json!({
            "provider": preferred_provider,
            "model": preferred_model,
            "score": 0.0,
            "latency_ms": 120,
            "cost_per_1k": 0.0,
            "success_rate": 0.9,
            "context_window": 8192,
            "supports_vision": false
        })
    });
    let selected_provider = payload_string(&selected, "provider", "ollama");
    let selected_model = payload_string(&selected, "model", "llama3.2:3b");
    let selected_context_window = selected
        .get("context_window")
        .and_then(Value::as_u64)
        .unwrap_or(8192);
    let reason = format!(
        "rust auto-route selected {} / {} by weighted latency-cost-success-context scoring",
        selected_provider, selected_model
    );
    let fallback_chain = scored
        .iter()
        .skip(1)
        .take(3)
        .map(|row| {
            json!({
                "provider": payload_string(row, "provider", ""),
                "model": payload_string(row, "model", ""),
                "score": row.get("score").and_then(Value::as_f64).unwrap_or(0.0)
            })
        })
        .collect::<Vec<_>>();

    let mut decision = json!({
        "authority": "rust_runtime_systems",
        "policy": "V6-DASHBOARD-008.1",
        "route_lane": "runtime-systems.run",
        "selected_provider": selected_provider,
        "selected_model": selected_model,
        "selected_model_id": format!(
            "{}/{}",
            payload_string(&selected, "provider", "ollama"),
            payload_string(&selected, "model", "llama3.2:3b")
        ),
        "selected_context_window": selected_context_window,
        "reason": reason,
        "context": {
            "token_count": token_count,
            "has_vision": has_vision,
            "asks_speed": asks_speed,
            "asks_cost": asks_cost,
            "asks_quality": asks_quality,
            "asks_long_context": asks_long_context
        },
        "fallback_chain": fallback_chain,
        "candidates": scored,
        "runtime_sync": {
            "spine_success_rate": runtime_success,
            "receipt_latency_p99_ms": payload_f64(payload, "receipt_latency_p99_ms", 0.0).max(0.0)
        }
    });
    let hash = receipt_hash(&decision);
    decision["receipt_hash"] = Value::String(hash);
    decision
}

fn contract_specific_gates(
    profile: RuntimeSystemContractProfile,
    payload: &Value,
) -> (serde_json::Map<String, Value>, Vec<String>) {
    let mut checks = serde_json::Map::new();
    let mut violations = Vec::<String>::new();

    match profile.id {
        "V9-AUDIT-026.1" => {
            let targets = payload_string_array(
                payload,
                "audit_targets",
                &[
                    "origin_integrity",
                    "supply_chain_provenance_v2",
                    "alpha_readiness",
                ],
            );
            let missing = missing_required_tokens(
                &targets,
                &[
                    "origin_integrity",
                    "supply_chain_provenance_v2",
                    "alpha_readiness",
                ],
            );
            checks.insert("audit_targets".to_string(), json!(targets));
            checks.insert("audit_targets_missing".to_string(), json!(missing));
            if !missing.is_empty() {
                violations.push(format!(
                    "specific_missing_audit_targets:{}",
                    missing.join("|")
                ));
            }
        }
        "V9-AUDIT-026.2" => {
            let actions = payload_string_array(
                payload,
                "self_healing_actions",
                &[
                    "refresh_spine_receipt",
                    "rebuild_supply_chain_bundle",
                    "reconcile_workspace_churn",
                ],
            );
            let missing = missing_required_tokens(
                &actions,
                &[
                    "refresh_spine_receipt",
                    "rebuild_supply_chain_bundle",
                    "reconcile_workspace_churn",
                ],
            );
            checks.insert("self_healing_actions".to_string(), json!(actions));
            checks.insert("self_healing_actions_missing".to_string(), json!(missing));
            if !missing.is_empty() {
                violations.push(format!(
                    "specific_missing_self_healing_actions:{}",
                    missing.join("|")
                ));
            }
        }
        "V9-AUDIT-026.3" => {
            let range = payload_string(payload, "confidence_range", "0.0-1.0");
            checks.insert("confidence_range".to_string(), json!(range.clone()));
            if range != "0.0-1.0" {
                violations.push(format!("specific_confidence_range_mismatch:{range}"));
            }
        }
        "V9-AUDIT-026.4" => {
            let consensus = payload_string(payload, "consensus_mode", "strict_match");
            checks.insert("consensus_mode".to_string(), json!(consensus.clone()));
            if consensus != "strict_match" {
                violations.push(format!("specific_consensus_mode_mismatch:{consensus}"));
            }
        }
        "V6-DASHBOARD-007.3" => {
            checks.insert(
                "dashboard_contract_guard".to_string(),
                dashboard_contract_guard_from_payload(payload),
            );
        }
        _ if profile.id.starts_with("V6-DASHBOARD-007.") => {
            checks.insert(
                "dashboard_runtime_authority".to_string(),
                dashboard_runtime_authority_from_payload(payload),
            );
        }
        _ if profile.id.starts_with("V6-DASHBOARD-008.") => {
            checks.insert(
                "dashboard_auto_route_authority".to_string(),
                dashboard_auto_route_from_payload(payload),
            );
        }
        _ => {}
    }

    (checks, violations)
}

fn count_lines(path: &Path) -> u64 {
    fs::read_to_string(path)
        .ok()
        .map(|raw| raw.lines().count() as u64)
        .unwrap_or(0)
}

fn collect_repo_language_lines(dir: &Path, rs_lines: &mut u64, ts_lines: &mut u64) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if path.is_dir() {
            if matches!(
                name,
                ".git"
                    | "target"
                    | "node_modules"
                    | "dist"
                    | "build"
                    | "coverage"
                    | "tmp"
                    | "local"
            ) {
                continue;
            }
            collect_repo_language_lines(&path, rs_lines, ts_lines);
            continue;
        }
        if name.ends_with(".rs") {
            *rs_lines += count_lines(&path);
        } else if name.ends_with(".ts") {
            *ts_lines += count_lines(&path);
        }
    }
}

fn repo_language_share(root: &Path) -> (u64, u64, f64) {
    let mut rs_lines = 0u64;
    let mut ts_lines = 0u64;
    collect_repo_language_lines(root, &mut rs_lines, &mut ts_lines);
    let total = rs_lines.saturating_add(ts_lines);
    let rust_share_pct = if total == 0 {
        0.0
    } else {
        (rs_lines as f64) * 100.0 / (total as f64)
    };
    (rs_lines, ts_lines, rust_share_pct)
}

#[derive(Debug, Clone)]
struct ContractExecution {
    summary: Value,
    claims: Vec<Value>,
    artifacts: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct FamilyContractRequirements {
    required_true: &'static [&'static str],
    min_values: &'static [(&'static str, f64)],
    max_values: &'static [(&'static str, f64)],
}

const EMPTY_REQUIRED_TRUE: &[&str] = &[];
const EMPTY_NUM_GATES: &[(&str, f64)] = &[];
