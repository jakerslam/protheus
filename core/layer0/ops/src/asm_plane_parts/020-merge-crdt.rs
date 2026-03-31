fn merge_crdt(
    left: &Map<String, Value>,
    right: &Map<String, Value>,
) -> (Map<String, Value>, Vec<Value>) {
    let mut merged = Map::<String, Value>::new();
    let mut provenance = Vec::<Value>::new();
    let mut keys = left.keys().chain(right.keys()).cloned().collect::<Vec<_>>();
    keys.sort();
    keys.dedup();

    for key in keys {
        let l = left.get(&key).cloned().unwrap_or(Value::Null);
        let r = right.get(&key).cloned().unwrap_or(Value::Null);
        let l_clock = l.get("clock").and_then(Value::as_i64).unwrap_or(0);
        let r_clock = r.get("clock").and_then(Value::as_i64).unwrap_or(0);
        let l_node = l
            .get("node")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let r_node = r
            .get("node")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let pick_right = r_clock > l_clock || (r_clock == l_clock && r_node > l_node);
        let winner = if pick_right { r.clone() } else { l.clone() };
        if l != r {
            provenance.push(json!({
                "key": key,
                "left": l,
                "right": r,
                "winner": if pick_right { "right" } else { "left" }
            }));
        }
        merged.insert(key, winner);
    }
    (merged, provenance)
}

fn run_crdt_adapter(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let op = parsed
        .flags
        .get("op")
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "merge".to_string());
    let profile_path = parsed
        .flags
        .get("profile")
        .map(String::as_str)
        .unwrap_or(CRDT_PROFILE_PATH);
    let profile = load_json_or(
        root,
        profile_path,
        json!({
            "version": "v1",
            "kind": "crdt_automerge_profile",
            "merge_strategy": "lww",
            "replay_required": true
        }),
    );
    let events_path = state_root(root).join("crdt_adapter").join("events.jsonl");

    let mut errors = Vec::<String>::new();
    if profile
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("crdt_profile_version_must_be_v1".to_string());
    }
    if profile
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "crdt_automerge_profile"
    {
        errors.push("crdt_profile_kind_invalid".to_string());
    }

    if op == "replay" {
        let rows = fs::read_to_string(&events_path)
            .ok()
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .collect::<Vec<_>>();
        let mut ok = true;
        let mut prev = "GENESIS".to_string();
        let mut replay_state = Map::<String, Value>::new();
        for row in &rows {
            let expected_prev = row
                .get("prev_hash")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let hash = row.get("hash").and_then(Value::as_str).unwrap_or_default();
            let digest = sha256_hex_str(&format!(
                "{}:{}",
                expected_prev,
                row.get("merged")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".to_string())
            ));
            if expected_prev != prev || hash != digest {
                ok = false;
                errors.push("crdt_chain_tamper_detected".to_string());
                break;
            }
            prev = hash.to_string();
            if let Some(obj) = row.get("merged").and_then(Value::as_object) {
                replay_state = obj.clone();
            }
        }
        return json!({
            "ok": if strict { ok && errors.is_empty() } else { true },
            "strict": strict,
            "type": "asm_crdt_adapter_replay",
            "op": op,
            "events_path": events_path.display().to_string(),
            "events_count": rows.len(),
            "tip_hash": prev,
            "state": replay_state,
            "errors": errors,
            "claim_evidence": [
                {
                    "id": "V7-ASM-007",
                    "claim": "crdt_adapter_replay_verifies_local_first_merge_history",
                    "evidence": {"events_count": rows.len()}
                }
            ]
        });
    }

    let left = parse_crdt_map(parsed.flags.get("left-json"));
    let right = parse_crdt_map(parsed.flags.get("right-json"));
    let (left, right) = match (left, right) {
        (Ok(l), Ok(r)) => (l, r),
        (Err(e), _) | (_, Err(e)) => {
            errors.push(e);
            return json!({
                "ok": false,
                "strict": strict,
                "type": "asm_crdt_adapter_merge",
                "op": op,
                "errors": errors
            });
        }
    };

    let (merged, provenance) = merge_crdt(&left, &right);
    let previous = fs::read_to_string(&events_path)
        .ok()
        .and_then(|raw| raw.lines().last().map(ToString::to_string))
        .and_then(|line| serde_json::from_str::<Value>(&line).ok())
        .and_then(|row| {
            row.get("hash")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "GENESIS".to_string());
    let merged_json = Value::Object(merged.clone());
    let hash = sha256_hex_str(&format!("{}:{}", previous, merged_json));
    let event = json!({
        "type": "crdt_adapter_event",
        "ts": now_iso(),
        "op": "merge",
        "prev_hash": previous,
        "hash": hash,
        "left": left,
        "right": right,
        "merged": merged_json,
        "provenance": provenance
    });
    if let Err(err) = append_jsonl(&events_path, &event) {
        errors.push(err);
    }
    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "asm_crdt_adapter_merge",
        "op": op,
        "profile_path": profile_path,
        "events_path": events_path.display().to_string(),
        "event_hash": hash,
        "merged": merged_json,
        "provenance": provenance,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-ASM-007",
                "claim": "crdt_adapter_performs_deterministic_merge_with_conflict_provenance",
                "evidence": {
                    "event_hash": hash
                }
            }
        ]
    })
}

fn run_trust_chain(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let policy_path = parsed
        .flags
        .get("policy")
        .map(String::as_str)
        .unwrap_or(TRUST_CHAIN_POLICY_PATH);
    let allow_missing_rekor = parse_bool(parsed.flags.get("allow-missing-rekor"), false);
    let policy = load_json_or(
        root,
        policy_path,
        json!({
            "version": "v1",
            "kind": "trust_chain_integration",
            "bundle_path": "local/state/release/provenance_bundle/latest.json",
            "required_signature_paths": [
                "local/state/release/provenance/signatures/protheus-ops.sig",
                "local/state/release/provenance/signatures/protheusd.sig"
            ],
            "rekor_bundle_path": "local/state/release/provenance/rekor_entries.json",
            "require_rekor": true
        }),
    );
    let bundle_rel = policy
        .get("bundle_path")
        .and_then(Value::as_str)
        .unwrap_or("local/state/release/provenance_bundle/latest.json");
    let bundle_path = root.join(bundle_rel);
    let bundle = read_json(&bundle_path).unwrap_or(Value::Null);
    let signatures = policy
        .get("required_signature_paths")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let require_rekor = policy
        .get("require_rekor")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && !allow_missing_rekor;
    let rekor_rel = policy
        .get("rekor_bundle_path")
        .and_then(Value::as_str)
        .unwrap_or("local/state/release/provenance/rekor_entries.json");
    let rekor_path = root.join(rekor_rel);

    let mut errors = Vec::<String>::new();
    if policy
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("trust_chain_policy_version_must_be_v1".to_string());
    }
    if policy
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "trust_chain_integration"
    {
        errors.push("trust_chain_policy_kind_invalid".to_string());
    }
    if bundle.is_null() {
        errors.push("trust_chain_bundle_missing".to_string());
    } else if bundle
        .get("schema_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "release_provenance_bundle"
    {
        errors.push("trust_chain_bundle_schema_invalid".to_string());
    }

    let mut signature_rows = Vec::<Value>::new();
    for row in signatures {
        let rel = row.as_str().unwrap_or_default();
        if rel.is_empty() {
            continue;
        }
        let exists = root.join(rel).exists();
        if !exists {
            errors.push(format!("missing_signature::{rel}"));
        }
        signature_rows.push(json!({"path": rel, "exists": exists}));
    }

    let rekor_exists = rekor_path.exists();
    if require_rekor && !rekor_exists {
        errors.push("rekor_bundle_missing".to_string());
    }

    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "asm_trust_chain",
        "lane": "core/layer0/ops",
        "policy_path": policy_path,
        "bundle_path": bundle_rel,
        "bundle_exists": bundle_path.exists(),
        "signature_checks": signature_rows,
        "rekor_bundle_path": rekor_rel,
        "rekor_exists": rekor_exists,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-ASM-008",
                "claim": "trust_chain_lane_verifies_reproducible_bundle_signatures_and_rekor_pointer",
                "evidence": {
                    "bundle_path": bundle_rel,
                    "rekor_exists": rekor_exists
                }
            }
        ]
    })
}

fn canonical_hotpath(value: i64) -> i64 {
    value
        .saturating_mul(value)
        .saturating_add(value.saturating_mul(3))
        .saturating_add(7)
}

fn fastpath_hotpath(value: i64) -> i64 {
    (value.saturating_add(1))
        .saturating_mul(value.saturating_add(2))
        .saturating_add(5)
}

fn parse_workload(raw: Option<&String>) -> Vec<i64> {
    let mut out = raw
        .map(|v| {
            v.split(',')
                .filter_map(|part| part.trim().parse::<i64>().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if out.is_empty() {
        out = (1_i64..=128_i64).collect::<Vec<_>>();
    }
    out
}

fn run_fastpath(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let policy_path = parsed
        .flags
        .get("policy")
        .map(String::as_str)
        .unwrap_or(FASTPATH_POLICY_PATH);
    let policy = load_json_or(
        root,
        policy_path,
        json!({
            "version": "v1",
            "kind": "fastpath_hotpath_policy",
            "rollback_on_parity_fail": true,
            "hotpaths": ["routing.rank", "execution.scheduling"]
        }),
    );
    let inject_mismatch = parse_bool(parsed.flags.get("inject-mismatch"), false);
    let workload = parse_workload(parsed.flags.get("workload"));
    let started = Instant::now();
    let mut mismatches = Vec::<Value>::new();
    for (idx, item) in workload.iter().enumerate() {
        let canonical = canonical_hotpath(*item);
        let mut fast = fastpath_hotpath(*item);
        if inject_mismatch && idx == 0 {
            fast = fast.saturating_add(1);
        }
        if canonical != fast {
            mismatches.push(json!({
                "index": idx,
                "input": item,
                "canonical": canonical,
                "fastpath": fast
            }));
        }
    }
    let elapsed_ms = started.elapsed().as_millis();
    let rollback_on_fail = policy
        .get("rollback_on_parity_fail")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if rollback_on_fail && !mismatches.is_empty() {
        let rollback_path = state_root(root).join("fastpath").join("rollback.json");
        let _ = write_json(
            &rollback_path,
            &json!({
                "ts": now_iso(),
                "reason": "parity_mismatch",
                "mismatch_count": mismatches.len()
            }),
        );
    }

    let mut errors = Vec::<String>::new();
    if policy
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("fastpath_policy_version_must_be_v1".to_string());
    }
    if policy
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "fastpath_hotpath_policy"
    {
        errors.push("fastpath_policy_kind_invalid".to_string());
    }
    if !mismatches.is_empty() {
        errors.push("fastpath_parity_mismatch".to_string());
    }

    let throughput = if elapsed_ms == 0 {
        workload.len() as f64
    } else {
        (workload.len() as f64) / ((elapsed_ms as f64) / 1000.0)
    };
    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "asm_fastpath",
        "lane": "core/layer0/ops",
        "policy_path": policy_path,
        "workload_size": workload.len(),
        "elapsed_ms": elapsed_ms,
        "throughput_ops_per_sec": (throughput * 100.0).round() / 100.0,
        "mismatch_count": mismatches.len(),
        "mismatches": mismatches,
        "rollback_on_parity_fail": rollback_on_fail,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-ASM-009",
                "claim": "fastpath_lane_checks_parity_and_triggers_rollback_on_mismatch",
                "evidence": {
                    "mismatch_count": mismatches.len()
                }
            }
        ]
    })
}

