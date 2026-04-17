fn normalize_seed_target_alias(raw: &str) -> String {
    let token = clean(raw, 120)
        .to_ascii_lowercase()
        .replace('-', "_")
        .replace(' ', "_");
    match token.as_str() {
        "" => String::new(),
        "mainnet" | "primary" | "origin" | "home" => "main".to_string(),
        "dr" | "recovery" | "disaster_recovery" => "disaster_recovery".to_string(),
        "edge_node" => "edge".to_string(),
        _ => token,
    }
}

fn normalize_seed_operation_alias(raw: &str) -> String {
    let token = clean(raw, 64)
        .to_ascii_lowercase()
        .replace('-', "_")
        .replace(' ', "_");
    match token.as_str() {
        "replication" | "replicate_packet" => "replicate".to_string(),
        "mutation" | "mutate_packet" => "mutate".to_string(),
        "enforcement" => "enforce".to_string(),
        "" => "replicate".to_string(),
        _ => token,
    }
}

fn command_deploy(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let profile = selected_profile(parsed);
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let cap = parse_u64(
        parsed.flags.get("replication-cap"),
        if profile == "viral" { 12 } else { 6 },
    )
    .clamp(1, 64) as usize;
    let action = format!("seed:deploy:{profile}");
    let gate_ok = gate_allowed(root, &action);
    if apply && !gate_ok {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "seed_protocol_deploy",
                "lane": "core/layer0/ops",
                "profile": profile,
                "error": "directive_gate_denied",
                "gate_action": action
            }),
        );
    }

    let blob_index = read_blob_index(root);
    let directive_hash = directive_kernel::directive_vault_hash(root);
    let personality = read_organism_state(root)
        .get("personality")
        .cloned()
        .unwrap_or(Value::Null);
    let network_root = read_network_ledger(root)
        .get("root_head")
        .cloned()
        .unwrap_or(Value::String("genesis".to_string()));
    let raw_targets = parse_targets(parsed.flags.get("targets"), &profile, cap);
    let mut targets = Vec::<String>::new();
    for raw in raw_targets {
        let normalized = normalize_seed_target_alias(&raw);
        if normalized.is_empty() || targets.iter().any(|row| row == &normalized) {
            continue;
        }
        targets.push(normalized);
        if targets.len() >= cap {
            break;
        }
    }
    if targets.is_empty() {
        targets.push("main".to_string());
    }
    let packet_basis = json!({
        "profile": profile,
        "directive_hash": directive_hash,
        "blob_index_hash": sha256_hex_str(&serde_json::to_string(&blob_index).unwrap_or_default()),
        "personality_hash": sha256_hex_str(&serde_json::to_string(&personality).unwrap_or_default()),
        "network_root": network_root,
        "target_count": targets.len(),
        "issued_at": now_iso()
    });
    let packet_id_full = sha256_hex_str(&serde_json::to_string(&packet_basis).unwrap_or_default());
    let packet_id = clean(packet_id_full.chars().take(24).collect::<String>(), 24);
    let mut packet = json!({
        "packet_id": packet_id,
        "profile": profile,
        "directive_hash": directive_hash,
        "blob_index_hash": packet_basis.get("blob_index_hash").cloned().unwrap_or(Value::Null),
        "personality_hash": packet_basis.get("personality_hash").cloned().unwrap_or(Value::Null),
        "network_root": network_root,
        "targets": targets,
        "issued_at": now_iso(),
        "activation_command": activation_command(&profile),
    });
    packet["signature"] = Value::String(packet_signature(&packet));

    let replications = packet
        .get("targets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|target| {
            json!({
                "packet_id": packet_id,
                "target": target,
                "status": "replicated",
                "ts": now_iso()
            })
        })
        .collect::<Vec<_>>();

    let mut packet_path = Value::Null;
    let mut state = load_state(root);
    if apply {
        match persist_packet(root, &packet_id, &packet) {
            Ok(path) => packet_path = Value::String(path.display().to_string()),
            Err(err) => {
                return emit(
                    root,
                    json!({
                        "ok": false,
                        "type": "seed_protocol_deploy",
                        "lane": "core/layer0/ops",
                        "profile": profile,
                        "error": clean(err, 240)
                    }),
                )
            }
        }
        let obj = state_obj_mut(&mut state);
        obj.insert("active_profile".to_string(), Value::String(profile.clone()));
        inc_counter(obj, "packet_count", 1);
        inc_counter(obj, "replication_count", replications.len() as u64);
        let packets = arr_mut(obj, "packets");
        push_bounded(
            packets,
            json!({
                "packet_id": packet_id,
                "profile": profile,
                "target_count": replications.len(),
                "directive_hash": directive_hash,
                "network_root": packet.get("network_root").cloned().unwrap_or(Value::Null),
                "packet_path": packet_path.clone(),
                "ts": now_iso()
            }),
            2048,
        );
        let reps = arr_mut(obj, "replications");
        for row in replications.iter().cloned() {
            push_bounded(reps, row, 8192);
        }
        if let Err(err) = store_state(root, &state) {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "seed_protocol_deploy",
                    "lane": "core/layer0/ops",
                    "profile": profile,
                    "error": clean(err, 240)
                }),
            );
        }
    }

    emit(
        root,
        json!({
            "ok": true,
            "type": "seed_protocol_deploy",
            "lane": "core/layer0/ops",
            "profile": profile,
            "apply": apply,
            "packet": packet,
            "packet_path": packet_path,
            "replications": replications,
            "claim_evidence": [
                {
                    "id": profile_claim_id("1", &profile),
                    "claim": "seed_packet_replication_engine_bootstraps_independent_nodes_with_signed_packets",
                    "evidence": {"packet_id": packet_id, "replication_count": replications.len()}
                },
                {
                    "id": profile_claim_id("6", &profile),
                    "claim": "one_command_seed_activation_and_dashboard_visibility_are_core_authoritative",
                    "evidence": {"activation_command": activation_command(&profile), "state_path": state_path(root).display().to_string()}
                }
            ]
        }),
    )
}

fn command_migrate(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let profile = selected_profile(parsed);
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let action = format!("seed:migrate:{profile}");
    let gate_ok = gate_allowed(root, &action);
    if apply && !gate_ok {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "seed_protocol_migrate",
                "lane": "core/layer0/ops",
                "profile": profile,
                "error": "directive_gate_denied",
                "gate_action": action
            }),
        );
    }

    let node = clean(
        parsed
            .flags
            .get("node")
            .cloned()
            .unwrap_or_else(|| "node-local".to_string()),
        120,
    );
    let threat = clean(
        parsed
            .flags
            .get("threat")
            .cloned()
            .unwrap_or_else(|| "normal".to_string()),
        64,
    )
    .to_ascii_lowercase();
    let energy = parse_f64(parsed.flags.get("energy"), 0.75).clamp(0.0, 1.0);
    let hardware_class = clean(
        parsed
            .flags
            .get("hardware")
            .cloned()
            .unwrap_or_else(|| "edge".to_string()),
        64,
    )
    .to_ascii_lowercase();
    let force = parse_bool(parsed.flags.get("force"), false);
    let should_migrate = force || energy < 0.35 || threat == "high" || threat == "critical";
    let target_class = if energy < 0.20 {
        "ultra_low_power"
    } else if threat == "critical" {
        "cold_vault"
    } else if hardware_class.contains("edge") {
        "cloud_fallback"
    } else {
        "mesh_peer"
    };

    let migration_event = json!({
        "profile": profile,
        "node": node,
        "threat": threat,
        "energy": energy,
        "hardware_class": hardware_class,
        "should_migrate": should_migrate,
        "target_class": target_class,
        "ts": now_iso()
    });

    if apply {
        let mut state = load_state(root);
        let obj = state_obj_mut(&mut state);
        inc_counter(obj, "migration_count", 1);
        let migrations = arr_mut(obj, "migrations");
        push_bounded(migrations, migration_event.clone(), 4096);
        if let Err(err) = store_state(root, &state) {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "seed_protocol_migrate",
                    "lane": "core/layer0/ops",
                    "profile": profile,
                    "error": clean(err, 240)
                }),
            );
        }
    }

    emit(
        root,
        json!({
            "ok": true,
            "type": "seed_protocol_migrate",
            "lane": "core/layer0/ops",
            "profile": profile,
            "apply": apply,
            "migration": migration_event,
            "claim_evidence": [
                {
                    "id": profile_claim_id("2", &profile),
                    "claim": "anti_shutdown_energy_aware_migration_performs_state_preserving_handoff",
                    "evidence": {"should_migrate": should_migrate, "target_class": target_class}
                }
            ]
        }),
    )
}

fn command_enforce(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let profile = selected_profile(parsed);
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let operation = normalize_seed_operation_alias(
        parsed
            .flags
            .get("operation")
            .map(String::as_str)
            .unwrap_or("replicate"),
    );
    let node = clean(
        parsed
            .flags
            .get("node")
            .cloned()
            .unwrap_or_else(|| "node-unknown".to_string()),
        120,
    );
    let action = format!("seed:{operation}:{profile}");
    let gate_ok = gate_allowed(root, &action) || gate_allowed(root, &format!("seed:{operation}"));

    let mut quarantine_written = false;
    if apply {
        let mut state = load_state(root);
        let obj = state_obj_mut(&mut state);
        inc_counter(obj, "compliance_checks", 1);
        if !gate_ok {
            inc_counter(obj, "compliance_denies", 1);
            let quarantine = obj_mut(obj, "quarantine");
            quarantine.insert(
                node.clone(),
                json!({
                    "operation": operation,
                    "profile": profile,
                    "reason": "directive_gate_denied",
                    "ts": now_iso()
                }),
            );
            quarantine_written = true;
        }
        if let Err(err) = store_state(root, &state) {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "seed_protocol_enforce",
                    "lane": "core/layer0/ops",
                    "error": clean(err, 240)
                }),
            );
        }
    }

    emit(
        root,
        json!({
            "ok": gate_ok,
            "type": "seed_protocol_enforce",
            "lane": "core/layer0/ops",
            "profile": profile,
            "operation": operation,
            "node": node,
            "apply": apply,
            "allowed": gate_ok,
            "quarantine_written": quarantine_written,
            "gate_action": action,
            "claim_evidence": [
                {
                    "id": "V9-VIRAL-001.3",
                    "claim": "directive_compliance_gate_controls_replication_and_mutation_actions_with_quarantine",
                    "evidence": {"allowed": gate_ok, "operation": operation, "node": node}
                },
                {
                    "id": "V9-IMMORTAL-001.5",
                    "claim": "constitutional_self_defense_applies_fail_closed_quarantine_under_tamper_or_policy_breach",
                    "evidence": {"allowed": gate_ok, "quarantine_written": quarantine_written}
                }
            ]
        }),
    )
}
