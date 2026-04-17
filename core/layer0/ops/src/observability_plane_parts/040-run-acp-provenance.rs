fn web_tooling_provider_contract_targets() -> [&'static str; 10] {
    [
        "brave",
        "duckduckgo",
        "exa",
        "firecrawl",
        "google",
        "minimax",
        "moonshot",
        "perplexity",
        "tavily",
        "xai",
    ]
}

fn normalize_web_tooling_provider_target(raw: Option<&String>) -> Option<String> {
    let normalized = raw
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if normalized.is_empty() {
        return None;
    }
    let canonical = match normalized.as_str() {
        "kimi" | "moonshot" => "moonshot",
        "grok" | "xai" => "xai",
        "duck_duck_go" | "duckduckgo" => "duckduckgo",
        "brave_search" | "brave" => "brave",
        _ => normalized.as_str(),
    };
    Some(clean(canonical.to_string(), 48))
}

fn run_acp_provenance(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        ACP_PROVENANCE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "observability_acp_provenance_contract",
            "allowed_ops": ["enable", "status", "trace", "debug"],
            "allowed_visibility_modes": ["off", "meta", "meta+receipt"],
            "require_source_identity": true,
            "require_intent": true
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allowed_ops = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|row| row.to_string())
        .collect::<Vec<_>>();
    if strict && !allowed_ops.iter().any(|row| row == &op) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "observability_plane_acp_provenance",
            "errors": ["observability_acp_provenance_op_invalid"]
        });
    }

    let config_path = provenance_config_path(root);
    let mut config = read_json(&config_path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "enabled": false,
            "visibility_mode": "meta+receipt",
            "updated_at": crate::now_iso()
        })
    });
    if !config.is_object() {
        config = json!({
            "version": "v1",
            "enabled": false,
            "visibility_mode": "meta+receipt",
            "updated_at": crate::now_iso()
        });
    }

    if op == "status" {
        let latest = read_json(&provenance_latest_path(root));
        let trace_history_rows = std::fs::read_to_string(provenance_history_path(root))
            .ok()
            .map(|raw| raw.lines().count())
            .unwrap_or(0);
        let web_tooling_trace_rows = std::fs::read_to_string(provenance_history_path(root))
            .ok()
            .map(|raw| {
                raw.lines()
                    .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                    .filter(|row| {
                        row.get("web_tooling")
                            .and_then(|v| v.get("detected"))
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0);
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "observability_plane_acp_provenance",
            "lane": "core/layer0/ops",
            "op": "status",
            "config": config,
            "latest_trace": latest,
            "trace_history_rows": trace_history_rows,
            "web_tooling_provider_contract_targets": web_tooling_provider_contract_targets(),
            "web_tooling_trace_rows": web_tooling_trace_rows,
            "claim_evidence": [
                {
                    "id": "V6-OBSERVABILITY-005.11",
                    "claim": "acp_provenance_status_surface_reports_end_to_end_activation_and_trace_health",
                    "evidence": {
                        "history_rows": trace_history_rows,
                        "web_tooling_trace_rows": web_tooling_trace_rows,
                        "web_tooling_provider_contract_targets": web_tooling_provider_contract_targets()
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if op == "enable" {
        let enabled = parse_bool(parsed.flags.get("enabled"), true);
        let visibility_mode = parse_visibility_mode(parsed.flags.get("visibility-mode").cloned());
        let allowed_modes_values = contract
            .get("allowed_visibility_modes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let allowed_modes = allowed_modes_values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if strict && !allowed_modes.iter().any(|row| row == &visibility_mode) {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "observability_plane_acp_provenance",
                "errors": ["observability_acp_provenance_visibility_mode_invalid"]
            });
        }
        config["enabled"] = Value::Bool(enabled);
        config["visibility_mode"] = Value::String(visibility_mode.clone());
        config["updated_at"] = Value::String(crate::now_iso());
        let _ = write_json(&config_path, &config);
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "observability_plane_acp_provenance",
            "lane": "core/layer0/ops",
            "op": "enable",
            "config": config,
            "artifact": {
                "config_path": config_path.display().to_string(),
                "config_sha256": sha256_hex_str(&read_json(&config_path).unwrap_or(Value::Null).to_string())
            },
            "claim_evidence": [
                {
                    "id": "V6-OBSERVABILITY-005.11",
                    "claim": "one_command_activation_enables_acp_provenance_with_deterministic_receipts",
                    "evidence": {
                        "enabled": enabled,
                        "visibility_mode": visibility_mode
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if strict
        && !config
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "observability_plane_acp_provenance",
            "op": op,
            "errors": ["observability_acp_provenance_not_enabled"]
        });
    }

    if op == "debug" {
        let trace_id = clean(
            parsed.flags.get("trace-id").cloned().unwrap_or_default(),
            120,
        );
        let rows = std::fs::read_to_string(provenance_history_path(root))
            .ok()
            .map(|raw| {
                raw.lines()
                    .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                    .filter(|row| {
                        if trace_id.is_empty() {
                            true
                        } else {
                            row.get("trace_id").and_then(Value::as_str) == Some(trace_id.as_str())
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "observability_plane_acp_provenance",
            "lane": "core/layer0/ops",
            "op": "debug",
            "trace_id": trace_id,
            "rows": rows,
            "claim_evidence": [
                {
                    "id": "V6-OBSERVABILITY-005.10",
                    "claim": "debug_surface_exposes_trace_chain_for_command_center_diagnostics",
                    "evidence": { "rows": rows.len() }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    let source_agent = clean(
        parsed
            .flags
            .get("source-agent")
            .cloned()
            .or_else(|| parsed.flags.get("source").cloned())
            .unwrap_or_default(),
        120,
    );
    let target_agent = clean(
        parsed
            .flags
            .get("target-agent")
            .cloned()
            .or_else(|| parsed.flags.get("target").cloned())
            .unwrap_or_else(|| "broadcast".to_string()),
        120,
    );
    let intent = clean(parsed.flags.get("intent").cloned().unwrap_or_default(), 180);
    let message = clean(
        parsed
            .flags
            .get("message")
            .cloned()
            .unwrap_or_else(|| "trace payload".to_string()),
        300,
    );
    let visibility_mode =
        parse_visibility_mode(parsed.flags.get("visibility-mode").cloned().or_else(|| {
            config
                .get("visibility_mode")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        }));
    let web_tooling_provider = normalize_web_tooling_provider_target(
        parsed
            .flags
            .get("web-provider")
            .or_else(|| parsed.flags.get("provider")),
    );
    let web_tooling_query = parsed.flags.get("query").map(|raw| clean(raw.clone(), 280));
    let web_tooling_detected = web_tooling_provider.is_some()
        || web_tooling_query
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        || intent.to_ascii_lowercase().contains("web")
        || message.to_ascii_lowercase().contains("web");
    let require_source = contract
        .get("require_source_identity")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let require_intent = contract
        .get("require_intent")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if strict
        && ((require_source && source_agent.is_empty()) || (require_intent && intent.is_empty()))
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "observability_plane_acp_provenance",
            "op": "trace",
            "errors": ["observability_acp_unprovenanced_message_denied"],
            "denial_reason": {
                "missing_source_agent": source_agent.is_empty(),
                "missing_intent": intent.is_empty()
            },
            "claim_evidence": [
                {
                    "id": "V6-OBSERVABILITY-005.10",
                    "claim": "anonymous_or_unprovenanced_messages_are_denied_fail_closed",
                    "evidence": {
                        "missing_source_agent": source_agent.is_empty(),
                        "missing_intent": intent.is_empty()
                    }
                }
            ]
        });
    }

    let history_rows = std::fs::read_to_string(provenance_history_path(root))
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let previous_hop_hash = history_rows
        .last()
        .and_then(|row| row.get("hop_hash"))
        .and_then(Value::as_str)
        .unwrap_or("genesis")
        .to_string();
    let trace_id = clean(
        parsed.flags.get("trace-id").cloned().unwrap_or_else(|| {
            format!(
                "trace_{}",
                &sha256_hex_str(&format!("{source_agent}:{target_agent}:{intent}:{message}"))[..12]
            )
        }),
        128,
    );
    let hop_index = history_rows
        .iter()
        .filter(|row| row.get("trace_id").and_then(Value::as_str) == Some(trace_id.as_str()))
        .count()
        + 1;
    let hop_meta = json!({
        "source_identity": source_agent,
        "target_identity": target_agent,
        "intent": intent,
        "timestamp": crate::now_iso()
    });
    let hop_hash = crate::v8_kernel::next_chain_hash(Some(&previous_hop_hash), &hop_meta);
    let mut trace_entry = json!({
        "version": "v1",
        "trace_id": trace_id,
        "hop_index": hop_index,
        "source_agent": hop_meta.get("source_identity").cloned().unwrap_or(Value::Null),
        "target_agent": hop_meta.get("target_identity").cloned().unwrap_or(Value::Null),
        "intent": hop_meta.get("intent").cloned().unwrap_or(Value::Null),
        "message": message,
        "ts": hop_meta.get("timestamp").cloned().unwrap_or(Value::Null),
        "previous_hop_hash": previous_hop_hash,
        "hop_hash": hop_hash,
        "web_tooling": {
            "detected": web_tooling_detected,
            "provider": web_tooling_provider,
            "query": web_tooling_query,
            "provider_contract_targets": web_tooling_provider_contract_targets()
        }
    });
    trace_entry["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&trace_entry));
    let _ = append_jsonl(&provenance_history_path(root), &trace_entry);
    let _ = write_json(&provenance_latest_path(root), &trace_entry);
    let visible = visible_trace_payload(&trace_entry, &visibility_mode);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "observability_plane_acp_provenance",
        "lane": "core/layer0/ops",
        "op": "trace",
        "trace_id": trace_entry.get("trace_id").cloned().unwrap_or(Value::Null),
        "hop": visible,
        "visibility_mode": visibility_mode,
        "web_tooling_provider_contract_targets": web_tooling_provider_contract_targets(),
        "artifact": {
            "history_path": provenance_history_path(root).display().to_string(),
            "latest_trace_path": provenance_latest_path(root).display().to_string()
        },
        "claim_evidence": [
            {
                "id": "V6-OBSERVABILITY-005.7",
                "claim": "inter_agent_messages_attach_source_timestamp_and_intent_metadata",
                "evidence": {
                    "source_agent": trace_entry.get("source_agent").cloned().unwrap_or(Value::Null),
                    "hop_index": hop_index
                }
            },
            {
                "id": "V6-OBSERVABILITY-005.8",
                "claim": "trace_id_propagates_across_hops_with_deterministic_chain_hashes",
                "evidence": {
                    "trace_id": trace_entry.get("trace_id").cloned().unwrap_or(Value::Null),
                    "hop_hash": trace_entry.get("hop_hash").cloned().unwrap_or(Value::Null)
                }
            },
            {
                "id": "V6-OBSERVABILITY-005.9",
                "claim": "trace_visibility_modes_gate_metadata_and_receipt_detail_surfaces",
                "evidence": {
                    "visibility_mode": visibility_mode
                }
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
                "type": "observability_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "monitor" => run_monitor(root, &parsed, strict),
        "workflow" => run_workflow(root, &parsed, strict),
        "incident" => run_incident(root, &parsed, strict),
        "selfhost" => run_selfhost(root, &parsed, strict),
        "acp-provenance" => run_acp_provenance(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "observability_plane_error",
            "error": "unknown_command",
            "command": command
        }),
    };
    if command == "status" {
        print_payload(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["monitor".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "monitor");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn workflow_upsert_creates_registry() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_workflow(
            root.path(),
            &crate::parse_args(&[
                "workflow".to_string(),
                "--op=upsert".to_string(),
                "--workflow-id=obs-main".to_string(),
            ]),
            true,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(workflows_state_path(root.path()).exists());
    }
}
