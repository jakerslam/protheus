fn normalize_web_tooling_provider_id(raw: &str) -> String {
    match clean(raw.replace('_', "-"), 80).to_ascii_lowercase().as_str() {
        "google" => "gemini".to_string(),
        "xai" => "grok".to_string(),
        "moonshot" => "kimi".to_string(),
        other => other.to_string(),
    }
}

fn run_csi_module(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CSI_MODULE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "substrate_csi_module_registry_contract",
            "allowed_ops": ["register", "activate", "list"],
            "allowed_privacy_classes": ["local", "sensitive", "restricted"],
            "required_fields": ["input_contract", "budget_units", "privacy_class", "degrade_behavior"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("substrate_csi_module_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "substrate_csi_module_registry_contract"
    {
        errors.push("substrate_csi_module_contract_kind_invalid".to_string());
    }
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "list".to_string()),
        20,
    )
    .to_ascii_lowercase();
    if strict
        && !contract
            .get("allowed_ops")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == op)
    {
        errors.push("substrate_csi_module_op_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_csi_module",
            "errors": errors
        });
    }

    let path = csi_module_registry_path(root);
    let mut registry = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "modules": {},
            "activations": []
        })
    });
    if !registry
        .get("modules")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        registry["modules"] = Value::Object(serde_json::Map::new());
    }
    if !registry
        .get("activations")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        registry["activations"] = Value::Array(Vec::new());
    }

    if op == "list" {
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "substrate_plane_csi_module",
            "lane": "core/layer0/ops",
            "op": op,
            "registry": registry,
            "claim_evidence": [
                {
                    "id": "V6-SUBSTRATE-001.2",
                    "claim": "csi_module_registry_lists_registered_and_activated_modules",
                    "evidence": {
                        "module_count": registry
                            .get("modules")
                            .and_then(Value::as_object)
                            .map(|m| m.len())
                            .unwrap_or(0)
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    let module = clean(
        parsed
            .flags
            .get("module")
            .cloned()
            .or_else(|| parsed.positional.get(2).cloned())
            .unwrap_or_default(),
        120,
    );
    if strict && module.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_csi_module",
            "errors": ["substrate_csi_module_id_required"]
        });
    }

    let out = match op.as_str() {
        "register" => {
            let input_contract = clean(
                parsed
                    .flags
                    .get("input-contract")
                    .cloned()
                    .unwrap_or_else(|| "csi.normalized_events.v1".to_string()),
                120,
            );
            let budget_units = parse_u64(parsed.flags.get("budget-units"), 1000);
            let privacy_class = clean(
                parsed
                    .flags
                    .get("privacy-class")
                    .cloned()
                    .unwrap_or_else(|| "local".to_string()),
                30,
            )
            .to_ascii_lowercase();
            let degrade_behavior = clean(
                parsed
                    .flags
                    .get("degrade-behavior")
                    .cloned()
                    .unwrap_or_else(|| "drop-to-presence-only".to_string()),
                120,
            );
            let web_tooling_provider = normalize_web_tooling_provider_id(
                parsed
                    .flags
                    .get("web-tooling-provider")
                    .map(String::as_str)
                    .or_else(|| parsed.flags.get("web-provider").map(String::as_str))
                    .unwrap_or(""),
            );
            let web_tooling_requires_auth = parsed
                .flags
                .get("web-tooling-requires-auth")
                .or_else(|| parsed.flags.get("web-requires-auth"))
                .map(|raw| {
                    matches!(
                        clean(raw.clone(), 12).to_ascii_lowercase().as_str(),
                        "1" | "true" | "yes" | "on"
                    )
                })
                .unwrap_or(true);
            let privacy_allowed = contract
                .get("allowed_privacy_classes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
                .any(|row| row == privacy_class);
            if strict && !privacy_allowed {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "substrate_plane_csi_module",
                    "errors": ["substrate_csi_module_privacy_class_invalid"]
                });
            }
            if strict
                && contract
                    .get("required_fields")
                    .and_then(Value::as_array)
                    .map(|required| {
                        required
                            .iter()
                            .filter_map(Value::as_str)
                            .any(|row| row == "degrade_behavior")
                    })
                    .unwrap_or(false)
                && degrade_behavior.is_empty()
            {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "substrate_plane_csi_module",
                    "errors": ["substrate_csi_module_degrade_behavior_required"]
                });
            }
            let module_doc = json!({
                "module": module,
                "input_contract": input_contract,
                "budget_units": budget_units,
                "privacy_class": privacy_class,
                "degrade_behavior": degrade_behavior,
                "web_tooling": {
                    "provider": if web_tooling_provider.is_empty() { Value::Null } else { Value::String(web_tooling_provider) },
                    "requires_auth": web_tooling_requires_auth,
                    "diagnostic_codes": [
                        "WEB_SEARCH_PROVIDER_INVALID_AUTODETECT",
                        "WEB_SEARCH_KEY_UNRESOLVED_NO_FALLBACK",
                        "WEB_FETCH_PROVIDER_KEY_UNRESOLVED_NO_FALLBACK"
                    ]
                },
                "registered_at": crate::now_iso(),
                "active": false
            });
            registry["modules"][&module] = module_doc.clone();
            registry["updated_at"] = Value::String(crate::now_iso());
            let _ = write_json(&path, &registry);
            let _ = append_jsonl(
                &state_root(root)
                    .join("csi")
                    .join("modules")
                    .join("history.jsonl"),
                &json!({"op": "register", "module": module_doc, "ts": crate::now_iso()}),
            );
            json!({
                "ok": true,
                "strict": strict,
                "type": "substrate_plane_csi_module",
                "lane": "core/layer0/ops",
                "op": op,
                "module": module_doc,
                "artifact": {
                    "path": path.display().to_string(),
                    "sha256": sha256_hex_str(&registry.to_string())
                }
            })
        }
        "activate" => {
            if strict && !registry["modules"].get(&module).is_some() {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "substrate_plane_csi_module",
                    "errors": ["substrate_csi_module_not_registered"]
                });
            }
            let web_requires_auth = registry["modules"][&module]
                .pointer("/web_tooling/requires_auth")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if strict && web_requires_auth {
                let env_map = std::env::vars().collect::<std::collections::HashMap<String, String>>();
                let auth_sources = crate::contract_lane_utils::web_tooling_auth_sources_from_env(&env_map);
                if auth_sources.is_empty() {
                    return json!({
                        "ok": false,
                        "strict": strict,
                        "type": "substrate_plane_csi_module",
                        "errors": ["substrate_csi_module_web_tooling_auth_missing"]
                    });
                }
            }
            registry["modules"][&module]["active"] = Value::Bool(true);
            registry["modules"][&module]["activated_at"] = Value::String(crate::now_iso());
            let activation = json!({
                "module": module,
                "activation_id": format!("act_{}", &sha256_hex_str(&format!("{}:{}", module, crate::now_iso()))[..10]),
                "ts": crate::now_iso()
            });
            let mut activations = registry
                .get("activations")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            activations.push(activation.clone());
            registry["activations"] = Value::Array(activations);
            registry["updated_at"] = Value::String(crate::now_iso());
            let _ = write_json(&path, &registry);
            let _ = append_jsonl(
                &state_root(root)
                    .join("csi")
                    .join("modules")
                    .join("history.jsonl"),
                &json!({"op": "activate", "activation": activation, "ts": crate::now_iso()}),
            );
            json!({
                "ok": true,
                "strict": strict,
                "type": "substrate_plane_csi_module",
                "lane": "core/layer0/ops",
                "op": op,
                "activation": activation,
                "artifact": {
                    "path": path.display().to_string(),
                    "sha256": sha256_hex_str(&registry.to_string())
                }
            })
        }
        _ => json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_csi_module",
            "errors": ["substrate_csi_module_op_invalid"]
        }),
    };
    let mut out = out;
    let mut claims = out
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    claims.push(json!({
        "id": "V6-SUBSTRATE-001.2",
        "claim": "csi_derived_module_registry_tracks_input_contract_budget_privacy_and_activation_receipts",
        "evidence": {
            "op": op,
            "module": module
        }
    }));
    out["claim_evidence"] = Value::Array(claims);
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_csi_embedded_profile(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CSI_EMBEDDED_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "substrate_csi_embedded_profile_contract",
            "profiles": {
                "esp32": {
                    "max_power_mw": 450.0,
                    "max_latency_ms": 250.0,
                    "max_bounded_memory_kb": 512,
                    "offline_required": true
                }
            }
        }),
    );
    let target = clean(
        parsed
            .flags
            .get("target")
            .cloned()
            .unwrap_or_else(|| "esp32".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let profile = contract
        .get("profiles")
        .and_then(|v| v.get(&target))
        .cloned()
        .unwrap_or(Value::Null);
    if strict && profile.is_null() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_csi_embedded_profile",
            "errors": ["substrate_csi_embedded_profile_unknown_target"]
        });
    }
    let power_mw = parse_f64(parsed.flags.get("power-mw"), 380.0);
    let latency_ms = parse_f64(parsed.flags.get("latency-ms"), 120.0);
    let bounded_memory_kb = parse_u64(parsed.flags.get("bounded-memory-kb"), 256);
    let offline = parse_bool(parsed.flags.get("offline"), true);
    let max_power = profile
        .get("max_power_mw")
        .and_then(Value::as_f64)
        .unwrap_or(450.0);
    let max_latency = profile
        .get("max_latency_ms")
        .and_then(Value::as_f64)
        .unwrap_or(250.0);
    let max_memory_kb = profile
        .get("max_bounded_memory_kb")
        .and_then(Value::as_u64)
        .unwrap_or(512);
    let offline_required = profile
        .get("offline_required")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mut reason_codes = Vec::<String>::new();
    if power_mw > max_power {
        reason_codes.push("power_budget_exceeded".to_string());
    }
    if latency_ms > max_latency {
        reason_codes.push("latency_budget_exceeded".to_string());
    }
    if bounded_memory_kb > max_memory_kb {
        reason_codes.push("bounded_memory_budget_exceeded".to_string());
    }
    if offline_required && !offline {
        reason_codes.push("offline_first_required".to_string());
    }
    let degraded_mode = !reason_codes.is_empty();
    let profile_state = json!({
        "version": "v1",
        "target": target,
        "power_mw": power_mw,
        "latency_ms": latency_ms,
        "bounded_memory_kb": bounded_memory_kb,
        "offline": offline,
        "degraded_mode": degraded_mode,
        "reason_codes": reason_codes,
        "telemetry": {
            "power_mw": power_mw,
            "latency_ms": latency_ms,
            "bounded_memory_kb": bounded_memory_kb
        },
        "ts": crate::now_iso()
    });
    let path = csi_embedded_profile_path(root, &target);
    let _ = write_json(&path, &profile_state);
    let _ = append_jsonl(
        &state_root(root)
            .join("csi")
            .join("embedded")
            .join("history.jsonl"),
        &profile_state,
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "substrate_plane_csi_embedded_profile",
        "lane": "core/layer0/ops",
        "profile": profile_state,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&profile_state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-SUBSTRATE-001.3",
                "claim": "embedded_csi_profile_tracks_power_latency_offline_and_degraded_mode_receipts",
                "evidence": {
                    "target": target,
                    "degraded_mode": degraded_mode
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
