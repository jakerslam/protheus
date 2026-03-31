pub fn compute_extract_failure_cluster_principle(
    input: &ExtractFailureClusterPrincipleInput,
) -> ExtractFailureClusterPrincipleOutput {
    let policy = input.policy.clone().unwrap_or_else(|| json!({}));
    if !to_bool_like(
        value_path(Some(&policy), &["first_principles", "enabled"]),
        false,
    ) {
        return ExtractFailureClusterPrincipleOutput { principle: None };
    }
    if !to_bool_like(
        value_path(
            Some(&policy),
            &["first_principles", "allow_failure_cluster_extraction"],
        ),
        false,
    ) {
        return ExtractFailureClusterPrincipleOutput { principle: None };
    }
    let session = input
        .session
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let signature_tokens = {
        let from_session = session
            .get("signature_tokens")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|v| value_to_string(Some(v)))
            .collect::<Vec<_>>();
        if from_session.is_empty() {
            compute_tokenize_text(&TokenizeTextInput {
                value: Some({
                    let sig = value_to_string(session.get("signature"));
                    if sig.is_empty() {
                        value_to_string(session.get("objective"))
                    } else {
                        sig
                    }
                }),
                max_tokens: Some(64),
            })
            .tokens
        } else {
            from_session
        }
    };
    let query = json!({
        "signature_tokens": signature_tokens,
        "trit_vector": [-1],
        "target": compute_normalize_target(&NormalizeTargetInput {
            value: Some(value_to_string(session.get("target")))
        }).value
    });
    let candidates = compute_select_library_candidates(&SelectLibraryCandidatesInput {
        file_path: input
            .paths
            .as_ref()
            .and_then(|v| v.as_object())
            .and_then(|m| m.get("library_path"))
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        policy: Some(policy.clone()),
        query: Some(query),
    })
    .candidates
    .into_iter()
    .filter(|entry| {
        normalize_trit_value(
            value_path(Some(entry), &["row", "outcome_trit"]).unwrap_or(&Value::Null),
        ) < 0
    })
    .collect::<Vec<_>>();
    let cluster_min = js_number_for_extract(value_path(
        Some(&policy),
        &["first_principles", "failure_cluster_min"],
    ))
    .unwrap_or(4.0) as usize;
    if candidates.len() < cluster_min {
        return ExtractFailureClusterPrincipleOutput { principle: None };
    }
    let avg_similarity = {
        let total = candidates
            .iter()
            .map(|row| js_number_for_extract(value_path(Some(row), &["similarity"])).unwrap_or(0.0))
            .sum::<f64>();
        total / (candidates.len() as f64).max(1.0)
    };
    let confidence = clamp_number(
        (((candidates.len() as f64) / ((cluster_min + 3) as f64)).min(1.0) * 0.6)
            + (avg_similarity * 0.4),
        0.0,
        1.0,
    );
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let signature_or_objective = {
        let sig = value_to_string(session.get("signature"));
        if sig.is_empty() {
            value_to_string(session.get("objective"))
        } else {
            sig
        }
    };
    let id_seed = format!(
        "{}|failure_cluster|{}",
        value_to_string(session.get("session_id")),
        signature_or_objective
    );
    let objective = clean_text_runtime(&value_to_string(session.get("objective")), 240);
    let filter_stack = compute_normalize_list(&NormalizeListInput {
        value: Some(
            session
                .get("filter_stack")
                .cloned()
                .unwrap_or(Value::Array(vec![])),
        ),
        max_len: Some(120),
    })
    .items
    .join(", ");
    let objective_id_value = {
        let v = clean_text_runtime(&value_to_string(session.get("objective_id")), 140);
        if v.is_empty() {
            Value::Null
        } else {
            Value::String(v)
        }
    };
    let objective_for_statement = objective.clone();
    let statement = clean_text_runtime(
        &format!(
            "Avoid repeating inversion filter stack ({}) for objective \"{}\" without introducing a materially different paradigm shift.",
            if filter_stack.is_empty() { "none".to_string() } else { filter_stack },
            if objective_for_statement.is_empty() { "unknown".to_string() } else { objective_for_statement }
        ),
        360
    );
    let principle = json!({
        "id": stable_id_runtime(&id_seed, "ifp"),
        "ts": now_iso,
        "source": "inversion_controller_failure_cluster",
        "objective": objective,
        "objective_id": objective_id_value,
        "statement": statement,
        "target": compute_normalize_target(&NormalizeTargetInput {
            value: Some(value_to_string(session.get("target")))
        }).value,
        "confidence": (confidence * 1_000_000.0).round() / 1_000_000.0,
        "polarity": -1,
        "failure_cluster_count": candidates.len(),
        "strategy_feedback": {
            "enabled": true,
            "suggested_bonus": 0
        },
        "session_id": clean_text_runtime(&value_to_string(session.get("session_id")), 80)
    });
    ExtractFailureClusterPrincipleOutput {
        principle: Some(principle),
    }
}

pub fn compute_persist_first_principle(
    input: &PersistFirstPrincipleInput,
) -> PersistFirstPrincipleOutput {
    let paths = input
        .paths
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let principle = input.principle.clone().unwrap_or_else(|| json!({}));
    let _ = compute_write_json_atomic(&WriteJsonAtomicInput {
        file_path: paths
            .get("first_principles_latest_path")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        value: Some(principle.clone()),
    });
    let _ = compute_append_jsonl(&AppendJsonlInput {
        file_path: paths
            .get("first_principles_history_path")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        row: Some(principle.clone()),
    });
    let _ = compute_upsert_first_principle_lock(&UpsertFirstPrincipleLockInput {
        file_path: paths
            .get("first_principles_lock_path")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        session: input.session.clone(),
        principle: Some(principle.clone()),
        now_iso: input.now_iso.clone(),
    });
    PersistFirstPrincipleOutput { principle }
}

pub fn compute_creative_penalty(input: &CreativePenaltyInput) -> CreativePenaltyOutput {
    let preferred = input
        .preferred_creative_lane_ids
        .iter()
        .map(|row| normalize_token_runtime(row, 120))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let selected_lane = input
        .selected_lane
        .as_deref()
        .map(|v| v.to_string())
        .filter(|v| !v.is_empty());
    if !input.enabled.unwrap_or(false) {
        return CreativePenaltyOutput {
            creative_lane_preferred: false,
            selected_lane,
            preferred_lanes: preferred,
            penalty: 0.0,
            applied: false,
        };
    }
    let Some(selected) = selected_lane.clone() else {
        return CreativePenaltyOutput {
            creative_lane_preferred: false,
            selected_lane: None,
            preferred_lanes: preferred,
            penalty: 0.0,
            applied: false,
        };
    };
    let is_preferred = preferred.iter().any(|row| row == &selected);
    let penalty = if is_preferred {
        0.0
    } else {
        input.non_creative_certainty_penalty.unwrap_or(0.0)
    };
    let penalty = clamp_number(penalty, 0.0, 0.5);
    let penalty = (penalty * 1_000_000.0).round() / 1_000_000.0;
    CreativePenaltyOutput {
        creative_lane_preferred: is_preferred,
        selected_lane: Some(selected),
        preferred_lanes: preferred,
        penalty,
        applied: penalty > 0.0,
    }
}

pub fn compute_extract_bullets(input: &ExtractBulletsInput) -> ExtractBulletsOutput {
    let max_items = input.max_items.unwrap_or(4).max(0) as usize;
    let markdown = input.markdown.as_deref().unwrap_or("");
    let mut out = Vec::new();
    let bullet_re = Regex::new(r"^[-*]\s+(.+)$").expect("valid bullet regex");
    let ordered_re = Regex::new(r"^\d+\.\s+(.+)$").expect("valid ordered regex");
    for line in markdown.lines() {
        let trimmed = line.trim();
        let capture = bullet_re
            .captures(trimmed)
            .or_else(|| ordered_re.captures(trimmed));
        let Some(cap) = capture else {
            continue;
        };
        let item = clean_text_runtime(cap.get(1).map(|m| m.as_str()).unwrap_or(""), 220);
        if item.is_empty() {
            continue;
        }
        out.push(item);
        if out.len() >= max_items {
            break;
        }
    }
    ExtractBulletsOutput { items: out }
}

pub fn compute_extract_list_items(input: &ExtractListItemsInput) -> ExtractListItemsOutput {
    let max_items = input.max_items.unwrap_or(8).max(0) as usize;
    let markdown = input.markdown.as_deref().unwrap_or("");
    let mut out = Vec::new();
    let bullet_re = Regex::new(r"^[-*]\s+(.+)$").expect("valid list regex");
    for line in markdown.lines() {
        let trimmed = line.trim();
        let Some(cap) = bullet_re.captures(trimmed) else {
            continue;
        };
        let item = clean_text_runtime(cap.get(1).map(|m| m.as_str()).unwrap_or(""), 160);
        if item.is_empty() {
            continue;
        }
        out.push(item);
        if out.len() >= max_items {
            break;
        }
    }
    ExtractListItemsOutput { items: out }
}

pub fn compute_parse_system_internal_permission(
    input: &ParseSystemInternalPermissionInput,
) -> ParseSystemInternalPermissionOutput {
    let markdown = input.markdown.as_deref().unwrap_or("");
    let permission_re = Regex::new(
        r"(?i)^-+\s*system_internal\s*:\s*\{\s*enabled:\s*(true|false)\s*,\s*sources:\s*\[([^\]]*)\]\s*\}\s*$",
    )
    .expect("valid system_internal regex");
    for line in markdown.lines() {
        let trimmed = line.trim();
        let Some(cap) = permission_re.captures(trimmed) else {
            continue;
        };
        let enabled = cap
            .get(1)
            .map(|m| m.as_str().to_lowercase() == "true")
            .unwrap_or(false);
        let sources = cap
            .get(2)
            .map(|m| {
                m.as_str()
                    .split(',')
                    .map(|row| normalize_token_runtime(row, 40))
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        return ParseSystemInternalPermissionOutput { enabled, sources };
    }
    ParseSystemInternalPermissionOutput {
        enabled: false,
        sources: Vec::new(),
    }
}

pub fn compute_parse_soul_token_data_pass_rules(
    input: &ParseSoulTokenDataPassRulesInput,
) -> ParseSoulTokenDataPassRulesOutput {
    let markdown = input.markdown.as_deref().unwrap_or("");
    let section = markdown.split("## Data Pass Rules").nth(1).unwrap_or("");
    let list = compute_extract_list_items(&ExtractListItemsInput {
        markdown: Some(section.to_string()),
        max_items: Some(12),
    });
    let rules = list
        .items
        .iter()
        .map(|row| normalize_token_runtime(row, 80))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    ParseSoulTokenDataPassRulesOutput { rules }
}

pub fn compute_ensure_system_passed_section(
    input: &EnsureSystemPassedSectionInput,
) -> EnsureSystemPassedSectionOutput {
    let body = input
        .feed_text
        .as_deref()
        .unwrap_or("")
        .trim_end_matches(|c: char| c.is_whitespace())
        .to_string();
    if body.contains("\n## System Passed") {
        return EnsureSystemPassedSectionOutput { text: body };
    }
    let text = [
        body,
        String::new(),
        "## System Passed".to_string(),
        String::new(),
        "Hash-verified system payloads pushed from internal sources (memory, loops, analytics)."
            .to_string(),
        "Entries are JSON payload records with deterministic hash verification.".to_string(),
        String::new(),
    ]
    .join("\n");
    EnsureSystemPassedSectionOutput { text }
}

pub fn compute_system_passed_payload_hash(
    input: &SystemPassedPayloadHashInput,
) -> SystemPassedPayloadHashOutput {
    let source = normalize_token_runtime(input.source.as_deref().unwrap_or(""), 80);
    let tags = input.tags.join(",");
    let payload = clean_text_runtime(input.payload.as_deref().unwrap_or(""), 2000);
    let seed = format!("v1|{source}|{tags}|{payload}");
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hash = hex::encode(hasher.finalize());
    SystemPassedPayloadHashOutput { hash }
}

pub fn compute_build_lens_position(input: &BuildLensPositionInput) -> BuildLensPositionOutput {
    let objective = input.objective.as_deref().unwrap_or("");
    let lower = objective.to_lowercase();
    let target = input.target.as_deref().unwrap_or("");
    let impact = input.impact.as_deref().unwrap_or("");
    let position = if lower.contains("memory") && lower.contains("security") {
        "Preserve memory determinism sequencing while keeping security fail-closed at dispatch boundaries.".to_string()
    } else if lower.contains("drift") {
        "Treat drift above tolerance as a hard stop and require rollback-ready proof before apply."
            .to_string()
    } else if target == "identity" || impact == "high" || impact == "critical" {
        "Use strict reversible slices with explicit receipts before any live apply.".to_string()
    } else {
        "Keep the smallest reversible path and preserve fail-closed controls before mutation."
            .to_string()
    };
    BuildLensPositionOutput { position }
}

pub fn compute_build_conclave_proposal_summary(
    input: &BuildConclaveProposalSummaryInput,
) -> BuildConclaveProposalSummaryOutput {
    let mut parts = Vec::new();
    for (value, max_len) in [
        (input.objective.as_deref().unwrap_or(""), 320usize),
        (input.objective_id.as_deref().unwrap_or(""), 120usize),
        (input.target.as_deref().unwrap_or(""), 40usize),
        (input.impact.as_deref().unwrap_or(""), 40usize),
        (input.mode.as_deref().unwrap_or(""), 24usize),
    ] {
        let clean = clean_text_runtime(value, max_len);
        if !clean.is_empty() {
            parts.push(clean);
        }
    }
    let summary = if parts.is_empty() {
        "inversion_self_modification_request".to_string()
    } else {
        parts.join(" | ")
    };
    BuildConclaveProposalSummaryOutput { summary }
}
