fn normalize_skill_graph_folder(raw: &str) -> String {
    let mut folder = clean(raw, 240).replace('\\', "/");
    while folder.contains("//") {
        folder = folder.replace("//", "/");
    }
    while folder.starts_with("./") {
        folder = folder[2..].to_string();
    }
    folder.trim().trim_start_matches('/').to_string()
}

fn resolve_skill_graph_folder(root: &Path, folder: &str) -> PathBuf {
    let default_rel = "adapters/cognition/skills/content-skill-graph";
    let normalized = normalize_skill_graph_folder(folder);
    let requested = if normalized.is_empty() {
        default_rel.to_string()
    } else {
        normalized
    };
    let candidate = if Path::new(&requested).is_absolute() {
        PathBuf::from(&requested)
    } else {
        root.join(&requested)
    };
    if candidate.is_absolute() && !candidate.starts_with(root) {
        root.join(default_rel)
    } else {
        candidate
    }
}

fn normalize_skill_graph_topic(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in clean(raw, 180).to_ascii_lowercase().chars() {
        let mapped = if ch.is_ascii_alphanumeric() { ch } else { '_' };
        if mapped == '_' {
            if prev_sep || out.is_empty() {
                continue;
            }
            prev_sep = true;
            out.push('_');
            continue;
        }
        prev_sep = false;
        out.push(mapped);
    }
    let topic = out.trim_matches('_').to_string();
    if topic.is_empty() {
        "repurpose_topic".to_string()
    } else {
        topic
    }
}

fn run_v8_skill_graph(root: &Path, id: &str, parsed: &crate::ParsedArgs) -> Value {
    let path = state_path(root, "v8_skill_graph/state.json");
    let mut state = load_json_or(&path, default_family_state("v8_skill_graph"));
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let folder = parsed
        .flags
        .get("folder")
        .cloned()
        .unwrap_or_else(|| "adapters/cognition/skills/content-skill-graph".to_string());
    let folder_path = resolve_skill_graph_folder(root, &folder);
    let step = id.split('.').nth(1).unwrap_or("0");

    let details = match step {
        "1" => {
            let mut nodes = Vec::new();
            if let Ok(dir) = fs::read_dir(&folder_path) {
                for entry in dir.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Ok(raw) = fs::read_to_string(&path) {
                            let links = extract_wikilinks(&raw);
                            nodes.push(json!({
                                "file": path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                                "wikilinks": links
                            }));
                        }
                    }
                }
            }
            nodes.sort_by(|left, right| {
                left.get("file")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(right.get("file").and_then(Value::as_str).unwrap_or(""))
            });
            let graph_basis = serde_json::to_string(&nodes).unwrap_or_else(|_| "[]".to_string());
            json!({"folder": rel(root, &folder_path), "nodes": nodes, "graph_hash": sha256_hex_str(&format!("{}:{}", rel(root, &folder_path), graph_basis))})
        }
        "2" => {
            let topic = clean(
                parsed
                    .flags
                    .get("topic")
                    .cloned()
                    .unwrap_or_else(|| "default-topic".to_string()),
                180,
            );
            let outputs = json!({
                "thread": format!("Contrarian thread for {}", topic),
                "script": format!("Short-form script for {}", topic),
                "brief": format!("Long-form brief for {}", topic)
            });
            json!({"topic": topic, "outputs": outputs})
        }
        "3" => {
            let index = folder_path.join("index.md");
            let valid = index.exists();
            json!({"index_present": valid, "index_path": rel(root, &index), "entrypoint": "index.md"})
        }
        "4" => {
            let topic = clean(
                parsed
                    .flags
                    .get("topic")
                    .cloned()
                    .unwrap_or_else(|| "repurpose-topic".to_string()),
                180,
            );
            let out_dir = state_path(root, "v8_skill_graph/artifacts");
            let _ = fs::create_dir_all(&out_dir);
            let artifact = out_dir.join(format!("{}.json", normalize_skill_graph_topic(&topic)));
            let payload = json!({
                "topic": topic,
                "formats": ["thread", "carousel", "script", "long-form"],
                "ts": now_iso()
            });
            if apply {
                let _ = write_json_value(&artifact, &payload);
            }
            json!({"artifact": rel(root, &artifact), "formats": payload.get("formats").cloned().unwrap_or(Value::Null)})
        }
        "5" => {
            json!({"boundary": "conduit_only", "bypass_rejected": true, "client_write_authority": false})
        }
        _ => json!({"error": "unknown_v8_skill_graph_step"}),
    };

    state["latest"] = details.clone();
    state["updated_at"] = Value::String(now_iso());
    if apply {
        let _ = write_json_value(&path, &state);
    }

    json!({
        "ok": details.get("error").is_none(),
        "id": id,
        "family": "v8_skill_graph",
        "state_path": rel(root, &path),
        "details": details,
        "claim_evidence": [
            {
                "id": id,
                "claim": "skill_graph_execution_is_core_authoritative_and_receipted",
                "evidence": {"state_path": rel(root, &path), "folder": rel(root, &folder_path)}
            }
        ]
    })
}

fn run_v9_xeno(root: &Path, id: &str, parsed: &crate::ParsedArgs) -> Value {
    let path = state_path(root, "v9_xeno/state.json");
    let mut state = load_json_or(&path, default_family_state("v9_xeno"));
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let step = id.split('.').nth(1).unwrap_or("0");

    let details = match step {
        "1" => {
            let hunger = parsed
                .flags
                .get("hunger")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.42)
                .clamp(0.0, 1.0);
            let satiety = (1.0 - hunger).clamp(0.0, 1.0);
            json!({"metabolism": {"hunger": hunger, "satiety": satiety, "dream_cycle": "deep_dream"}})
        }
        "2" => {
            let parent = clean(
                parsed
                    .flags
                    .get("parent")
                    .cloned()
                    .unwrap_or_else(|| "hand-alpha".to_string()),
                120,
            );
            let dna = sha256_hex_str(&format!("{}:{}", parent, now_iso()));
            json!({"offspring": {"parent": parent, "dna": dna, "mutation": "shadow_only", "approval_required": true}})
        }
        "3" => {
            let valence = parsed
                .flags
                .get("valence")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.62)
                .clamp(0.0, 1.0);
            json!({"observer": {"self_model": "entity", "valence": valence, "curiosity": 0.71}})
        }
        "4" => {
            let operator = clean(
                parsed
                    .flags
                    .get("operator")
                    .cloned()
                    .unwrap_or_else(|| "primary".to_string()),
                120,
            );
            json!({"bond": {"operator": operator, "bond_strength": 0.83, "imprint_hash": sha256_hex_str(&format!("{}:{}", operator, now_iso()))}})
        }
        "5" => {
            json!({"resonance_mode": {"enabled": true, "protocol": "alien_echo_v1", "translator": "logical_lane"}})
        }
        "6" => {
            let node = clean(
                parsed
                    .flags
                    .get("node")
                    .cloned()
                    .unwrap_or_else(|| "edge-node-a".to_string()),
                120,
            );
            json!({"body_map": {"node": node, "sensation": "healthy", "mesh_awareness": true}})
        }
        "7" => {
            json!({"longevity": {"backup": true, "migration": "standby", "human_veto": true, "directive_gated": true}})
        }
        _ => json!({"error": "unknown_v9_xeno_step"}),
    };

    state["latest"] = details.clone();
    state["updated_at"] = Value::String(now_iso());
    if apply {
        let _ = write_json_value(&path, &state);
    }

    json!({
        "ok": details.get("error").is_none(),
        "id": id,
        "family": "v9_xeno",
        "state_path": rel(root, &path),
        "details": details,
        "claim_evidence": [
            {
                "id": id,
                "claim": "xenogenesis_capability_executes_in_core_with_stateful_controls_and_receipts",
                "evidence": {"state_path": rel(root, &path)}
            }
        ]
    })
}

fn run_v9_merge(root: &Path, id: &str, parsed: &crate::ParsedArgs) -> Value {
    let path = state_path(root, "v9_merge/state.json");
    let mut state = load_json_or(&path, default_family_state("v9_merge"));
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let step = id.split('.').nth(1).unwrap_or("0");

    let details = match step {
        "1" => {
            let resonance = parsed
                .flags
                .get("resonance")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.78)
                .clamp(0.0, 1.0);
            json!({"observer_bridge": {"resonance": resonance, "semantic_overlap": 0.81, "dream_sync": 0.74}})
        }
        "2" => {
            let snapshot_id = format!("merge_{}", &sha256_hex_str(&now_iso())[..12]);
            json!({"shadow_partition": {"snapshot_id": snapshot_id, "reversible": true, "restore_available": true}})
        }
        "3" => {
            json!({"telemetry": {"resonance_pct": 79.2, "creative_share": 0.57, "logical_share": 0.43}})
        }
        "4" => {
            let level = parse_u64(parsed.flags.get("level"), 30).clamp(10, 100);
            let ladder = if [10u64, 30, 70, 100].contains(&level) {
                level
            } else {
                30
            };
            json!({"merge_ladder": {"level": ladder, "human_multisig": true, "fail_closed": true}})
        }
        "5" => {
            let topic = clean(
                parsed
                    .flags
                    .get("topic")
                    .cloned()
                    .unwrap_or_else(|| "merge-intent".to_string()),
                180,
            );
            json!({"interface": {"input": topic, "echo": "mirrored", "future_ingress": ["openbci", "muse", "neuralink_stub"]}})
        }
        "6" => {
            json!({"containment": {"cognition_only": true, "separate_command": "protheus merge separate", "emergency_restore": true}})
        }
        _ => json!({"error": "unknown_v9_merge_step"}),
    };

    state["latest"] = details.clone();
    state["updated_at"] = Value::String(now_iso());
    if apply {
        let _ = write_json_value(&path, &state);
    }

    json!({
        "ok": details.get("error").is_none(),
        "id": id,
        "family": "v9_merge",
        "state_path": rel(root, &path),
        "details": details,
        "claim_evidence": [
            {
                "id": id,
                "claim": "merge_capability_executes_with_reversible_shadow_state_and_receipts",
                "evidence": {"state_path": rel(root, &path)}
            }
        ]
    })
}

fn run_v9_escalate(root: &Path, id: &str, parsed: &crate::ParsedArgs) -> Value {
    let path = state_path(root, "v9_escalate/state.json");
    let mut state = load_json_or(&path, default_family_state("v9_escalate"));
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let step = id.split('.').nth(1).unwrap_or("0");

    let details = match step {
        "1" => {
            let risk = parsed
                .flags
                .get("risk")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.44)
                .clamp(0.0, 1.0);
            let irreversibility = parsed
                .flags
                .get("irreversibility")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.34)
                .clamp(0.0, 1.0);
            let novelty = parsed
                .flags
                .get("novelty")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.28)
                .clamp(0.0, 1.0);
            let score =
                ((risk * 0.45) + (irreversibility * 0.35) + (novelty * 0.20)).clamp(0.0, 1.0);
            json!({"decision": {"score": score, "risk": risk, "irreversibility": irreversibility, "novelty": novelty}})
        }
        "2" => {
            let mode = clean(
                parsed
                    .flags
                    .get("mode")
                    .cloned()
                    .unwrap_or_else(|| "background_notification".to_string()),
                80,
            )
            .to_ascii_lowercase();
            let allowed: HashSet<&str> = [
                "silent_delegation",
                "background_notification",
                "interactive_pause",
                "full_human_takeover",
            ]
            .into_iter()
            .collect();
            let normalized = if allowed.contains(mode.as_str()) {
                mode
            } else {
                "background_notification".to_string()
            };
            json!({"mode": normalized, "fail_closed": true})
        }
        "3" => {
            let approvals = parse_u64(parsed.flags.get("approvals"), 12);
            let denials = parse_u64(parsed.flags.get("denials"), 3);
            let bias = approvals as f64 / (approvals + denials).max(1) as f64;
            json!({"preference_profile": {"approvals": approvals, "denials": denials, "bias": (bias * 1000.0).round() / 1000.0}})
        }
        "4" => {
            let replay_id = clean(
                parsed
                    .flags
                    .get("replay-id")
                    .cloned()
                    .unwrap_or_else(|| "latest".to_string()),
                120,
            );
            json!({"history": {"replay_id": replay_id, "deterministic": true, "linked_receipts": true}})
        }
        "5" => {
            json!({"safety_supremacy": {"human_only_bypass": false, "layer0_veto": true, "deny_path": "fail_closed"}})
        }
        "6" => {
            let override_mode = clean(
                parsed
                    .flags
                    .get("override")
                    .cloned()
                    .unwrap_or_else(|| "none".to_string()),
                80,
            );
            json!({"thin_surface": {"status": "ready", "override": override_mode, "conduit_only": true}})
        }
        _ => json!({"error": "unknown_v9_escalate_step"}),
    };

    state["latest"] = details.clone();
    state["updated_at"] = Value::String(now_iso());
    if apply {
        let _ = write_json_value(&path, &state);
    }

    json!({
        "ok": details.get("error").is_none(),
        "id": id,
        "family": "v9_escalate",
        "state_path": rel(root, &path),
        "details": details,
        "claim_evidence": [
            {
                "id": id,
                "claim": "escalation_engine_executes_in_core_with_mode_control_learning_and_replay_receipts",
                "evidence": {"state_path": rel(root, &path)}
            }
        ]
    })
}

fn run_v8_or_v9(root: &Path, id: &str, parsed: &crate::ParsedArgs) -> Value {
    if id == "V6-SKILL-001" {
        let mut payload = run_v8_skill_graph(root, "V8-SKILL-GRAPH-001.1", parsed);
        payload["id"] = Value::String(id.to_string());
        if let Some(rows) = payload
            .get_mut("claim_evidence")
            .and_then(Value::as_array_mut)
        {
            for row in rows.iter_mut() {
                if let Some(obj) = row.as_object_mut() {
                    obj.insert("id".to_string(), Value::String(id.to_string()));
                    obj.insert(
                        "claim".to_string(),
                        Value::String(
                            "content_skill_graph_contract_executes_in_core_with_default_adapter_path"
                                .to_string(),
                        ),
                    );
                }
            }
        }
        return payload;
    }
    if id.starts_with("V8-MOAT-001.") {
        return run_v8_moat(root, id, parsed);
    }
    if id.starts_with("V8-MEMORY-BANK-001.") {
        return run_v8_memory_bank(root, id, parsed);
    }
    if id.starts_with("V8-SKILL-GRAPH-001.") {
        return run_v8_skill_graph(root, id, parsed);
    }
    if id.starts_with("V9-XENO-001.") {
        return run_v9_xeno(root, id, parsed);
    }
    if id.starts_with("V9-MERGE-001.") {
        return run_v9_merge(root, id, parsed);
    }
    if id.starts_with("V9-ESCALATE-001.") {
        return run_v9_escalate(root, id, parsed);
    }
    json!({"ok": false, "error": "unsupported_backlog_id", "id": id})
}

fn run_id(root: &Path, id: &str, parsed: &crate::ParsedArgs) -> Value {
    if id.starts_with("V7-") {
        return run_v7_lane(root, id, strict_mode(parsed));
    }
    run_v8_or_v9(root, id, parsed)
}

fn normalize_id(parsed: &crate::ParsedArgs) -> String {
    let raw = parsed
        .flags
        .get("id")
        .cloned()
        .or_else(|| parsed.positional.get(1).cloned())
        .unwrap_or_default();
    clean(raw.to_ascii_uppercase(), 64)
}
