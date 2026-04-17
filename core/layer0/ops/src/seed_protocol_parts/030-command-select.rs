fn clean_id(raw: Option<&str>, fallback: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in raw.unwrap_or_default().chars() {
        let next = if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };
        if next == '-' {
            if prev_sep {
                continue;
            }
            prev_sep = true;
        } else {
            prev_sep = false;
        }
        out.push(next);
        if out.len() >= 120 {
            break;
        }
    }
    let cleaned = out.trim_matches('-').to_string();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn command_select(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let profile = selected_profile(parsed);
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let action = format!("seed:select:{profile}");
    let gate_ok = gate_allowed(root, &action);
    if apply && !gate_ok {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "seed_protocol_select",
                "lane": "core/layer0/ops",
                "profile": profile,
                "error": "directive_gate_denied",
                "gate_action": action
            }),
        );
    }

    let top_k = parse_u64(parsed.flags.get("top"), 5).clamp(1, 50) as usize;
    let starved_threshold = parsed
        .flags
        .get("starved-threshold")
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .unwrap_or(10.0)
        .clamp(0.0, 1_000_000.0);
    let ledger = read_network_ledger(root);
    let balances = ledger
        .get("balances")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let stakes = ledger
        .get("staked")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut totals = std::collections::BTreeMap::<String, (f64, f64)>::new();
    for (node, bal) in &balances {
        let node_id = clean_id(Some(node.as_str()), "node-unknown");
        let entry = totals.entry(node_id).or_insert((0.0, 0.0));
        entry.0 += bal.as_f64().unwrap_or(0.0);
    }
    for (node, stake) in &stakes {
        let node_id = clean_id(Some(node.as_str()), "node-unknown");
        let entry = totals.entry(node_id).or_insert((0.0, 0.0));
        entry.1 += stake.as_f64().unwrap_or(0.0);
    }
    let mut scored = totals
        .into_iter()
        .map(|(node, (b, s))| {
            let score = b + (s * 2.0);
            (node, b, s, score)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| {
        b.3.partial_cmp(&a.3)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    let selected = scored
        .iter()
        .take(top_k)
        .enumerate()
        .map(|(idx, (node, b, s, score))| {
            json!({
                "rank": idx + 1,
                "node": node,
                "balance": b,
                "staked": s,
                "score": score
            })
        })
        .collect::<Vec<_>>();
    let starved = scored
        .iter()
        .skip(top_k)
        .filter(|(_, _, _, score)| *score < starved_threshold)
        .map(|(node, _, _, score)| {
            json!({
                "node": node,
                "score": score,
                "reason": format!("score<{starved_threshold}")
            })
        })
        .collect::<Vec<_>>();

    if apply {
        let mut state = load_state(root);
        let obj = state_obj_mut(&mut state);
        inc_counter(obj, "selection_rounds", 1);
        let history = arr_mut(obj, "selection_history");
        push_bounded(
            history,
            json!({
                "profile": profile,
                "top_k": top_k,
                "selected": selected,
                "starved": starved,
                "ts": now_iso()
            }),
            2048,
        );
        if let Err(err) = store_state(root, &state) {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "seed_protocol_select",
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
            "type": "seed_protocol_select",
            "lane": "core/layer0/ops",
            "profile": profile,
            "apply": apply,
            "top_k": top_k,
            "selected": selected,
            "starved": starved,
            "claim_evidence": [
                {
                    "id": profile_claim_id("4", &profile),
                    "claim": "evolutionary_selection_prioritizes_high_contribution_nodes_and_starves_low_value_nodes",
                    "evidence": {"selected_count": selected.len(), "starved_count": starved.len()}
                },
                {
                    "id": if profile == "viral" { "V9-IMMORTAL-001.3" } else { "V9-VIRAL-001.4" },
                    "claim": "selection_engine_behavior_is_shared_across_viral_and_immortal_profiles",
                    "evidence": {"profile": profile, "selected_count": selected.len()}
                }
            ]
        }),
    )
}

fn command_archive(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let profile = selected_profile(parsed);
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let action = format!("seed:archive:{profile}");
    let gate_ok = gate_allowed(root, &action);
    if apply && !gate_ok {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "seed_protocol_archive",
                "lane": "core/layer0/ops",
                "profile": profile,
                "error": "directive_gate_denied",
                "gate_action": action
            }),
        );
    }

    let lineage_id = clean(
        parsed
            .flags
            .get("lineage-id")
            .cloned()
            .unwrap_or_else(|| format!("lineage-{}", now_iso().replace([':', '.'], "-"))),
        160,
    );
    let last_packet_id = load_state(root)
        .get("packets")
        .and_then(Value::as_array)
        .and_then(|rows| rows.last())
        .and_then(|row| row.get("packet_id"))
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string();
    let archive_leaf = sha256_hex_str(&format!(
        "{lineage_id}:{profile}:{last_packet_id}:{}",
        directive_kernel::directive_vault_hash(root)
    ));

    let mut archive_merkle_root = Value::Null;
    if apply {
        let mut state = load_state(root);
        let obj = state_obj_mut(&mut state);
        inc_counter(obj, "archive_count", 1);
        let archives = arr_mut(obj, "archives");
        push_bounded(
            archives,
            json!({
                "lineage_id": lineage_id,
                "profile": profile,
                "leaf_hash": archive_leaf,
                "packet_id": last_packet_id,
                "ts": now_iso()
            }),
            4096,
        );
        let leaves = archives
            .iter()
            .filter_map(|row| row.get("leaf_hash").and_then(Value::as_str))
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let root_hash = deterministic_merkle_root(&leaves);
        obj.insert(
            "archive_merkle_root".to_string(),
            Value::String(root_hash.clone()),
        );
        archive_merkle_root = Value::String(root_hash);
        if let Err(err) = store_state(root, &state) {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "seed_protocol_archive",
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
            "type": "seed_protocol_archive",
            "lane": "core/layer0/ops",
            "profile": profile,
            "apply": apply,
            "lineage_id": lineage_id,
            "leaf_hash": archive_leaf,
            "archive_merkle_root": archive_merkle_root,
            "claim_evidence": [
                {
                    "id": profile_claim_id("5", &profile),
                    "claim": "deep_time_archive_is_merkle_linked_and_receipted_for_lineage_inheritance",
                    "evidence": {"lineage_id": lineage_id, "leaf_hash": archive_leaf}
                },
                {
                    "id": if profile == "viral" { "V9-IMMORTAL-001.4" } else { "V9-VIRAL-001.5" },
                    "claim": "genetic_archive_is_shared_between_profiles_with_profile_scoped_lineage_records",
                    "evidence": {"profile": profile}
                }
            ]
        }),
    )
}

fn command_defend(root: &Path, parsed: &crate::ParsedArgs) -> i32 {
    let profile = selected_profile(parsed);
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let action = "seed:defend";
    let gate_ok = gate_allowed(root, action);
    if apply && !gate_ok {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "seed_protocol_defend",
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
            .unwrap_or_else(|| "node-unknown".to_string()),
        120,
    );
    let signal = clean(
        parsed
            .flags
            .get("signal")
            .cloned()
            .unwrap_or_else(|| "tamper".to_string()),
        80,
    )
    .to_ascii_lowercase();
    let severity = clean(
        parsed
            .flags
            .get("severity")
            .cloned()
            .unwrap_or_else(|| "high".to_string()),
        24,
    )
    .to_ascii_lowercase();
    let quarantine = severity == "high" || severity == "critical" || signal == "tamper";

    if apply {
        let mut state = load_state(root);
        let obj = state_obj_mut(&mut state);
        inc_counter(obj, "defense_event_count", 1);
        let events = arr_mut(obj, "defense_events");
        push_bounded(
            events,
            json!({
                "profile": profile,
                "node": node,
                "signal": signal,
                "severity": severity,
                "quarantine": quarantine,
                "ts": now_iso()
            }),
            4096,
        );
        if quarantine {
            let q = obj_mut(obj, "quarantine");
            q.insert(
                node.clone(),
                json!({
                    "reason": format!("defense_signal:{signal}"),
                    "severity": severity,
                    "profile": profile,
                    "ts": now_iso()
                }),
            );
        }
        if let Err(err) = store_state(root, &state) {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "seed_protocol_defend",
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
            "type": "seed_protocol_defend",
            "lane": "core/layer0/ops",
            "profile": profile,
            "apply": apply,
            "node": node,
            "signal": signal,
            "severity": severity,
            "quarantine": quarantine,
            "claim_evidence": [
                {
                    "id": "V9-IMMORTAL-001.5",
                    "claim": "constitutional_self_defense_enforces_quarantine_and_anti_tamper_receipts",
                    "evidence": {"node": node, "quarantine": quarantine}
                },
                {
                    "id": "V9-VIRAL-001.2",
                    "claim": "anti_shutdown_survival_flow_preserves_state_under_attack_signals",
                    "evidence": {"signal": signal, "severity": severity}
                }
            ]
        }),
    )
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops seed-protocol status");
        println!("  protheus-ops seed-protocol deploy [--profile=viral|immortal] [--targets=a,b] [--replication-cap=<n>] [--apply=1|0]");
        println!("  protheus-ops seed-protocol migrate [--profile=viral|immortal] [--node=<id>] [--threat=normal|high|critical] [--energy=<0..1>] [--hardware=edge|cloud] [--apply=1|0]");
        println!("  protheus-ops seed-protocol enforce [--profile=viral|immortal] [--operation=replicate|migrate|mutate|network] [--node=<id>] [--apply=1|0]");
        println!("  protheus-ops seed-protocol select [--profile=viral|immortal] [--top=<n>] [--apply=1|0]");
        println!("  protheus-ops seed-protocol archive [--profile=viral|immortal] [--lineage-id=<id>] [--apply=1|0]");
        println!("  protheus-ops seed-protocol defend [--profile=viral|immortal] [--node=<id>] [--signal=tamper] [--severity=high] [--apply=1|0]");
        return 0;
    }

    match command.as_str() {
        "status" | "monitor" => command_status(root),
        "deploy" | "ignite" => command_deploy(root, &parsed),
        "migrate" => command_migrate(root, &parsed),
        "enforce" => command_enforce(root, &parsed),
        "select" => command_select(root, &parsed),
        "archive" => command_archive(root, &parsed),
        "defend" => command_defend(root, &parsed),
        _ => emit(
            root,
            json!({
                "ok": false,
                "type": "seed_protocol_error",
                "lane": "core/layer0/ops",
                "error": "unknown_command",
                "command": command,
                "exit_code": 2
            }),
        ),
    }
}
