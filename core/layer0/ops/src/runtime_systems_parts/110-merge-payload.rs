
fn merge_payload(mut payload: Value, defaults: &Value) -> Value {
    let Some(payload_obj) = payload.as_object_mut() else {
        return defaults.clone();
    };
    if let Some(default_obj) = defaults.as_object() {
        for (k, v) in default_obj {
            payload_obj.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    payload
}
fn contract_command_allowed(command: &str) -> bool {
    matches!(
        command,
        "run" | "build" | "bootstrap" | "package" | "settle" | "status" | "verify"
    )
}

fn strict_for(system_id: &str, args: &[String]) -> bool {
    lane_utils::parse_bool(
        lane_utils::parse_flag(args, "strict", true).as_deref(),
        looks_like_contract_id(system_id),
    )
}

fn parse_limit(raw: Option<String>, fallback: usize, max: usize) -> usize {
    let parsed = raw
        .as_deref()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(fallback);
    parsed.clamp(1, max.max(1))
}

fn family_roi_weight(family: &str) -> i64 {
    match family {
        "security_sandbox_redteam" => 130,
        "f100_assurance" => 125,
        "swarm_runtime_scaling" => 120,
        "memory_depth_stack" => 116,
        "learning_rsi_pipeline" => 112,
        "automation_mission_stack" => 110,
        "skills_runtime_pack" => 108,
        "competitive_execution_moat" => 106,
        "power_execution" => 104,
        "organism_parallel_intelligence" => 102,
        "ecosystem_scale_v11" => 100,
        "ecosystem_scale_v8" => 95,
        "swarm_orchestration" => 93,
        _ => 80,
    }
}

fn contract_roi_boost(id: &str) -> i64 {
    if id.starts_with("V10-PERF-001.") {
        30
    } else if id.starts_with("V6-DASHBOARD-007.") || id.starts_with("V6-DASHBOARD-008.") {
        26
    } else if id.starts_with("V6-SECURITY-") || id.starts_with("V8-SECURITY-") {
        25
    } else if id.starts_with("V6-WORKFLOW-") || id.starts_with("V8-SWARM-") {
        20
    } else if id.starts_with("V6-MEMORY-") || id.starts_with("V8-MEMORY-") {
        18
    } else if id.starts_with("V7-F100-") {
        16
    } else if id.starts_with("V10-") || id.starts_with("V11-") {
        12
    } else {
        0
    }
}

fn profile_roi_score(profile: RuntimeSystemContractProfile) -> i64 {
    family_roi_weight(profile.family) + contract_roi_boost(profile.id)
}

fn manifest_payload() -> Value {
    let profiles = actionable_profiles();
    let mut by_family: BTreeMap<String, usize> = BTreeMap::new();
    let contracts = profiles
        .iter()
        .map(|profile| {
            *by_family
                .entry(profile.family.to_string())
                .or_insert(0usize) += 1;
            profile_json(*profile)
        })
        .collect::<Vec<_>>();

    let mut out = json!({
        "ok": true,
        "type": "runtime_systems_manifest",
        "lane": LANE_ID,
        "counts": {
            "contracts": profiles.len(),
            "families": by_family.len()
        },
        "families": by_family,
        "contracts": contracts
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn payload_sha(payload: &Value) -> String {
    let encoded = serde_json::to_vec(payload).unwrap_or_default();
    hex::encode(Sha256::digest(encoded))
}

fn status_payload(root: &Path, system_id: &str, command: &str) -> Value {
    let latest = lane_utils::read_json(&latest_path(root, system_id));
    let profile = profile_for(system_id);
    let mut out = json!({
        "ok": true,
        "type": "runtime_systems_status",
        "lane": LANE_ID,
        "command": command,
        "system_id": system_id,
        "latest_path": lane_utils::rel_path(root, &latest_path(root, system_id)),
        "history_path": lane_utils::rel_path(root, &history_path(root, system_id)),
        "has_state": latest.is_some(),
        "latest": latest,
        "contract_profile": profile.map(profile_json)
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn roi_sweep_payload(root: &Path, args: &[String]) -> Result<Value, String> {
    let profiles = actionable_profiles();
    let limit = parse_limit(
        lane_utils::parse_flag(args, "limit", true),
        400,
        profiles.len(),
    );
    let apply =
        lane_utils::parse_bool(lane_utils::parse_flag(args, "apply", true).as_deref(), true);
    let strict = lane_utils::parse_bool(
        lane_utils::parse_flag(args, "strict", true).as_deref(),
        true,
    );

    let mut ranked = profiles
        .iter()
        .copied()
        .map(|profile| (profile_roi_score(profile), profile))
        .collect::<Vec<(i64, RuntimeSystemContractProfile)>>();
    ranked.sort_by(|(score_a, profile_a), (score_b, profile_b)| {
        score_b
            .cmp(score_a)
            .then_with(|| profile_a.id.cmp(profile_b.id))
    });

    let mut executed = Vec::<Value>::new();
    let mut success = 0u64;
    let mut failed = 0u64;
    let mut failed_ids = Vec::<String>::new();
    for (score, profile) in ranked.into_iter().take(limit) {
        match execute_contract_lane(root, profile.id, apply, strict) {
            Ok(result) => {
                let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
                if ok {
                    success += 1;
                } else {
                    failed += 1;
                    failed_ids.push(profile.id.to_string());
                }
                executed.push(json!({
                    "id": profile.id,
                    "family": profile.family,
                    "roi_score": score,
                    "ok": ok,
                    "receipt_hash": result.get("receipt_hash").cloned().unwrap_or(Value::Null),
                    "artifacts_count": result.get("artifacts").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0)
                }));
            }
            Err(err) => {
                failed += 1;
                failed_ids.push(profile.id.to_string());
                executed.push(json!({
                    "id": profile.id,
                    "family": profile.family,
                    "roi_score": score,
                    "ok": false,
                    "error": err
                }));
            }
        }
    }

    let mut out = json!({
        "ok": failed == 0,
        "type": "runtime_systems_roi_sweep",
        "lane": LANE_ID,
        "apply": apply,
        "strict": strict,
        "limit_requested": limit,
        "selected_count": executed.len(),
        "total_actionable_contracts": profiles.len(),
        "success_count": success,
        "failed_count": failed,
        "failed_ids": failed_ids,
        "executed": executed,
        "claim_evidence": [{
            "id": "runtime_systems_roi_top_contract_sweep",
            "claim": "top_ranked_runtime_contracts_execute_with_fail_closed_receipted_lane",
            "evidence": {
                "limit_requested": limit,
                "selected_count": success + failed,
                "success_count": success,
                "failed_count": failed,
                "strict": strict,
                "apply": apply
            }
        }]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}

fn run_payload(
    root: &Path,
    system_id: &str,
    command: &str,
    args: &[String],
) -> Result<Value, String> {
    let apply_default = !read_only_command(command);
    let apply = lane_utils::parse_bool(
        lane_utils::parse_flag(args, "apply", true).as_deref(),
        apply_default,
    );
    let strict = strict_for(system_id, args);
    let profile = profile_for(system_id);
    if strict && looks_like_contract_id(system_id) && profile.is_none() {
        return Err(format!("unknown_runtime_contract_id:{system_id}"));
    }
    if strict && profile.is_some() && !contract_command_allowed(command) {
        return Err(format!("contract_command_not_allowed:{command}"));
    }
    let payload = payload_object(lane_utils::parse_flag(args, "payload-json", true).as_deref())?;
    let payload = if let Some(profile) = profile {
        merge_payload(payload, &contract_defaults(profile))
    } else {
        payload
    };
    let assimilation_execution = if profile.is_none() {
        execute_assimilation_protocol_for_system(
            root, system_id, command, &payload, args, apply, strict,
        )?
    } else {
        None
    };
    let contract_execution = if let Some(profile) = profile {
        execute_contract_profile(root, profile, &payload, apply, strict)?
    } else if let Some(execution) = assimilation_execution.clone() {
        execution
    } else {
        ContractExecution {
            summary: json!({}),
            claims: Vec::new(),
            artifacts: Vec::new(),
        }
    };
    let passthrough = collect_passthrough(args);
    let ts = now_iso();
    let mut row = json!({
        "type": "runtime_systems_run",
        "lane": LANE_ID,
        "command": command,
        "system_id": system_id,
        "ts": ts,
        "payload": payload,
        "payload_sha256": payload_sha(&payload),
        "passthrough": passthrough,
        "apply": apply,
        "strict": strict,
        "contract_execution": contract_execution.summary,
        "contract_profile": profile.map(profile_json)
    });
    row["ok"] = Value::Bool(true);
    row["receipt_hash"] = Value::String(receipt_hash(&row));

    if apply {
        lane_utils::write_json(&latest_path(root, system_id), &row)?;
        lane_utils::append_jsonl(&history_path(root, system_id), &row)?;
    }

    let mut out = json!({
        "ok": true,
        "type": "runtime_systems_run",
        "lane": LANE_ID,
        "command": command,
        "system_id": system_id,
        "apply": apply,
        "strict": strict,
        "latest_path": lane_utils::rel_path(root, &latest_path(root, system_id)),
        "history_path": lane_utils::rel_path(root, &history_path(root, system_id)),
        "payload_sha256": row.get("payload_sha256").cloned().unwrap_or(Value::Null),
        "contract_execution": row.get("contract_execution").cloned().unwrap_or(Value::Null),
        "artifacts": contract_execution.artifacts.clone(),
        "contract_profile": row.get("contract_profile").cloned().unwrap_or(Value::Null),
        "claim_evidence": [mutation_receipt_claim(system_id, command, apply, strict)]
    });
    if let Some(profile) = profile {
        let mut claims = vec![
            json!({
                "id": profile.id,
                "claim": "actionable_contract_id_routes_through_authoritative_runtime_system_plane",
                "evidence": {
                    "family": profile.family,
                    "objective": profile.objective,
                    "strict_conduit_only": profile.strict_conduit_only,
                    "strict_fail_closed": profile.strict_fail_closed
                }
            }),
            mutation_receipt_claim(system_id, command, apply, strict),
        ];
        claims.extend(contract_execution.claims);
        out["claim_evidence"] = Value::Array(claims);
    } else if !contract_execution.claims.is_empty() {
        let mut claims = vec![mutation_receipt_claim(system_id, command, apply, strict)];
        claims.extend(contract_execution.claims);
        out["claim_evidence"] = Value::Array(claims);
    }
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    Ok(out)
}

fn cli_error(argv: &[String], err: &str, exit_code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "runtime_systems_cli_error",
        "lane": LANE_ID,
        "argv": argv,
        "error": lane_utils::clean_text(Some(err), 300),
        "exit_code": exit_code
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = if command == "manifest" {
        Ok(manifest_payload())
    } else if command == "roi-sweep" {
        roi_sweep_payload(root, &argv[1..])
    } else if command == "entrypoint-context" {
        entrypoint_authority_context_payload(&argv[1..])
    } else {
        let system_id = system_id_from_args(&command, &argv[1..]);
        if system_id.is_empty() {
            print_json_line(&cli_error(argv, "system_id_missing", 2));
            return 2;
        }
        match command.as_str() {
            "status" | "verify" => Ok(status_payload(root, &system_id, &command)),
            _ => run_payload(root, &system_id, &command, &argv[1..]),
        }
    };

    match payload {
        Ok(out) => {
            let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&out);
            if ok {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_json_line(&cli_error(argv, &err, 2));
            2
        }
    }
}

pub fn execute_contract_lane(
    root: &Path,
    system_id: &str,
    apply: bool,
    strict: bool,
) -> Result<Value, String> {
    let args = vec![
        format!("--apply={}", if apply { 1 } else { 0 }),
        format!("--strict={}", if strict { 1 } else { 0 }),
    ];
    run_payload(root, system_id, "run", &args)
}
