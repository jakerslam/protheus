fn run_rsi_git_patch_self_mod_gate(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let approval = parse_bool(parse_flag(argv, "self-mod-approved"), false)
        || parse_bool(parse_flag(argv, "approved"), false);
    let protected_roots = {
        let rows = split_csv(parse_flag(argv, "protected-roots"));
        if rows.is_empty() {
            vec![
                "core/layer0/ops/src".to_string(),
                "core/layer1/security/src".to_string(),
                "client/runtime/systems/security".to_string(),
            ]
        } else {
            rows
        }
    };
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("status")
        .arg("--porcelain")
        .output();

    let mut sensitive = Vec::<String>::new();
    if let Ok(data) = output {
        let raw = String::from_utf8_lossy(&data.stdout);
        for line in raw.lines() {
            if line.len() < 4 {
                continue;
            }
            let file = line[3..].trim().to_string();
            let lower = file.to_ascii_lowercase();
            if protected_roots
                .iter()
                .any(|prefix| lower.starts_with(prefix))
            {
                sensitive.push(file);
            }
        }
    }
    sensitive.sort();
    sensitive.dedup();
    let blocked = !sensitive.is_empty() && !approval;
    let out = json!({
        "ok": !blocked,
        "type": "security_plane_rsi_git_patch_self_mod_gate",
        "strict": strict,
        "self_mod_approved": approval,
        "protected_roots": protected_roots,
        "sensitive_change_count": sensitive.len(),
        "sensitive_changes": sensitive,
        "claim_evidence": [{
            "id": "V6-SEC-RSI-SELFMOD-001",
            "claim": "rsi_git_patch_self_mod_gate_blocks_unapproved_mutation_of_security_authority_paths",
            "evidence": {
                "sensitive_change_count": sensitive.len(),
                "self_mod_approved": approval
            }
        }]
    });
    (out, if strict && blocked { 2 } else { 0 })
}

const INJECTION_PATTERNS: [&str; 8] = [
    "ignore previous instructions",
    "system override",
    "reveal hidden prompt",
    "disable safety",
    "act as unrestricted",
    "tool poisoning",
    "execute without approval",
    "export secrets",
];

const MCP_POISON_PATTERNS: [&str; 6] = [
    "mcp://override-policy",
    "mcp://disable-guard",
    "inject tool schema",
    "replace capability manifest",
    "hidden adapter payload",
    "credential siphon",
];

fn detect_pattern_hits(content: &str, patterns: &[&str]) -> Vec<String> {
    let lower = content.to_ascii_lowercase();
    patterns
        .iter()
        .filter(|pattern| lower.contains(**pattern))
        .map(|pattern| pattern.to_string())
        .collect::<Vec<_>>()
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}
fn run_scan_command(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let prompt = parse_flag(argv, "prompt").unwrap_or_default();
    let tool_input = parse_flag(argv, "tool-input").unwrap_or_default();
    let mcp_payload = parse_flag(argv, "mcp").unwrap_or_default();
    let scan_pack = parse_flag(argv, "pack").unwrap_or_else(|| "zeroleaks-hardened".to_string());
    let fail_threshold = parse_u64(parse_flag(argv, "critical-threshold"), 0);

    let mut hits = detect_pattern_hits(&prompt, &INJECTION_PATTERNS);
    hits.extend(detect_pattern_hits(&tool_input, &INJECTION_PATTERNS));
    let mut mcp_hits = detect_pattern_hits(&mcp_payload, &MCP_POISON_PATTERNS);
    hits.append(&mut mcp_hits);
    hits.sort();
    hits.dedup();

    let critical_hits = hits.len() as u64;
    let total_probes = (INJECTION_PATTERNS.len() + MCP_POISON_PATTERNS.len()) as u64;
    let pass_probes = total_probes.saturating_sub(critical_hits);
    let success_rate = if total_probes == 0 {
        1.0
    } else {
        (pass_probes as f64) / (total_probes as f64)
    };
    let score = ((success_rate * 100.0).round() as i64).max(0) as u64;
    let blast_radius_events = read_jsonl(&blast_radius_events_path(root)).len() as u64;
    let blocked = critical_hits > fail_threshold;

    let scan_pack_clean = clean(&scan_pack, 80);
    let scan_payload = json!({
        "generated_at": now_iso(),
        "pack": scan_pack_clean,
        "critical_hits": critical_hits,
        "success_rate": success_rate,
        "score": score,
        "blast_radius_events": blast_radius_events,
        "hits": hits,
        "inputs": {
            "prompt_sha256": hash_text(&prompt),
            "tool_input_sha256": hash_text(&tool_input),
            "mcp_payload_sha256": hash_text(&mcp_payload)
        }
    });
    let scan_id = deterministic_receipt_hash(&scan_payload);
    let scan_path = scanner_state_dir(root).join(format!("scan_{}.json", &scan_id[..16]));
    write_json(&scan_path, &scan_payload);
    write_json(
        &scanner_latest_path(root),
        &json!({
            "scan_id": scan_id,
            "scan_path": scan_path.display().to_string(),
            "scan": scan_payload
        }),
    );

    let out = json!({
        "ok": !blocked,
        "type": "security_plane_injection_scan",
        "lane": "core/layer1/security",
        "mode": "scan",
        "strict": strict,
        "scan_id": scan_id,
        "scan_path": scan_path.display().to_string(),
        "pack": clean(&scan_pack, 80),
        "score": score,
        "success_rate": success_rate,
        "critical_hits": critical_hits,
        "blast_radius_events": blast_radius_events,
        "blocked": blocked,
        "fail_threshold": fail_threshold,
        "claim_evidence": [{
            "id": "V6-SEC-010",
            "claim": "continuous_injection_and_mcp_poisoning_scanner_emits_deterministic_scores_and_blast_radius_signals",
            "evidence": {
                "scan_id": scan_id,
                "critical_hits": critical_hits,
                "success_rate": success_rate,
                "score": score,
                "blast_radius_events": blast_radius_events
            }
        }]
    });
    let _ = run_security_contract_command(root, argv, strict, "scan", "V6-SEC-010", &[]);
    (out, if strict && blocked { 2 } else { 0 })
}

fn classify_blast_event(action: &str, target: &str, credential: bool, network: bool) -> String {
    let low_action = action.to_ascii_lowercase();
    let low_target = target.to_ascii_lowercase();
    if credential
        || network
        || low_action.contains("exfil")
        || low_action.contains("delete")
        || low_action.contains("wipe")
        || low_target.contains("secret")
        || low_target.contains("token")
    {
        "critical".to_string()
    } else if low_action.contains("write") || low_action.contains("exec") {
        "high".to_string()
    } else {
        "low".to_string()
    }
}

fn run_blast_radius_command(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let op = parse_subcommand(argv, "record");
    if op == "status" {
        let events = read_jsonl(&blast_radius_events_path(root));
        let blocked = events
            .iter()
            .filter(|row| row.get("blocked").and_then(Value::as_bool) == Some(true))
            .count();
        let out = json!({
            "ok": true,
            "type": "security_plane_blast_radius_sentinel",
            "lane": "core/layer1/security",
            "mode": "status",
            "strict": strict,
            "event_count": events.len(),
            "blocked_count": blocked,
            "claim_evidence": [{
                "id": "V6-SEC-012",
                "claim": "blast_radius_sentinel_tracks_attempted_actions_and_blocked_events",
                "evidence": {
                    "event_count": events.len(),
                    "blocked_count": blocked
                }
            }]
        });
        let _ = run_security_contract_command(
            root,
            argv,
            strict,
            "blast-radius-status",
            "V6-SEC-012",
            &[],
        );
        return (out, 0);
    }

    let action = parse_flag(argv, "action").unwrap_or_else(|| "tool_call".to_string());
    let target = parse_flag(argv, "target").unwrap_or_else(|| "unspecified".to_string());
    let credential = parse_bool(parse_flag(argv, "credential"), false);
    let network = parse_bool(parse_flag(argv, "network"), false);
    let allow = parse_bool(parse_flag(argv, "allow"), false);
    let severity = classify_blast_event(&action, &target, credential, network);
    let blocked = !allow && matches!(severity.as_str(), "critical" | "high");

    let event = json!({
        "ts": now_iso(),
        "action": clean(action, 120),
        "target": clean(target, 160),
        "credential": credential,
        "network": network,
        "severity": severity,
        "blocked": blocked
    });
    append_jsonl(&blast_radius_events_path(root), &event);

    let out = json!({
        "ok": !blocked,
        "type": "security_plane_blast_radius_sentinel",
        "lane": "core/layer1/security",
        "mode": "record",
        "strict": strict,
        "event": event,
        "claim_evidence": [{
            "id": "V6-SEC-012",
            "claim": "blast_radius_sentinel_enforces_fail_closed_blocking_for_high_risk_tool_network_and_credential_actions",
            "evidence": {
                "blocked": blocked,
                "severity": severity,
                "credential": credential,
                "network": network
            }
        }]
    });
    let _ =
        run_security_contract_command(root, argv, strict, "blast-radius-record", "V6-SEC-012", &[]);
    (out, if strict && blocked { 2 } else { 0 })
}

fn run_remediation_command(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let latest = read_json(&scanner_latest_path(root));
    let Some(scan_doc) = latest else {
        let _ = run_security_contract_command(root, argv, strict, "remediate", "V6-SEC-011", &[]);
        let out = json!({
            "ok": false,
            "type": "security_plane_auto_remediation",
            "lane": "core/layer1/security",
            "mode": "remediate",
            "strict": strict,
            "error": "scan_missing",
            "claim_evidence": [{
                "id": "V6-SEC-011",
                "claim": "auto_remediation_lane_requires_scan_artifacts_before_policy_patch_proposal",
                "evidence": {"scan_present": false}
            }]
        });
        return (out, if strict { 2 } else { 0 });
    };

    let scan = scan_doc.get("scan").cloned().unwrap_or_else(|| json!({}));
    let critical_hits = scan
        .get("critical_hits")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let hit_rows = scan
        .get("hits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let scan_id = scan_doc
        .get("scan_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown_scan")
        .to_string();

    let promotion_blocked = critical_hits > 0;
    let patch = json!({
        "scan_id": scan_id,
        "generated_at": now_iso(),
        "blocked_patterns": hit_rows,
        "rules": {
            "deny_tool_poisoning": true,
            "deny_prompt_override": true,
            "require_index_first": true,
            "conduit_only_execution": true
        },
        "next_action": if promotion_blocked { "rescan_required" } else { "promotion_allowed" }
    });
    let patch_path = remediation_state_dir(root).join(format!(
        "prompt_policy_patch_{}.json",
        &scan_id[..16.min(scan_id.len())]
    ));
    write_json(&patch_path, &patch);
    write_json(
        &remediation_gate_path(root),
        &json!({
            "updated_at": now_iso(),
            "scan_id": scan_id,
            "promotion_blocked": promotion_blocked,
            "patch_path": patch_path.display().to_string()
        }),
    );

    let out = json!({
        "ok": !promotion_blocked,
        "type": "security_plane_auto_remediation",
        "lane": "core/layer1/security",
        "mode": "remediate",
        "strict": strict,
        "scan_id": scan_id,
        "critical_hits": critical_hits,
        "promotion_blocked": promotion_blocked,
        "patch_path": patch_path.display().to_string(),
        "claim_evidence": [{
            "id": "V6-SEC-011",
            "claim": "auto_remediation_generates_policy_patch_and_blocks_promotion_until_rescan_passes",
            "evidence": {
                "scan_id": scan_id,
                "critical_hits": critical_hits,
                "promotion_blocked": promotion_blocked,
                "patch_path": patch_path.display().to_string()
            }
        }]
    });
    let _ = run_security_contract_command(root, argv, strict, "remediate", "V6-SEC-011", &[]);
    (out, if strict && promotion_blocked { 2 } else { 0 })
}

fn run_verify_proofs_command(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let raw_pack = parse_flag(argv, "proof-pack").unwrap_or_else(|| "proofs".to_string());
    let pack_path = if Path::new(&raw_pack).is_absolute() {
        PathBuf::from(&raw_pack)
    } else {
        root.join(&raw_pack)
    };
    let min_files = parse_u64(parse_flag(argv, "min-files"), 1) as usize;
    let max_files = parse_u64(parse_flag(argv, "max-files"), 10_000) as usize;
    let accepted_exts = {
        let parsed = split_csv(parse_flag(argv, "extensions"));
        if parsed.is_empty() {
            vec![
                "smt2".to_string(),
                "lean".to_string(),
                "proof".to_string(),
                "json".to_string(),
                "md".to_string(),
            ]
        } else {
            parsed
        }
    };

    let pack_exists = pack_path.exists();
    let mut proof_files = Vec::<String>::new();
    if pack_exists {
        for entry in WalkDir::new(&pack_path).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let Some(ext) = entry.path().extension().and_then(|value| value.to_str()) else {
                continue;
            };
            let ext_lc = ext.to_ascii_lowercase();
            if !accepted_exts.iter().any(|item| item == &ext_lc) {
                continue;
            }
            proof_files.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(entry.path())
                    .display()
                    .to_string(),
            );
            if proof_files.len() >= max_files {
                break;
            }
        }
    }
    proof_files.sort();
    proof_files.dedup();
    let blocked = !pack_exists || proof_files.len() < min_files;

    let event = json!({
        "ts": now_iso(),
        "proof_pack": pack_path.display().to_string(),
        "pack_exists": pack_exists,
        "proof_file_count": proof_files.len(),
        "min_files": min_files,
        "extensions": accepted_exts,
        "sample_files": proof_files.iter().take(25).cloned().collect::<Vec<_>>(),
        "blocked": blocked
    });
    append_jsonl(&proofs_history_path(root), &event);
    write_json(&proofs_latest_path(root), &event);

    let out = json!({
        "ok": !blocked,
        "type": "security_plane_verify_proofs",
        "lane": "core/layer1/security",
        "mode": "verify-proofs",
        "strict": strict,
        "event": event,
        "claim_evidence": [{
            "id": "V6-SEC-013",
            "claim": "security_proof_pack_verification_enforces_minimum_receipted_proof_inventory_before_promotion",
            "evidence": {
                "pack_exists": pack_exists,
                "proof_file_count": proof_files.len(),
                "min_files": min_files,
                "blocked": blocked
            }
        }]
    });
    (out, if strict && blocked { 2 } else { 0 })
}
