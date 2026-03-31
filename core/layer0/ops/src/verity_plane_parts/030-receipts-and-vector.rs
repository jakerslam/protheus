fn vector_classification(score: f64) -> &'static str {
    if score >= 0.95 {
        "Strong"
    } else if score >= 0.85 {
        "Weak"
    } else if score >= 0.70 {
        "Drifting"
    } else {
        "Compromised"
    }
}

fn verification_mode(fidelity_score: f64, cfg: &VerityPlaneSignedConfig) -> &'static str {
    if fidelity_score < cfg.fidelity_lock_threshold {
        "judicial_lock"
    } else if fidelity_score < cfg.fidelity_warning_threshold {
        "verification_mode"
    } else {
        "normal"
    }
}

fn latest_receipt_hash(root: &Path) -> String {
    read_json(&verity_latest_path(root))
        .and_then(|payload| {
            payload
                .get("receipt_hash")
                .and_then(Value::as_str)
                .map(|value| value.to_string())
        })
        .unwrap_or_else(|| "genesis".to_string())
}

fn latest_fidelity_score(root: &Path) -> f64 {
    read_json(&verity_latest_path(root))
        .and_then(|payload| payload.get("fidelity_score").and_then(Value::as_f64))
        .unwrap_or(1.0)
}

fn latest_vector_alignment(root: &Path) -> f64 {
    read_json(&verity_latest_path(root))
        .and_then(|payload| payload.get("vector_alignment").and_then(Value::as_f64))
        .unwrap_or(1.0)
}

fn build_ultimate_vector_payload() -> Value {
    json!({
        "id": ULTIMATE_VECTOR_ID,
        "description": ULTIMATE_VECTOR_DESCRIPTION,
    })
}

fn build_verity_receipt(
    root: &Path,
    cfg: &VerityPlaneSignedConfig,
    operation_type: &str,
    fidelity_score: f64,
    vector_alignment: f64,
    drift_delta: f64,
    known_invariants: Vec<String>,
    truth_tags: Vec<String>,
    metadata: Value,
) -> Value {
    let parent_hash = latest_receipt_hash(root);
    let op_hash = deterministic_receipt_hash(&json!({
        "operation_type": operation_type,
        "metadata": metadata,
    }));
    let mut receipt = json!({
        "ok": true,
        "type": "verity_receipt",
        "plane": "verity",
        "ts": now_iso(),
        "operation_type": operation_type,
        "operation_hash": op_hash,
        "parent_verity_hash": parent_hash,
        "fidelity_score": round4(clamp01(fidelity_score)),
        "drift_delta": round4(drift_delta),
        "mode": cfg.mode,
        "vector_alignment": round4(clamp01(vector_alignment)),
        "vector_state": vector_classification(vector_alignment),
        "known_invariants": known_invariants,
        "truth_tags": truth_tags,
        "ultimate_vector": build_ultimate_vector_payload(),
        "metadata": metadata
    });
    let receipt_hash = deterministic_receipt_hash(&receipt);
    receipt["receipt_hash"] = Value::String(receipt_hash);
    append_jsonl(&verity_receipts_path(root), &receipt);
    write_json(&verity_latest_path(root), &receipt);
    write_json(
        &verity_vector_state_path(root),
        &json!({
            "ok": true,
            "ts": receipt.get("ts").and_then(Value::as_str).unwrap_or_default(),
            "vector_alignment": receipt.get("vector_alignment").and_then(Value::as_f64).unwrap_or(1.0),
            "vector_state": receipt.get("vector_state").and_then(Value::as_str).unwrap_or("Strong"),
            "ultimate_vector": build_ultimate_vector_payload(),
            "receipt_hash": receipt.get("receipt_hash").cloned().unwrap_or(Value::Null),
        }),
    );
    receipt
}

fn build_verity_event(root: &Path, event_type: &str, payload: Value) -> Value {
    let mut event = json!({
        "ok": true,
        "type": "verity_event",
        "event_type": event_type,
        "ts": now_iso(),
        "payload": payload,
    });
    event["event_hash"] = Value::String(deterministic_receipt_hash(&event));
    append_jsonl(&verity_events_path(root), &event);
    event
}

fn parse_tag_list(raw: Option<String>) -> Vec<String> {
    raw.map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

pub fn record_cli_domain_receipt(root: &Path, domain: &str, argv: &[String]) {
    if domain.trim().is_empty() {
        return;
    }
    let (cfg, signature_valid, _) = load_verity_plane_config(root);
    let baseline_fidelity = latest_fidelity_score(root);
    let baseline_vector = latest_vector_alignment(root);
    let normalized_domain = domain.trim().to_ascii_lowercase();
    let mut fidelity_score: f64 = if matches!(
        normalized_domain.as_str(),
        "security-plane" | "t0-invariants-kernel" | "verity-plane"
    ) {
        1.0
    } else if normalized_domain.contains("legacy") {
        0.92
    } else {
        0.995
    };
    let mut vector_alignment: f64 = if matches!(
        normalized_domain.as_str(),
        "metakernel" | "substrate-plane" | "security-plane" | "verity-plane"
    ) {
        0.99
    } else {
        0.965
    };
    if cfg.mode == VERITY_MODE_SIMULATION {
        fidelity_score = fidelity_score.max(0.90);
        vector_alignment = vector_alignment.max(0.90);
    }
    let drift_delta = round4(fidelity_score - baseline_fidelity);
    let mut truth_tags = vec!["consistent".to_string()];
    if vector_alignment >= cfg.vector_warning_threshold {
        truth_tags.push("vector_aligned".to_string());
    } else {
        truth_tags.push("vector_drifting".to_string());
    }
    if !signature_valid {
        truth_tags.push("policy_signature_recovered".to_string());
    }
    if (vector_alignment - baseline_vector).abs() > 0.01 {
        truth_tags.push("vector_shift".to_string());
    }
    let metadata = json!({
        "domain": normalized_domain,
        "argv_len": argv.len(),
        "argv_preview": argv.iter().take(6).cloned().collect::<Vec<String>>(),
    });
    let receipt = build_verity_receipt(
        root,
        &cfg,
        "cli_domain_invocation",
        fidelity_score,
        vector_alignment,
        drift_delta,
        vec![
            "no_undetectable_falsehood".to_string(),
            "state_change_explainable_from_parent_chain".to_string(),
            "vector_integrity".to_string(),
        ],
        truth_tags,
        metadata,
    );
    if fidelity_score < cfg.fidelity_lock_threshold {
        let _ = build_verity_event(
            root,
            "judicial_lock_triggered",
            json!({
                "reason": "fidelity_score_below_lock_threshold",
                "fidelity_score": fidelity_score,
                "lock_threshold": cfg.fidelity_lock_threshold,
                "receipt_hash": receipt.get("receipt_hash").cloned().unwrap_or(Value::Null),
            }),
        );
    }
}

fn status_payload(root: &Path, argv: &[String]) -> Value {
    let limit = parse_usize(parse_flag(argv, "limit"), 10, 1, 100);
    let (cfg, signature_valid, cfg_path) = load_verity_plane_config(root);
    let receipts = load_recent_jsonl(&verity_receipts_path(root), limit);
    let events = load_recent_jsonl(&verity_events_path(root), limit);
    let latest = read_json(&verity_latest_path(root)).unwrap_or_else(|| json!({}));
    let fidelity_score = latest
        .get("fidelity_score")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let vector_alignment = latest
        .get("vector_alignment")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let mode = verification_mode(fidelity_score, &cfg);
    let mut out = json!({
        "ok": true,
        "type": "verity_plane_status",
        "ultimate_vector": build_ultimate_vector_payload(),
        "policy": {
            "mode": cfg.mode,
            "fidelity_warning_threshold": cfg.fidelity_warning_threshold,
            "fidelity_lock_threshold": cfg.fidelity_lock_threshold,
            "vector_warning_threshold": cfg.vector_warning_threshold,
            "signature_valid": signature_valid,
            "config_path": cfg_path.to_string_lossy().to_string(),
        },
        "current": {
            "fidelity_score": round4(fidelity_score),
            "vector_alignment": round4(vector_alignment),
            "vector_state": vector_classification(vector_alignment),
        },
        "verification_mode": mode,
        "paths": {
            "receipts_path": verity_receipts_path(root).to_string_lossy().to_string(),
            "events_path": verity_events_path(root).to_string_lossy().to_string(),
            "latest_path": verity_latest_path(root).to_string_lossy().to_string(),
            "vector_state_path": verity_vector_state_path(root).to_string_lossy().to_string(),
        },
        "recent_limit": limit,
        "recent_receipts": receipts,
        "recent_events": events,
        "drift": load_verity_drift_snapshot(root, limit),
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn drift_status_payload(root: &Path, argv: &[String]) -> Value {
    let limit = parse_usize(parse_flag(argv, "limit"), 10, 1, 100);
    let mut out = json!({
        "ok": true,
        "type": "verity_drift_status",
        "ultimate_vector": build_ultimate_vector_payload(),
        "drift": load_verity_drift_snapshot(root, limit),
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn run_record_event(root: &Path, argv: &[String]) -> Value {
    let operation = parse_flag(argv, "operation")
        .unwrap_or_else(|| "manual_verity_event".to_string())
        .to_ascii_lowercase();
    let (cfg, _, _) = load_verity_plane_config(root);
    let baseline_fidelity = latest_fidelity_score(root);
    let baseline_vector = latest_vector_alignment(root);
    let fidelity_score = parse_f64(parse_flag(argv, "fidelity"), baseline_fidelity, 0.0, 1.0);
    let vector_alignment = parse_f64(parse_flag(argv, "vector"), baseline_vector, 0.0, 1.0);
    let drift_delta = parse_f64(
        parse_flag(argv, "drift-delta"),
        fidelity_score - baseline_fidelity,
        -1.0,
        1.0,
    );
    let mut truth_tags = parse_tag_list(parse_flag(argv, "tags"));
    if truth_tags.is_empty() {
        truth_tags.push("consistent".to_string());
    }
    let metadata = parse_flag(argv, "metadata-json")
        .as_deref()
        .and_then(parse_json)
        .unwrap_or_else(|| {
            json!({
                "source": "cli_record_event",
                "argv": argv
            })
        });
    let receipt = build_verity_receipt(
        root,
        &cfg,
        operation.as_str(),
        fidelity_score,
        vector_alignment,
        drift_delta,
        vec![
            "no_undetectable_falsehood".to_string(),
            "drift_measured".to_string(),
        ],
        truth_tags,
        metadata,
    );
    let mode = verification_mode(fidelity_score, &cfg);
    if mode == "judicial_lock" {
        let _ = build_verity_event(
            root,
            "judicial_lock_triggered",
            json!({
                "reason": "record_event_fidelity_below_lock_threshold",
                "fidelity_score": fidelity_score,
                "lock_threshold": cfg.fidelity_lock_threshold,
                "receipt_hash": receipt.get("receipt_hash").cloned().unwrap_or(Value::Null),
            }),
        );
    }
    let mut out = json!({
        "ok": true,
        "type": "verity_record_event",
        "verification_mode": mode,
        "receipt": receipt,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn run_vector_check(root: &Path, argv: &[String]) -> Value {
    let (cfg, _, _) = load_verity_plane_config(root);
    let target = parse_f64(parse_flag(argv, "target"), 1.0, 0.0, 1.0);
    let fidelity_score = latest_fidelity_score(root);
    let vector_alignment = latest_vector_alignment(root);
    let delta_to_target = round4(target - vector_alignment);
    let classification = vector_classification(vector_alignment);
    let event = build_verity_event(
        root,
        "vector_check",
        json!({
            "target_alignment": target,
            "current_alignment": round4(vector_alignment),
            "delta_to_target": delta_to_target,
            "classification": classification,
            "mode": cfg.mode,
        }),
    );
    let receipt = build_verity_receipt(
        root,
        &cfg,
        "vector_check",
        fidelity_score,
        vector_alignment,
        delta_to_target,
        vec![
            "vector_integrity".to_string(),
            "trajectory_alignment_evaluated".to_string(),
        ],
        vec![
            if vector_alignment >= cfg.vector_warning_threshold {
                "vector_aligned"
            } else {
                "vector_drift_detected"
            }
            .to_string(),
        ],
        json!({
            "target_alignment": target,
            "classification": classification,
            "event_hash": event.get("event_hash").cloned().unwrap_or(Value::Null),
        }),
    );
    let mut out = json!({
        "ok": true,
        "type": "verity_vector_check",
        "target_alignment": target,
        "current_alignment": round4(vector_alignment),
        "delta_to_target": delta_to_target,
        "classification": classification,
        "verification_mode": verification_mode(fidelity_score, &cfg),
        "event": event,
        "receipt": receipt,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn run_refine_event(root: &Path, argv: &[String]) -> Value {
    let falsehood = parse_flag(argv, "falsehood")
        .unwrap_or_else(|| "unspecified_falsehood".to_string())
        .trim()
        .to_string();
    let refined = parse_flag(argv, "refined")
        .unwrap_or_else(|| "unspecified_refinement".to_string())
        .trim()
        .to_string();
    let (cfg, _, _) = load_verity_plane_config(root);
    let before_fidelity = parse_f64(
        parse_flag(argv, "before-fidelity"),
        latest_fidelity_score(root),
        0.0,
        1.0,
    );
    let after_fidelity = parse_f64(
        parse_flag(argv, "after-fidelity"),
        (before_fidelity + 0.03).min(1.0),
        0.0,
        1.0,
    );
    let delta = round4(after_fidelity - before_fidelity);
    let event = build_verity_event(
        root,
        "refinement_event",
        json!({
            "falsehood_discovered": falsehood,
            "refined_state": refined,
            "before_fidelity_score": round4(before_fidelity),
            "after_fidelity_score": round4(after_fidelity),
            "fidelity_delta": delta,
        }),
    );
    let receipt = build_verity_receipt(
        root,
        &cfg,
        "refinement_event",
        after_fidelity,
        latest_vector_alignment(root),
        delta,
        vec![
            "no_undetectable_falsehood".to_string(),
            "drift_measured".to_string(),
            "self_deception_flagged".to_string(),
        ],
        vec!["refined".to_string(), "drift_detected".to_string()],
        json!({
            "falsehood_discovered": falsehood,
            "refined_state": refined,
            "event_hash": event.get("event_hash").cloned().unwrap_or(Value::Null),
        }),
    );
    let mut out = json!({
        "ok": true,
        "type": "verity_refinement_event",
        "event": event,
        "receipt": receipt,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}
