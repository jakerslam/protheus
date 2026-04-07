pub fn compute_trim_library(input: &TrimLibraryInput) -> TrimLibraryOutput {
    let rows = compute_read_jsonl(&ReadJsonlInput {
        file_path: input.file_path.clone(),
    })
    .rows
    .into_iter()
    .map(|row| {
        let mut normalized =
            compute_normalize_library_row(&NormalizeLibraryRowInput { row: Some(row) }).row;
        if let Some(obj) = normalized.as_object_mut() {
            let maturity_band = value_to_string(obj.get("maturity_band"));
            if maturity_band.is_empty() {
                obj.insert(
                    "maturity_band".to_string(),
                    Value::String("novice".to_string()),
                );
            }
        }
        normalized
    })
    .collect::<Vec<_>>();
    let cap = parse_number_like(input.max_entries.as_ref())
        .unwrap_or(4000.0)
        .floor() as i64;
    let cap = cap.max(100) as usize;
    if rows.len() <= cap {
        return TrimLibraryOutput { rows };
    }
    let mut sorted = rows;
    sorted.sort_by(|a, b| {
        let a_ts = value_to_string(a.as_object().and_then(|m| m.get("ts")));
        let b_ts = value_to_string(b.as_object().and_then(|m| m.get("ts")));
        a_ts.cmp(&b_ts)
    });
    let keep = sorted.split_off(sorted.len().saturating_sub(cap));
    let path = input.file_path.as_deref().unwrap_or("").trim();
    if !path.is_empty() {
        let payload = keep
            .iter()
            .map(|row| serde_json::to_string(row).unwrap_or_else(|_| "null".to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = fs::write(path, format!("{payload}\n"));
    }
    TrimLibraryOutput { rows: keep }
}

pub fn compute_detect_immutable_axiom_violation(
    input: &DetectImmutableAxiomViolationInput,
) -> DetectImmutableAxiomViolationOutput {
    let axioms_policy = value_path(input.policy.as_ref(), &["immutable_axioms"])
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    if !to_bool_like(axioms_policy.get("enabled"), false) {
        return DetectImmutableAxiomViolationOutput { hits: Vec::new() };
    }
    let rows = axioms_policy
        .get("axioms")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if rows.is_empty() {
        return DetectImmutableAxiomViolationOutput { hits: Vec::new() };
    }
    let decision = input
        .decision_input
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let objective = clean_text_runtime(&value_to_string(decision.get("objective")), 500);
    let signature = clean_text_runtime(&value_to_string(decision.get("signature")), 500);
    let filters = decision
        .get("filters")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|row| clean_text_runtime(&value_to_string(Some(row)), 120))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let haystack = clean_text_runtime(
        &[objective.clone(), signature.clone(), filters.join(" ")]
            .join(" ")
            .to_lowercase(),
        2400,
    );
    let token_set = compute_tokenize_text(&TokenizeTextInput {
        value: Some(haystack.clone()),
        max_tokens: Some(64),
    })
    .tokens;
    let intent_tags = compute_normalize_list(&NormalizeListInput {
        value: Some(
            decision
                .get("intent_tags")
                .cloned()
                .unwrap_or(Value::Array(vec![])),
        ),
        max_len: Some(80),
    })
    .items;

    let mut hits = Vec::new();
    for axiom in rows {
        let Some(axiom_obj) = axiom.as_object() else {
            continue;
        };
        let id = compute_normalize_token(&NormalizeTokenInput {
            value: Some(value_to_string(axiom_obj.get("id"))),
            max_len: Some(80),
        })
        .value;
        if id.is_empty() {
            continue;
        }
        let patterns = axiom_obj
            .get("patterns")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|row| {
                compute_normalize_axiom_pattern(&NormalizeAxiomPatternInput {
                    value: Some(value_to_string(Some(row))),
                })
                .value
            })
            .filter(|row| !row.is_empty())
            .collect::<Vec<_>>();
        let mut pattern_matched = false;
        for pattern in patterns {
            let source = compute_pattern_to_word_regex(&PatternToWordRegexInput {
                pattern: Some(pattern),
                max_len: Some(220),
            })
            .source;
            let Some(source) = source else {
                continue;
            };
            let Ok(re) = Regex::new(&source) else {
                continue;
            };
            if re.is_match(&haystack) {
                pattern_matched = true;
                break;
            }
        }

        let regex_rules = axiom_obj
            .get("regex")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|row| clean_text_runtime(&value_to_string(Some(row)), 220))
            .filter(|row| !row.is_empty())
            .collect::<Vec<_>>();
        let regex_matched = regex_rules.iter().any(|rule| {
            Regex::new(rule)
                .ok()
                .map(|re| re.is_match(&haystack))
                .unwrap_or(false)
        });

        let tag_rules = compute_normalize_list(&NormalizeListInput {
            value: Some(
                axiom_obj
                    .get("intent_tags")
                    .cloned()
                    .unwrap_or(Value::Array(vec![])),
            ),
            max_len: Some(80),
        })
        .items;
        let tag_matched = tag_rules
            .iter()
            .any(|tag| intent_tags.iter().any(|it| it == tag));

        let signal_cfg = axiom_obj
            .get("signals")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        let signal_groups = compute_count_axiom_signal_groups(&CountAxiomSignalGroupsInput {
            action_terms: compute_normalize_axiom_signal_terms(&NormalizeAxiomSignalTermsInput {
                terms: signal_cfg
                    .get("action_terms")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default(),
            })
            .terms,
            subject_terms: compute_normalize_axiom_signal_terms(&NormalizeAxiomSignalTermsInput {
                terms: signal_cfg
                    .get("subject_terms")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default(),
            })
            .terms,
            object_terms: compute_normalize_axiom_signal_terms(&NormalizeAxiomSignalTermsInput {
                terms: signal_cfg
                    .get("object_terms")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default(),
            })
            .terms,
            min_signal_groups: axiom_obj.get("min_signal_groups").and_then(|v| v.as_i64()),
            haystack: Some(haystack.clone()),
            token_set: token_set.clone(),
        });
        let structured_signal_configured = signal_groups.configured_groups > 0;
        let structured_pattern_match =
            pattern_matched && (!structured_signal_configured || signal_groups.pass);
        if tag_matched || regex_matched || structured_pattern_match {
            hits.push(id);
        }
    }
    hits.sort();
    hits.dedup();
    DetectImmutableAxiomViolationOutput { hits }
}

pub fn compute_maturity_score(input: &ComputeMaturityScoreInput) -> ComputeMaturityScoreOutput {
    let state = input
        .state
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let policy = input
        .policy
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let stats = state
        .get("stats")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let total = js_number_for_extract(stats.get("total_tests"))
        .unwrap_or(0.0)
        .max(0.0);
    let passed = js_number_for_extract(stats.get("passed_tests"))
        .unwrap_or(0.0)
        .max(0.0);
    let destructive = js_number_for_extract(stats.get("destructive_failures"))
        .unwrap_or(0.0)
        .max(0.0);
    let non_destructive_rate = if total > 0.0 {
        ((total - destructive) / total).max(0.0)
    } else {
        1.0
    };
    let pass_rate = if total > 0.0 {
        (passed / total).max(0.0)
    } else {
        0.0
    };
    let target_test_count = js_number_for_extract(value_path(
        Some(&Value::Object(policy.clone())),
        &["maturity", "target_test_count"],
    ))
    .unwrap_or(40.0)
    .max(1.0);
    let experience = (total / target_test_count).min(1.0);
    let weights = value_path(
        Some(&Value::Object(policy.clone())),
        &["maturity", "score_weights"],
    )
    .and_then(|v| v.as_object())
    .cloned()
    .unwrap_or_default();
    let w_pass = js_number_for_extract(weights.get("pass_rate")).unwrap_or(0.0);
    let w_non = js_number_for_extract(weights.get("non_destructive_rate")).unwrap_or(0.0);
    let w_exp = js_number_for_extract(weights.get("experience")).unwrap_or(0.0);
    let weight_total = (w_pass + w_non + w_exp).max(0.0001);
    let score = ((pass_rate * w_pass) + (non_destructive_rate * w_non) + (experience * w_exp))
        / weight_total;
    let s = clamp_number(score, 0.0, 1.0);
    let bands = value_path(Some(&Value::Object(policy.clone())), &["maturity", "bands"])
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let novice = js_number_for_extract(bands.get("novice")).unwrap_or(0.25);
    let developing = js_number_for_extract(bands.get("developing")).unwrap_or(0.45);
    let mature = js_number_for_extract(bands.get("mature")).unwrap_or(0.65);
    let seasoned = js_number_for_extract(bands.get("seasoned")).unwrap_or(0.82);
    let band = if s < novice {
        "novice".to_string()
    } else if s < developing {
        "developing".to_string()
    } else if s < mature {
        "mature".to_string()
    } else if s < seasoned {
        "seasoned".to_string()
    } else {
        "legendary".to_string()
    };
    ComputeMaturityScoreOutput {
        score: (s * 1_000_000.0).round() / 1_000_000.0,
        band,
        pass_rate: (pass_rate * 1_000_000.0).round() / 1_000_000.0,
        non_destructive_rate: (non_destructive_rate * 1_000_000.0).round() / 1_000_000.0,
        experience: (experience * 1_000_000.0).round() / 1_000_000.0,
    }
}

pub fn compute_select_library_candidates(
    input: &SelectLibraryCandidatesInput,
) -> SelectLibraryCandidatesOutput {
    let policy = input.policy.clone().unwrap_or_else(|| json!({}));
    let query = input
        .query
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let rows = compute_read_jsonl(&ReadJsonlInput {
        file_path: input.file_path.clone(),
    })
    .rows
    .into_iter()
    .map(|row| compute_normalize_library_row(&NormalizeLibraryRowInput { row: Some(row) }).row)
    .collect::<Vec<_>>();
    let min_similarity = js_number_for_extract(value_path(
        Some(&policy),
        &["library", "min_similarity_for_reuse"],
    ))
    .unwrap_or(0.35);
    let mut scored = rows
        .into_iter()
        .map(|row| {
            let similarity = compute_library_match_score(&LibraryMatchScoreInput {
                query_signature_tokens: query
                    .get("signature_tokens")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|v| value_to_string(Some(v)))
                    .collect::<Vec<_>>(),
                query_trit_vector: query
                    .get("trit_vector")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default(),
                query_target: query
                    .get("target")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string()),
                row_signature_tokens: row
                    .as_object()
                    .and_then(|m| m.get("signature_tokens"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|v| value_to_string(Some(v)))
                    .collect::<Vec<_>>(),
                row_outcome_trit: row
                    .as_object()
                    .and_then(|m| m.get("outcome_trit"))
                    .and_then(|v| v.as_i64()),
                row_target: row
                    .as_object()
                    .and_then(|m| m.get("target"))
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string()),
                token_weight: value_path(Some(&policy), &["library", "token_weight"])
                    .and_then(|v| v.as_f64()),
                trit_weight: value_path(Some(&policy), &["library", "trit_weight"])
                    .and_then(|v| v.as_f64()),
                target_weight: value_path(Some(&policy), &["library", "target_weight"])
                    .and_then(|v| v.as_f64()),
            })
            .score;
            let base_certainty = clamp_number(
                js_number_for_extract(row.as_object().and_then(|m| m.get("certainty")))
                    .unwrap_or(0.0),
                0.0,
                1.0,
            );
            let outcome_trit = normalize_trit_value(
                row.as_object()
                    .and_then(|m| m.get("outcome_trit"))
                    .unwrap_or(&Value::Null),
            );
            let confidence_multiplier = if outcome_trit == 1 {
                1.0
            } else if outcome_trit == 0 {
                0.9
            } else {
                0.6
            };
            let candidate_certainty =
                clamp_number(base_certainty * confidence_multiplier, 0.0, 1.0);
            json!({
                "row": row,
                "similarity": (similarity * 1_000_000.0).round() / 1_000_000.0,
                "candidate_certainty": (candidate_certainty * 1_000_000.0).round() / 1_000_000.0
            })
        })
        .filter(|entry| {
            js_number_for_extract(entry.as_object().and_then(|m| m.get("similarity")))
                .unwrap_or(0.0)
                >= min_similarity
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| {
        let a_sim =
            js_number_for_extract(a.as_object().and_then(|m| m.get("similarity"))).unwrap_or(0.0);
        let b_sim =
            js_number_for_extract(b.as_object().and_then(|m| m.get("similarity"))).unwrap_or(0.0);
        if (b_sim - a_sim).abs() > f64::EPSILON {
            return b_sim
                .partial_cmp(&a_sim)
                .unwrap_or(std::cmp::Ordering::Equal);
        }
        let a_cert =
            js_number_for_extract(a.as_object().and_then(|m| m.get("candidate_certainty")))
                .unwrap_or(0.0);
        let b_cert =
            js_number_for_extract(b.as_object().and_then(|m| m.get("candidate_certainty")))
                .unwrap_or(0.0);
        if (b_cert - a_cert).abs() > f64::EPSILON {
            return b_cert
                .partial_cmp(&a_cert)
                .unwrap_or(std::cmp::Ordering::Equal);
        }
        let a_ts = value_to_string(
            a.as_object()
                .and_then(|m| m.get("row"))
                .and_then(|v| v.as_object())
                .and_then(|m| m.get("ts")),
        );
        let b_ts = value_to_string(
            b.as_object()
                .and_then(|m| m.get("row"))
                .and_then(|v| v.as_object())
                .and_then(|m| m.get("ts")),
        );
        b_ts.cmp(&a_ts)
    });
    SelectLibraryCandidatesOutput { candidates: scored }
}
