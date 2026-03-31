pub fn compute_capability_cooldown_key(
    input: &CapabilityCooldownKeyInput,
) -> CapabilityCooldownKeyOutput {
    let raw = input
        .capability_key
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if raw.is_empty() {
        return CapabilityCooldownKeyOutput {
            cooldown_key: String::new(),
        };
    }
    let normalized = Regex::new(r"[^a-z0-9:_-]")
        .expect("valid capability cooldown key regex")
        .replace_all(&raw, "_")
        .to_string();
    CapabilityCooldownKeyOutput {
        cooldown_key: format!("capability:{normalized}"),
    }
}

pub fn compute_readiness_retry_cooldown_key(
    input: &ReadinessRetryCooldownKeyInput,
) -> ReadinessRetryCooldownKeyOutput {
    let sid = normalize_spaces(input.strategy_id.as_deref().unwrap_or("")).to_ascii_lowercase();
    let sid = Regex::new(r"[^a-z0-9:_-]")
        .expect("valid readiness strategy regex")
        .replace_all(&sid, "_")
        .to_string();
    if sid.is_empty() {
        return ReadinessRetryCooldownKeyOutput {
            cooldown_key: String::new(),
        };
    }
    let mode = normalize_spaces(input.execution_mode.as_deref().unwrap_or("")).to_ascii_lowercase();
    let mode = Regex::new(r"[^a-z0-9:_-]")
        .expect("valid readiness mode regex")
        .replace_all(&mode, "_")
        .to_string();
    if mode.is_empty() {
        return ReadinessRetryCooldownKeyOutput {
            cooldown_key: format!("readiness:strategy:{sid}"),
        };
    }
    ReadinessRetryCooldownKeyOutput {
        cooldown_key: format!("readiness:strategy:{sid}:mode:{mode}"),
    }
}

pub fn compute_source_eye_id(input: &SourceEyeIdInput) -> SourceEyeIdOutput {
    let eye_ref = input.eye_ref.as_deref().unwrap_or("").trim();
    let eye_id = eye_ref.strip_prefix("eye:").unwrap_or(eye_ref).to_string();
    SourceEyeIdOutput { eye_id }
}

pub fn compute_deprioritized_source_proposal(
    input: &DeprioritizedSourceProposalInput,
) -> DeprioritizedSourceProposalOutput {
    let eye_id = input
        .eye_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if eye_id.is_empty() {
        return DeprioritizedSourceProposalOutput {
            deprioritized: false,
        };
    }
    let deprioritized = input
        .deprioritized_eye_ids
        .iter()
        .any(|row| row.trim().eq_ignore_ascii_case(&eye_id));
    DeprioritizedSourceProposalOutput { deprioritized }
}

pub fn compute_composite_eligibility_min(
    input: &CompositeEligibilityMinInput,
) -> CompositeEligibilityMinOutput {
    let normalized = normalize_risk_level(input.risk.as_deref().unwrap_or(""));
    let base_min = input.base_min;
    if normalized != "low" || input.execution_mode.as_deref().unwrap_or("") != "canary_execute" {
        return CompositeEligibilityMinOutput {
            min_score: base_min,
        };
    }
    let relax = input.canary_low_risk_relax.max(0.0);
    CompositeEligibilityMinOutput {
        min_score: (base_min - relax).max(55.0),
    }
}

pub fn compute_clamp_threshold(input: &ClampThresholdInput) -> ClampThresholdOutput {
    let name = input.name.as_deref().unwrap_or("").trim();
    let (lo, hi) = match name {
        "min_signal_quality" => (40.0, 90.0),
        "min_sensory_signal_score" => (35.0, 85.0),
        "min_sensory_relevance_score" => (35.0, 85.0),
        "min_directive_fit" => (25.0, 90.0),
        "min_actionability_score" => (30.0, 90.0),
        "min_eye_score_ema" => (30.0, 90.0),
        _ => (0.0, 100.0),
    };
    let rounded = if input.value.is_finite() {
        input.value.round()
    } else {
        0.0
    };
    let threshold = rounded.max(lo).min(hi);
    ClampThresholdOutput { threshold }
}

pub fn compute_applied_thresholds(input: &AppliedThresholdsInput) -> AppliedThresholdsOutput {
    let mut out = std::collections::BTreeMap::new();
    for (key, base_val) in input.base.iter() {
        if !base_val.is_finite() {
            continue;
        }
        let delta = input.deltas.get(key).copied().unwrap_or(0.0);
        let clamped = compute_clamp_threshold(&ClampThresholdInput {
            name: Some(key.clone()),
            value: base_val + delta,
        })
        .threshold;
        out.insert(key.clone(), clamped);
    }
    AppliedThresholdsOutput { thresholds: out }
}

pub fn compute_extract_eye_from_evidence_ref(
    input: &ExtractEyeFromEvidenceRefInput,
) -> ExtractEyeFromEvidenceRefOutput {
    let text = input.reference.as_deref().unwrap_or("");
    let re = Regex::new(r"\beye:([^\s]+)").expect("valid eye ref regex");
    let eye_id = re
        .captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string());
    ExtractEyeFromEvidenceRefOutput { eye_id }
}

pub fn compute_total_outcomes(input: &TotalOutcomesInput) -> TotalOutcomesOutput {
    let total = input.shipped + input.no_change + input.reverted;
    TotalOutcomesOutput { total }
}

pub fn compute_derive_entity_bias(input: &DeriveEntityBiasInput) -> DeriveEntityBiasOutput {
    let total = compute_total_outcomes(&TotalOutcomesInput {
        shipped: input.shipped,
        no_change: input.no_change,
        reverted: input.reverted,
    })
    .total;
    if total < input.min_total {
        return DeriveEntityBiasOutput { bias: 0.0, total };
    }
    let shipped_rate = if total > 0.0 {
        input.shipped / total
    } else {
        0.0
    };
    let churn_rate = if total > 0.0 {
        (input.no_change + input.reverted) / total
    } else {
        0.0
    };
    let bias = if shipped_rate >= 0.6 {
        -3.0
    } else if shipped_rate >= 0.45 {
        -2.0
    } else if churn_rate >= 0.8 {
        5.0
    } else if churn_rate >= 0.65 {
        3.0
    } else if churn_rate >= 0.5 {
        1.0
    } else {
        0.0
    };
    DeriveEntityBiasOutput { bias, total }
}

pub fn compute_build_overlay(input: &BuildOverlayInput) -> BuildOverlayOutput {
    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::HashMap<String, BuildOverlayEntryOutput> =
        std::collections::HashMap::new();
    for event in &input.events {
        let proposal_id = normalize_spaces(event.proposal_id.as_deref().unwrap_or(""));
        if proposal_id.is_empty() {
            continue;
        }
        if !map.contains_key(&proposal_id) {
            order.push(proposal_id.clone());
            map.insert(
                proposal_id.clone(),
                BuildOverlayEntryOutput {
                    proposal_id: proposal_id.clone(),
                    decision: None,
                    decision_ts: None,
                    decision_reason: None,
                    last_outcome: None,
                    last_outcome_ts: None,
                    last_evidence_ref: None,
                    outcomes: BuildOverlayOutcomeCountsOutput {
                        shipped: 0,
                        reverted: 0,
                        no_change: 0,
                    },
                },
            );
        }
        let Some(cur) = map.get_mut(&proposal_id) else {
            continue;
        };
        let event_type = normalize_spaces(event.event_type.as_deref().unwrap_or(""));
        let ts = event
            .ts
            .as_deref()
            .map(normalize_spaces)
            .unwrap_or_default();
        if event_type == "decision" {
            let decision = normalize_spaces(event.decision.as_deref().unwrap_or(""));
            if !decision.is_empty() {
                let newer = cur
                    .decision_ts
                    .as_deref()
                    .map(|row| ts.as_str() >= row)
                    .unwrap_or(true);
                if newer {
                    cur.decision = Some(decision);
                    cur.decision_ts = if ts.is_empty() {
                        None
                    } else {
                        Some(ts.clone())
                    };
                    let reason = normalize_spaces(event.reason.as_deref().unwrap_or(""));
                    cur.decision_reason = if reason.is_empty() {
                        None
                    } else {
                        Some(reason)
                    };
                }
            }
        } else if event_type == "outcome" {
            let outcome = normalize_spaces(event.outcome.as_deref().unwrap_or(""));
            if outcome == "shipped" {
                cur.outcomes.shipped += 1;
            } else if outcome == "reverted" {
                cur.outcomes.reverted += 1;
            } else if outcome == "no_change" {
                cur.outcomes.no_change += 1;
            }
            if !outcome.is_empty() {
                let newer = cur
                    .last_outcome_ts
                    .as_deref()
                    .map(|row| ts.as_str() >= row)
                    .unwrap_or(true);
                if newer {
                    cur.last_outcome = Some(outcome);
                    cur.last_outcome_ts = if ts.is_empty() {
                        None
                    } else {
                        Some(ts.clone())
                    };
                    let evidence_ref =
                        normalize_spaces(event.evidence_ref.as_deref().unwrap_or(""));
                    cur.last_evidence_ref = if evidence_ref.is_empty() {
                        None
                    } else {
                        Some(evidence_ref)
                    };
                }
            }
        }
    }
    let entries = order
        .into_iter()
        .filter_map(|proposal_id| map.remove(&proposal_id))
        .collect();
    BuildOverlayOutput { entries }
}

pub fn compute_has_adaptive_mutation_signal(
    input: &HasAdaptiveMutationSignalInput,
) -> HasAdaptiveMutationSignalOutput {
    let type_re = Regex::new(
        r"(?i)\b(adaptive[_-]?mutation|mutation(?:[_-]proposal)?|topology[_-]?mutation|genome[_-]?mutation|self[_-]?(?:mutation|modify)|branch[_-]?(?:rewire|prune))\b",
    )
    .expect("valid adaptive mutation type regex");
    let signal_re = Regex::new(
        r"(?i)\b(mutation(?:[_-]?(?:guard|policy|kernel|budget|ttl|quarantine|veto|rollback|lineage|attestation))?|topology[_-]?mutation|genome[_-]?mutation|self[_-]?(?:mutation|modify)|branch[_-]?(?:rewire|prune))\b",
    )
    .expect("valid adaptive mutation signal regex");
    let proposal_type = normalize_spaces(input.proposal_type.as_deref().unwrap_or(""));
    if !proposal_type.is_empty() && type_re.is_match(&proposal_type) {
        return HasAdaptiveMutationSignalOutput { has_signal: true };
    }
    if input.adaptive_mutation
        || input.mutation_proposal
        || input.topology_mutation
        || input.self_improvement_change
    {
        return HasAdaptiveMutationSignalOutput { has_signal: true };
    }
    let blob = input
        .signal_blob
        .as_deref()
        .map(normalize_spaces)
        .unwrap_or_default();
    if blob.is_empty() {
        return HasAdaptiveMutationSignalOutput { has_signal: false };
    }
    HasAdaptiveMutationSignalOutput {
        has_signal: type_re.is_match(&blob) || signal_re.is_match(&blob),
    }
}

pub fn compute_adaptive_mutation_execution_guard(
    input: &AdaptiveMutationExecutionGuardInput,
) -> AdaptiveMutationExecutionGuardOutput {
    if !input.guard_required {
        return AdaptiveMutationExecutionGuardOutput {
            required: false,
            applies: false,
            pass: true,
            reason: None,
            reasons: Vec::new(),
            controls: AdaptiveMutationExecutionGuardControlsOutput {
                safety_attestation: None,
                rollback_receipt: None,
                guard_receipt_id: None,
                mutation_kernel_applies: false,
                mutation_kernel_pass: true,
            },
        };
    }
    if !input.applies {
        return AdaptiveMutationExecutionGuardOutput {
            required: true,
            applies: false,
            pass: true,
            reason: None,
            reasons: Vec::new(),
            controls: AdaptiveMutationExecutionGuardControlsOutput {
                safety_attestation: None,
                rollback_receipt: None,
                guard_receipt_id: None,
                mutation_kernel_applies: false,
                mutation_kernel_pass: true,
            },
        };
    }

    let safety_attestation = normalize_spaces(input.safety_attestation.as_deref().unwrap_or(""));
    let rollback_receipt = normalize_spaces(input.rollback_receipt.as_deref().unwrap_or(""));
    let guard_receipt_id = normalize_spaces(input.guard_receipt_id.as_deref().unwrap_or(""));
    let mut reasons: Vec<String> = Vec::new();
    if !input.metadata_applies {
        reasons.push("adaptive_mutation_guard_metadata_missing".to_string());
    }
    if !input.guard_pass {
        let reason = normalize_spaces(
            input
                .guard_reason
                .as_deref()
                .unwrap_or("adaptive_mutation_guard_failed"),
        );
        reasons.push(if reason.is_empty() {
            "adaptive_mutation_guard_failed".to_string()
        } else {
            reason
        });
    }
    if safety_attestation.is_empty() {
        reasons.push("adaptive_mutation_missing_safety_attestation".to_string());
    }
    if rollback_receipt.is_empty() {
        reasons.push("adaptive_mutation_missing_rollback_receipt".to_string());
    }
    if guard_receipt_id.is_empty() {
        reasons.push("adaptive_mutation_missing_execution_guard_receipt".to_string());
    }
    if input.mutation_kernel_applies && !input.mutation_kernel_pass {
        reasons.push("adaptive_mutation_kernel_failed".to_string());
    }

    let mut uniq_reasons: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for reason in reasons {
        if reason.is_empty() || !seen.insert(reason.clone()) {
            continue;
        }
        uniq_reasons.push(reason);
    }
    let reason = uniq_reasons.first().cloned();
    AdaptiveMutationExecutionGuardOutput {
        required: true,
        applies: true,
        pass: uniq_reasons.is_empty(),
        reason,
        reasons: uniq_reasons,
        controls: AdaptiveMutationExecutionGuardControlsOutput {
            safety_attestation: if safety_attestation.is_empty() {
                None
            } else {
                Some(safety_attestation)
            },
            rollback_receipt: if rollback_receipt.is_empty() {
                None
            } else {
                Some(rollback_receipt)
            },
            guard_receipt_id: if guard_receipt_id.is_empty() {
                None
            } else {
                Some(guard_receipt_id)
            },
            mutation_kernel_applies: input.mutation_kernel_applies,
            mutation_kernel_pass: input.mutation_kernel_pass,
        },
    }
}

fn stable_selection_index(seed: &str, size: usize) -> usize {
    if size == 0 {
        return 0;
    }
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hash = hasher.finalize();
    let mut first_12 = String::with_capacity(12);
    for byte in hash[..6].iter() {
        first_12.push_str(&format!("{byte:02x}"));
    }
    let n = u64::from_str_radix(&first_12, 16).unwrap_or(0);
    (n % size as u64) as usize
}
