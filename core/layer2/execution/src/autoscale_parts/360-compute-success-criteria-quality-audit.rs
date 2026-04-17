pub fn compute_success_criteria_quality_audit(
    input: &SuccessCriteriaQualityAuditInput,
) -> SuccessCriteriaQualityAuditOutput {
    let base = input
        .verification
        .clone()
        .filter(|value| value.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    let Some(base_obj) = base.as_object() else {
        return SuccessCriteriaQualityAuditOutput { verification: base };
    };
    let criteria = base_obj
        .get("success_criteria")
        .and_then(|value| value.as_object());
    if criteria.is_none() {
        let mut out = base_obj.clone();
        out.insert("criteria_quality".to_string(), serde_json::Value::Null);
        out.insert(
            "criteria_quality_insufficient".to_string(),
            serde_json::Value::Bool(false),
        );
        return SuccessCriteriaQualityAuditOutput {
            verification: serde_json::Value::Object(out),
        };
    }
    let criteria = criteria.expect("checked is_some");
    let checks = criteria
        .get("checks")
        .and_then(|value| value.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let obj = row.as_object()?;
                    let reason = [
                        obj.get("reason"),
                        obj.get("error"),
                        obj.get("message"),
                        obj.get("outcome"),
                    ]
                    .into_iter()
                    .flatten()
                    .map(js_like_string)
                    .map(|value| value.trim().to_string())
                    .find(|value| !value.is_empty());
                    let evaluated = obj
                        .get("evaluated")
                        .and_then(|value| value.as_bool())
                        .or_else(|| obj.get("pass").and_then(|value| value.as_bool()))
                        .or_else(|| {
                            obj.get("status")
                                .and_then(|value| value.as_str())
                                .map(|status| {
                                    matches!(
                                        status.trim().to_ascii_lowercase().as_str(),
                                        "pass" | "passed" | "ok" | "success" | "true"
                                    )
                                })
                        })
                        .unwrap_or(false);
                    let has_signal = obj.get("evaluated").is_some()
                        || obj.get("pass").is_some()
                        || obj.get("status").is_some()
                        || reason.is_some();
                    if !has_signal {
                        return None;
                    }
                    Some(AssessSuccessCriteriaQualityCheckInput { evaluated, reason })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let total_count = criteria
        .get("total_count")
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(checks.len() as f64);
    let unknown_count = criteria
        .get("unknown_count")
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(0.0)
        .min(total_count.max(0.0));
    let synthesized = criteria
        .get("synthesized")
        .and_then(|value| value.as_bool())
        .or_else(|| {
            criteria
                .get("synthesized")
                .and_then(|value| value.as_str())
                .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        })
        .unwrap_or(false);
    let quality = compute_assess_success_criteria_quality(&AssessSuccessCriteriaQualityInput {
        checks,
        total_count,
        unknown_count,
        synthesized,
    });
    let quality_json = serde_json::to_value(&quality).unwrap_or_else(|_| serde_json::json!({}));
    let mut out = base_obj.clone();
    out.insert("criteria_quality".to_string(), quality_json);
    out.insert(
        "criteria_quality_insufficient".to_string(),
        serde_json::Value::Bool(quality.insufficient),
    );
    SuccessCriteriaQualityAuditOutput {
        verification: serde_json::Value::Object(out),
    }
}

pub fn compute_detect_eyes_terminology_drift(
    input: &DetectEyesTerminologyDriftInput,
) -> DetectEyesTerminologyDriftOutput {
    let mut warnings = Vec::<DetectEyesTerminologyDriftWarning>::new();
    let mut seen = std::collections::BTreeSet::<String>::new();
    let eye_terms_re = Regex::new(r"\beye\b|\beyes\b").expect("valid eye regex");
    for proposal in &input.proposals {
        let proposal_obj = proposal.as_object();
        if proposal_obj.is_none() {
            continue;
        }
        let proposal_obj = proposal_obj.expect("checked is_some");
        let evidence = proposal_obj
            .get("evidence")
            .and_then(|value| value.as_array())
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| row.as_object())
                    .map(|row| ProposalTextBlobEvidenceEntryInput {
                        evidence_ref: row.get("evidence_ref").map(js_like_string),
                        path: row.get("path").map(js_like_string),
                        title: row.get("title").map(js_like_string),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let blob = compute_proposal_text_blob(&ProposalTextBlobInput {
            title: proposal_obj.get("title").map(js_like_string),
            summary: proposal_obj.get("summary").map(js_like_string),
            suggested_next_command: proposal_obj
                .get("suggested_next_command")
                .map(js_like_string),
            suggested_command: proposal_obj.get("suggested_command").map(js_like_string),
            notes: proposal_obj.get("notes").map(js_like_string),
            evidence,
        })
        .blob;
        if blob.is_empty() || !eye_terms_re.is_match(&blob) {
            continue;
        }
        let mut matched_tools = Vec::<String>::new();
        for token in &input.tool_capability_tokens {
            let mentioned = compute_tool_token_mentioned(&ToolTokenMentionedInput {
                blob: Some(blob.clone()),
                token: Some(token.clone()),
            });
            if mentioned.mentioned {
                matched_tools.push(token.clone());
            }
        }
        if matched_tools.is_empty() {
            continue;
        }
        let proposal_id = proposal_obj
            .get("id")
            .map(js_like_string)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let dedup_key = format!(
            "{}:{}",
            proposal_id.clone().unwrap_or_else(|| "unknown".to_string()),
            matched_tools.join(",")
        );
        if !seen.insert(dedup_key) {
            continue;
        }
        let sample = proposal_obj
            .get("title")
            .map(js_like_string)
            .unwrap_or_default();
        let sample = normalize_spaces(&sample);
        let sample = sample.chars().take(140).collect::<String>();
        warnings.push(DetectEyesTerminologyDriftWarning {
            proposal_id,
            reason: "tools_labeled_as_eyes".to_string(),
            matched_tools: matched_tools.into_iter().take(5).collect(),
            sample,
        });
        if warnings.len() >= 5 {
            break;
        }
    }
    DetectEyesTerminologyDriftOutput { warnings }
}

pub fn compute_normalize_stored_proposal_row(
    input: &NormalizeStoredProposalRowInput,
) -> NormalizeStoredProposalRowOutput {
    let Some(raw) = input.proposal.as_ref() else {
        return NormalizeStoredProposalRowOutput {
            proposal: serde_json::Value::Null,
        };
    };
    let Some(raw_obj) = raw.as_object() else {
        return NormalizeStoredProposalRowOutput {
            proposal: raw.clone(),
        };
    };
    let mut next = raw_obj.clone();
    let fallback = input
        .fallback
        .as_ref()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "pending".to_string());
    let normalized_status = compute_normalize_proposal_status(&NormalizeProposalStatusInput {
        raw_status: next.get("status").map(js_like_string),
        fallback: Some(fallback),
    })
    .normalized_status;
    next.insert(
        "status".to_string(),
        serde_json::Value::String(normalized_status),
    );
    let normalized_type = input
        .proposal_type
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "local_state_fallback".to_string());
    next.insert(
        "type".to_string(),
        serde_json::Value::String(normalized_type.clone()),
    );
    let mut meta = next
        .get("meta")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    meta.insert(
        "normalized_proposal_type".to_string(),
        serde_json::Value::String(normalized_type),
    );
    meta.insert(
        "proposal_type_source".to_string(),
        serde_json::Value::String(
            input
                .proposal_type_source
                .as_ref()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ),
    );
    meta.insert(
        "proposal_type_inferred".to_string(),
        serde_json::Value::Bool(input.proposal_type_inferred.unwrap_or(false)),
    );
    next.insert("meta".to_string(), serde_json::Value::Object(meta));
    NormalizeStoredProposalRowOutput {
        proposal: serde_json::Value::Object(next),
    }
}

pub fn compute_recent_proposal_key_counts(
    input: &RecentProposalKeyCountsInput,
) -> RecentProposalKeyCountsOutput {
    let cutoff_ms = input
        .cutoff_ms
        .filter(|value| value.is_finite())
        .unwrap_or(0.0);
    let mut counts = std::collections::BTreeMap::<String, f64>::new();
    for evt in &input.events {
        let key = evt
            .proposal_key
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let Some(key) = key else {
            continue;
        };
        let ts_ms = evt.ts_ms.unwrap_or(f64::NAN);
        if !ts_ms.is_finite() || ts_ms < cutoff_ms {
            continue;
        }
        let result = evt
            .result
            .as_ref()
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        if result != "executed"
            && result != "score_only_preview"
            && result != "stop_repeat_gate_circuit_breaker"
            && !evt.is_attempt
        {
            continue;
        }
        let next = counts.get(&key).copied().unwrap_or(0.0) + 1.0;
        counts.insert(key, next);
    }
    RecentProposalKeyCountsOutput { counts }
}

pub fn compute_capability_attempt_count_for_date(
    input: &CapabilityAttemptCountForDateInput,
) -> CapabilityAttemptCountForDateOutput {
    let keys = input
        .keys
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::BTreeSet<_>>();
    if keys.is_empty() {
        return CapabilityAttemptCountForDateOutput { count: 0.0 };
    }
    let mut count = 0.0;
    for evt in &input.events {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        if event_type != "autonomy_run" || !evt.is_attempt {
            continue;
        }
        let key = evt
            .capability_key
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if key.is_empty() {
            continue;
        }
        if keys.contains(&key) {
            count += 1.0;
        }
    }
    CapabilityAttemptCountForDateOutput { count }
}

pub fn compute_capability_outcome_stats_in_window(
    input: &CapabilityOutcomeStatsInWindowInput,
) -> CapabilityOutcomeStatsInWindowOutput {
    let keys = input
        .keys
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::BTreeSet<_>>();
    let mut out = CapabilityOutcomeStatsInWindowOutput {
        executed: 0.0,
        shipped: 0.0,
        no_change: 0.0,
        reverted: 0.0,
    };
    if keys.is_empty() {
        return out;
    }
    for evt in &input.events {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        let result = evt
            .result
            .as_ref()
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        if event_type != "autonomy_run" || result != "executed" {
            continue;
        }
        let key = evt
            .capability_key
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if key.is_empty() || !keys.contains(&key) {
            continue;
        }
        out.executed += 1.0;
        let outcome = evt
            .outcome
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if outcome == "shipped" {
            out.shipped += 1.0;
        } else if outcome == "no_change" {
            out.no_change += 1.0;
        } else if outcome == "reverted" {
            out.reverted += 1.0;
        }
    }
    out
}

pub fn compute_execute_confidence_history(
    input: &ExecuteConfidenceHistoryInput,
) -> ExecuteConfidenceHistoryOutput {
    let mut out = ExecuteConfidenceHistoryOutput {
        window_days: input.window_days,
        proposal_type: input
            .proposal_type
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty()),
        capability_key: input
            .capability_key
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty()),
        matched_events: 0.0,
        confidence_fallback: 0.0,
        route_blocked: 0.0,
        executed: 0.0,
        shipped: 0.0,
        no_change: 0.0,
        reverted: 0.0,
        no_change_rate: 0.0,
        reverted_rate: 0.0,
    };
    for evt in &input.events {
        if !evt.matched {
            continue;
        }
        out.matched_events += 1.0;
        let result = evt
            .result
            .as_ref()
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        if result == "score_only_fallback_low_execution_confidence" {
            out.confidence_fallback += 1.0;
            continue;
        }
        if result == "score_only_fallback_route_block" || result == "init_gate_blocked_route" {
            out.route_blocked += 1.0;
            continue;
        }
        if result != "executed" {
            continue;
        }
        out.executed += 1.0;
        let outcome = evt
            .outcome
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if outcome == "shipped" {
            out.shipped += 1.0;
        } else if outcome == "no_change" {
            out.no_change += 1.0;
        } else if outcome == "reverted" {
            out.reverted += 1.0;
        }
    }
    if out.executed > 0.0 {
        out.no_change_rate = ((out.no_change / out.executed) * 1000.0).round() / 1000.0;
        out.reverted_rate = ((out.reverted / out.executed) * 1000.0).round() / 1000.0;
    }
    out
}
