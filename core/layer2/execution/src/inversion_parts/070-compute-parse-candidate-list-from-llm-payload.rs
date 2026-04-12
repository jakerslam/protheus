fn candidate_rows_from_payload(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(arr) => arr.clone(),
        Value::Object(obj) => obj
            .get("candidates")
            .and_then(|v| v.as_array())
            .cloned()
            .or_else(|| {
                obj.get("payload")
                    .map(candidate_rows_from_payload)
                    .filter(|rows| !rows.is_empty())
            })
            .or_else(|| {
                obj.get("output")
                    .map(candidate_rows_from_payload)
                    .filter(|rows| !rows.is_empty())
            })
            .unwrap_or_default(),
        Value::String(raw) => serde_json::from_str::<Value>(raw)
            .ok()
            .map(|parsed| candidate_rows_from_payload(&parsed))
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

pub fn compute_parse_candidate_list_from_llm_payload(
    input: &ParseCandidateListFromLlmPayloadInput,
) -> ParseCandidateListFromLlmPayloadOutput {
    let rows = input
        .payload
        .as_ref()
        .map(candidate_rows_from_payload)
        .unwrap_or_default();
    let mut out = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        let row_obj = row.as_object();
        let filters_src = row_obj
            .and_then(|obj| obj.get("filters"))
            .cloned()
            .or_else(|| row_obj.and_then(|obj| obj.get("filter_stack")).cloned())
            .or_else(|| row_obj.and_then(|obj| obj.get("filterStack")).cloned())
            .unwrap_or_else(|| Value::String(String::new()));
        let mut filters = compute_normalize_list(&NormalizeListInput {
            value: Some(filters_src),
            max_len: Some(120),
        })
        .items;
        filters.truncate(8);
        if filters.is_empty() {
            continue;
        }
        let fallback_id = format!("llm_{}", idx + 1);
        let id_raw = row_obj
            .and_then(|obj| obj.get("id"))
            .map(|v| value_to_string(Some(v)))
            .unwrap_or_else(|| fallback_id.clone());
        let id = {
            let token = normalize_token_runtime(&id_raw, 80);
            if token.is_empty() {
                fallback_id.clone()
            } else {
                token
            }
        };
        let probability = round6(clamp_number(
            parse_number_like(row_obj.and_then(|obj| obj.get("probability"))).unwrap_or(0.55),
            0.0,
            1.0,
        ));
        let rationale = clean_text_runtime(
            &row_obj
                .and_then(|obj| obj.get("rationale"))
                .or_else(|| row_obj.and_then(|obj| obj.get("reason")))
                .map(|v| value_to_string(Some(v)))
                .unwrap_or_default(),
            220,
        );
        out.push(InversionCandidateRow {
            id,
            filters,
            source: "right_brain_llm".to_string(),
            probability,
            rationale,
        });
    }
    ParseCandidateListFromLlmPayloadOutput { candidates: out }
}

pub fn compute_heuristic_filter_candidates(
    input: &HeuristicFilterCandidatesInput,
) -> HeuristicFilterCandidatesOutput {
    let objective = input.objective.as_deref().unwrap_or("");
    let tags = compute_tokenize_text(&TokenizeTextInput {
        value: Some(objective.to_string()),
        max_tokens: Some(64),
    })
    .tokens;
    let has_tag = |needle: &str| tags.iter().any(|tag| tag == needle);
    let mut base: Vec<Vec<String>> = vec![
        vec![
            "assumption_inversion".to_string(),
            "constraint_reframe".to_string(),
        ],
        vec!["resource_rebalance".to_string(), "path_split".to_string()],
        vec![
            "goal_decomposition".to_string(),
            "fallback_pathing".to_string(),
        ],
        vec![
            "evidence_intensification".to_string(),
            "risk_guard_compaction".to_string(),
        ],
        vec![
            "time_horizon_reframe".to_string(),
            "bounded_parallel_probe".to_string(),
        ],
        vec![
            "negative_space_scan".to_string(),
            "safe_counterfactual".to_string(),
        ],
    ];
    if has_tag("budget") || has_tag("cost") {
        base.push(vec![
            "cost_lane_swap".to_string(),
            "constraint_reframe".to_string(),
        ]);
    }
    if has_tag("yield") || has_tag("quality") {
        base.push(vec![
            "yield_reframe".to_string(),
            "verification_gate".to_string(),
        ]);
    }
    if has_tag("drift") {
        base.push(vec![
            "drift_anchor".to_string(),
            "identity_guard".to_string(),
        ]);
    }
    let mut out = Vec::new();
    for (idx, filters) in base.iter().enumerate() {
        let mut normalized = compute_normalize_list(&NormalizeListInput {
            value: Some(Value::Array(
                filters
                    .iter()
                    .map(|row| Value::String(row.clone()))
                    .collect::<Vec<_>>(),
            )),
            max_len: Some(120),
        })
        .items;
        normalized.truncate(8);
        let probability = round6(clamp_number(0.42 + (idx as f64 * 0.03), 0.0, 1.0));
        out.push(InversionCandidateRow {
            id: format!("heur_{}", idx + 1),
            filters: normalized,
            source: "heuristic".to_string(),
            probability,
            rationale: "heuristic seed".to_string(),
        });
    }
    HeuristicFilterCandidatesOutput { candidates: out }
}

pub fn compute_score_trial(input: &ScoreTrialInput) -> ScoreTrialOutput {
    let decision = input.decision.as_ref();
    let candidate = input.candidate.as_ref();
    let trial_cfg = input.trial_cfg.as_ref();
    let weights = value_path(trial_cfg, &["score_weights"]);
    let w_allowed = js_or_number(value_path(weights, &["decision_allowed"]), 0.35);
    let w_attractor = js_or_number(value_path(weights, &["attractor"]), 0.2);
    let w_certainty = js_or_number(value_path(weights, &["certainty_margin"]), 0.15);
    let w_library = js_or_number(value_path(weights, &["library_similarity"]), 0.1);
    let w_probe = js_or_number(value_path(weights, &["runtime_probe"]), 0.2);
    let weight_total = (w_allowed + w_attractor + w_certainty + w_library + w_probe).max(0.0001);
    let certainty_margin = clamp_number(
        number_path(decision, &["input", "effective_certainty"], 0.0)
            - number_path(decision, &["gating", "required_certainty"], 0.0),
        -1.0,
        1.0,
    );
    let certainty_score = if certainty_margin <= 0.0 {
        0.0
    } else {
        clamp_number(certainty_margin, 0.0, 1.0)
    };
    let allowed_score = if js_truthy(value_path(decision, &["allowed"])) {
        1.0
    } else {
        0.0
    };
    let attractor_score = number_path(decision, &["attractor", "score"], 0.0);
    let library_score = number_path(candidate, &["score_hint"], 0.0);
    let probe_score = if input.runtime_probe_pass.unwrap_or(false) {
        1.0
    } else {
        0.0
    };
    let score = ((allowed_score * w_allowed)
        + (attractor_score * w_attractor)
        + (certainty_score * w_certainty)
        + (library_score * w_library)
        + (probe_score * w_probe))
        / weight_total;
    ScoreTrialOutput {
        score: round6(clamp_number(score, 0.0, 1.0)),
    }
}

pub fn compute_mutate_trial_candidates(
    input: &MutateTrialCandidatesInput,
) -> MutateTrialCandidatesOutput {
    let mutation_stack = [
        "constraint_reframe",
        "goal_decomposition",
        "fallback_pathing",
        "risk_guard_compaction",
    ];
    let mut out = Vec::new();
    let mut idx = 0usize;
    for row in &input.rows {
        let row_obj = row.as_object();
        let mut filters = compute_normalize_list(&NormalizeListInput {
            value: row_obj
                .and_then(|obj| obj.get("filters"))
                .cloned()
                .or_else(|| Some(json!([]))),
            max_len: Some(120),
        })
        .items;
        let extra = mutation_stack[idx % mutation_stack.len()].to_string();
        idx += 1;
        filters.push(extra);
        let mut merged = compute_normalize_list(&NormalizeListInput {
            value: Some(Value::Array(
                filters.into_iter().map(Value::String).collect::<Vec<_>>(),
            )),
            max_len: Some(120),
        })
        .items;
        merged.truncate(8);

        let fallback_seed = if row.is_null() {
            "{}".to_string()
        } else {
            serde_json::to_string(row).unwrap_or_else(|_| "{}".to_string())
        };
        let id_prefix = row_obj
            .and_then(|obj| obj.get("id"))
            .map(|v| value_to_string(Some(v)))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| stable_id_runtime(&fallback_seed, "mut"));
        let source_prefix = row_obj
            .and_then(|obj| obj.get("source"))
            .map(|v| value_to_string(Some(v)))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "trial".to_string());
        let probability = round6(clamp_number(
            js_or_number(row_obj.and_then(|obj| obj.get("probability")), 0.4) * 0.92,
            0.0,
            1.0,
        ));
        let score_hint = round6(clamp_number(
            parse_number_like(row_obj.and_then(|obj| obj.get("score_hint"))).unwrap_or(0.0) * 0.94,
            0.0,
            1.0,
        ));

        let mut next = row_obj.cloned().unwrap_or_default();
        next.insert(
            "id".to_string(),
            Value::String(format!("{id_prefix}_m{idx}")),
        );
        next.insert(
            "filters".to_string(),
            Value::Array(
                merged
                    .iter()
                    .map(|row| Value::String(row.clone()))
                    .collect::<Vec<_>>(),
            ),
        );
        next.insert(
            "source".to_string(),
            Value::String(format!("{source_prefix}_mutated")),
        );
        next.insert("probability".to_string(), json!(probability));
        next.insert("score_hint".to_string(), json!(score_hint));
        out.push(Value::Object(next));
    }
    MutateTrialCandidatesOutput { rows: out }
}

const TIER_TARGETS: [&str; 5] = [
    "tactical",
    "belief",
    "identity",
    "directive",
    "constitution",
];
const TIER_METRICS: [&str; 5] = [
    "live_apply_attempts",
    "live_apply_successes",
    "live_apply_safe_aborts",
    "shadow_passes",
    "shadow_critical_failures",
];

fn now_iso_runtime() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_ts_ms_runtime(value: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0)
}

fn array_to_string_rows(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .map(|row| value_to_string(Some(row)))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn default_tier_event_map_value() -> Value {
    json!({
        "tactical": [],
        "belief": [],
        "identity": [],
        "directive": [],
        "constitution": []
    })
}

pub fn compute_normalize_iso_events(input: &NormalizeIsoEventsInput) -> NormalizeIsoEventsOutput {
    let max_rows = input.max_rows.unwrap_or(10000).clamp(1, 100000) as usize;
    let mut out = input
        .src
        .iter()
        .map(|row| value_to_string(Some(row)).trim().to_string())
        .filter(|row| parse_ts_ms_runtime(row) > 0)
        .collect::<Vec<_>>();
    if out.len() > max_rows {
        out = out[(out.len() - max_rows)..].to_vec();
    }
    out.sort_by_key(|row| parse_ts_ms_runtime(row));
    let mut dedup = Vec::new();
    for row in out {
        if !dedup.iter().any(|existing| existing == &row) {
            dedup.push(row);
        }
    }
    NormalizeIsoEventsOutput { events: dedup }
}

pub fn compute_expand_legacy_count_to_events(
    input: &ExpandLegacyCountToEventsInput,
) -> ExpandLegacyCountToEventsOutput {
    let n = clamp_int_value(input.count.as_ref(), 0, 4096, 0);
    if n <= 0 {
        return ExpandLegacyCountToEventsOutput { events: Vec::new() };
    }
    let ts = input.ts.clone().unwrap_or_else(now_iso_runtime);
    ExpandLegacyCountToEventsOutput {
        events: (0..n).map(|_| ts.clone()).collect::<Vec<_>>(),
    }
}

fn normalize_tier_event_map_value(
    src: Option<&Value>,
    fallback: Option<&Value>,
    legacy_counts: Option<&Value>,
    legacy_ts: &str,
) -> Value {
    let mut out = serde_json::Map::new();
    for target in TIER_TARGETS {
        let src_rows = src
            .and_then(|v| v.as_object())
            .and_then(|m| m.get(target))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if !src_rows.is_empty() {
            let normalized = compute_normalize_iso_events(&NormalizeIsoEventsInput {
                src: src_rows,
                max_rows: Some(10000),
            });
            out.insert(
                target.to_string(),
                Value::Array(
                    normalized
                        .events
                        .into_iter()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                ),
            );
            continue;
        }

        let legacy_count = legacy_counts
            .and_then(|v| v.as_object())
            .and_then(|m| m.get(target))
            .cloned();
        if legacy_count.is_some() {
            let legacy = compute_expand_legacy_count_to_events(&ExpandLegacyCountToEventsInput {
                count: legacy_count,
                ts: Some(legacy_ts.to_string()),
            });
            if !legacy.events.is_empty() {
                out.insert(
                    target.to_string(),
                    Value::Array(
                        legacy
                            .events
                            .into_iter()
                            .map(Value::String)
                            .collect::<Vec<_>>(),
                    ),
                );
                continue;
            }
        }

        let fallback_rows = array_to_string_rows(
            fallback
                .and_then(|v| v.as_object())
                .and_then(|m| m.get(target)),
        );
        out.insert(
            target.to_string(),
            Value::Array(
                fallback_rows
                    .into_iter()
                    .map(Value::String)
                    .collect::<Vec<_>>(),
            ),
        );
    }
    Value::Object(out)
}

pub fn compute_normalize_tier_event_map(
    input: &NormalizeTierEventMapInput,
) -> NormalizeTierEventMapOutput {
    let legacy_ts = input.legacy_ts.clone().unwrap_or_else(now_iso_runtime);
    NormalizeTierEventMapOutput {
        map: normalize_tier_event_map_value(
            input.src.as_ref(),
            input.fallback.as_ref(),
            input.legacy_counts.as_ref(),
            &legacy_ts,
        ),
    }
}
