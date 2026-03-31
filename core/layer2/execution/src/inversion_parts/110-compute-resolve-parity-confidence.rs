pub fn compute_resolve_parity_confidence(
    input: &ResolveParityConfidenceInput,
) -> ResolveParityConfidenceOutput {
    let arg_value = input
        .arg_candidates
        .iter()
        .find_map(|row| compute_extract_numeric(&ExtractNumericInput { value: row.clone() }).value);
    if let Some(value) = arg_value {
        return ResolveParityConfidenceOutput {
            value: round6(clamp_number(value, 0.0, 1.0)),
            source: "arg".to_string(),
        };
    }
    let path_hint = input.path_hint.clone().unwrap_or_default();
    if path_hint.is_empty() {
        return ResolveParityConfidenceOutput {
            value: 0.0,
            source: "none".to_string(),
        };
    }
    let payload = input.payload.as_ref().and_then(|v| v.as_object());
    if payload.is_none() {
        return ResolveParityConfidenceOutput {
            value: 0.0,
            source: clean_text_runtime(
                input
                    .path_source
                    .as_deref()
                    .filter(|row| !row.is_empty())
                    .unwrap_or(&path_hint),
                260,
            ),
        };
    }
    let payload_value = input.payload.as_ref();
    let value = [
        value_path(payload_value, &["confidence"]),
        value_path(payload_value, &["parity_confidence"]),
        value_path(payload_value, &["pass_rate"]),
        value_path(payload_value, &["score"]),
    ]
    .iter()
    .find_map(|row| parse_number_like(*row))
    .unwrap_or(0.0);
    ResolveParityConfidenceOutput {
        value: round6(clamp_number(value, 0.0, 1.0)),
        source: clean_text_runtime(
            input
                .path_source
                .as_deref()
                .filter(|row| !row.is_empty())
                .unwrap_or(&path_hint),
            260,
        ),
    }
}
pub fn compute_attractor_score(input: &ComputeAttractorScoreInput) -> ComputeAttractorScoreOutput {
    let attractor_enabled = map_bool_key(input.attractor.as_ref(), "enabled", false);
    if !attractor_enabled {
        return ComputeAttractorScoreOutput {
            enabled: false,
            score: 1.0,
            required: 0.0,
            pass: true,
            components: json!({}),
        };
    }

    let objective_text = clean_text_runtime(input.objective.as_deref().unwrap_or(""), 600);
    let signature_text = clean_text_runtime(input.signature.as_deref().unwrap_or(""), 600);
    let joined = format!("{} {}", objective_text, signature_text).to_lowercase();
    let token_rows = clean_text_runtime(&joined, 1600)
        .split_whitespace()
        .map(|row| row.trim().to_lowercase())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let token_set = compute_tokenize_text(&TokenizeTextInput {
        value: Some(joined.clone()),
        max_tokens: None,
    })
    .tokens;

    let constraint_markers = [
        Regex::new(r"(?i)\bmust\b").expect("valid constraint regex"),
        Regex::new(r"(?i)\bwithin\b").expect("valid constraint regex"),
        Regex::new(r"(?i)\bby\s+\d").expect("valid constraint regex"),
        Regex::new(r"(?i)\bunder\b").expect("valid constraint regex"),
        Regex::new(r"(?i)\blimit\b").expect("valid constraint regex"),
        Regex::new(r"(?i)\bno more than\b").expect("valid constraint regex"),
        Regex::new(r"(?i)\bat most\b").expect("valid constraint regex"),
        Regex::new(r"(?i)\bcap\b").expect("valid constraint regex"),
        Regex::new(r"(?i)\brequire(?:s|d)?\b").expect("valid constraint regex"),
    ];
    let measurable_markers = [
        Regex::new(r"[%$]").expect("valid measurable regex"),
        Regex::new(r"(?i)\bms\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\bseconds?\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\bminutes?\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\bhours?\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\bdays?\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\bdollars?\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\brevenue\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\byield\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\bdrift\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\blatency\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\bthroughput\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\berror(?:_rate| rate)?\b").expect("valid measurable regex"),
        Regex::new(r"(?i)\baccuracy\b").expect("valid measurable regex"),
    ];
    let comparison_markers = [
        Regex::new(r">=?\s*\d").expect("valid comparison regex"),
        Regex::new(r"<=?\s*\d").expect("valid comparison regex"),
        Regex::new(r"(?i)\b(?:reduce|increase|improve|decrease|raise|lower)\b")
            .expect("valid comparison regex"),
    ];
    let external_markers = [
        Regex::new(r"(?i)https?://").expect("valid external regex"),
        Regex::new(r"(?i)\bgithub\b").expect("valid external regex"),
        Regex::new(r"(?i)\bupwork\b").expect("valid external regex"),
        Regex::new(r"(?i)\breddit\b").expect("valid external regex"),
        Regex::new(r"(?i)\bmarket\b").expect("valid external regex"),
        Regex::new(r"(?i)\bcustomer\b").expect("valid external regex"),
        Regex::new(r"(?i)\busers?\b").expect("valid external regex"),
        Regex::new(r"(?i)\bapi\b").expect("valid external regex"),
        Regex::new(r"(?i)\bweb\b").expect("valid external regex"),
        Regex::new(r"(?i)\bexternal\b").expect("valid external regex"),
    ];

    let number_markers = token_set
        .iter()
        .filter(|tok| tok.chars().any(|ch| ch.is_ascii_digit()))
        .count() as f64;
    let constraint_hits = constraint_markers
        .iter()
        .filter(|re| re.is_match(&joined))
        .count() as f64;
    let measurable_hits = measurable_markers
        .iter()
        .filter(|re| re.is_match(&joined))
        .count() as f64;
    let comparison_hits = comparison_markers
        .iter()
        .filter(|re| re.is_match(&joined))
        .count() as f64;
    let external_hits = external_markers
        .iter()
        .filter(|re| re.is_match(&joined))
        .count() as f64;

    let external_signals_count =
        clamp_int_value(input.external_signals_count.as_ref(), 0, 100000, 0) as f64;
    let evidence_count = clamp_int_value(input.evidence_count.as_ref(), 0, 100000, 0) as f64;
    let word_count = (token_rows.len() as i64).clamp(0, 4000);
    let lexical_diversity = if word_count > 0 {
        clamp_number(
            token_set.len() as f64 / (word_count.max(1) as f64),
            0.0,
            1.0,
        )
    } else {
        0.0
    };

    let verbosity_cfg = input
        .attractor
        .as_ref()
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("verbosity"));
    let soft_word_cap = clamp_int_value(
        verbosity_cfg
            .and_then(|v| v.as_object())
            .and_then(|m| m.get("soft_word_cap")),
        8,
        1000,
        70,
    );
    let hard_word_cap = clamp_int_value(
        verbosity_cfg
            .and_then(|v| v.as_object())
            .and_then(|m| m.get("hard_word_cap")),
        soft_word_cap + 1,
        2000,
        180,
    );
    let low_diversity_floor = clamp_number(
        parse_number_like(
            verbosity_cfg
                .and_then(|v| v.as_object())
                .and_then(|m| m.get("low_diversity_floor")),
        )
        .unwrap_or(0.28),
        0.05,
        0.95,
    );

    let constraint_evidence = clamp_number(
        (constraint_hits * 0.55 + number_markers.min(3.0) * 0.45) / 4.0,
        0.0,
        1.0,
    );
    let measurable_evidence = clamp_number(
        (measurable_hits * 0.6 + comparison_hits * 0.4) / 4.0,
        0.0,
        1.0,
    );
    let external_grounding = clamp_number(
        (external_hits * 0.6 + external_signals_count.min(4.0) * 0.4) / 3.0,
        0.0,
        1.0,
    );
    let evidence_backing = clamp_number(
        (constraint_hits * 0.2)
            + (measurable_hits * 0.2)
            + (external_hits * 0.15)
            + (comparison_hits * 0.1)
            + (evidence_count.min(5.0) * 0.35),
        0.0,
        1.0,
    );
    let specificity = round6(clamp_number(
        (constraint_evidence * 0.4) + (measurable_evidence * 0.35) + (external_grounding * 0.25),
        0.0,
        1.0,
    ));

    let verbosity_over = if word_count > soft_word_cap {
        clamp_number(
            (word_count - soft_word_cap) as f64 / ((hard_word_cap - soft_word_cap).max(1) as f64),
            0.0,
            1.0,
        )
    } else {
        0.0
    };
    let low_diversity_penalty = if lexical_diversity < low_diversity_floor {
        clamp_number(
            (low_diversity_floor - lexical_diversity) / low_diversity_floor.max(0.01),
            0.0,
            1.0,
        )
    } else {
        0.0
    };
    let weak_evidence_penalty = 1.0
        - clamp_number(
            (constraint_evidence * 0.4)
                + (measurable_evidence * 0.3)
                + (external_grounding * 0.2)
                + (evidence_backing * 0.1),
            0.0,
            1.0,
        );
    let verbosity_penalty = round6(clamp_number(
        (verbosity_over * weak_evidence_penalty * 0.75) + (low_diversity_penalty * 0.25),
        0.0,
        1.0,
    ));

    let objective_specificity_weight = js_or_number(
        value_path(
            input.attractor.as_ref(),
            &["weights", "objective_specificity"],
        ),
        0.0,
    );
    let evidence_backing_weight = js_or_number(
        value_path(input.attractor.as_ref(), &["weights", "evidence_backing"]),
        0.0,
    );
    let constraint_weight = if value_path(
        input.attractor.as_ref(),
        &["weights", "constraint_evidence"],
    )
    .is_some()
    {
        parse_number_like(value_path(
            input.attractor.as_ref(),
            &["weights", "constraint_evidence"],
        ))
        .unwrap_or(0.0)
    } else {
        objective_specificity_weight * 0.4
    };
    let measurable_weight =
        if value_path(input.attractor.as_ref(), &["weights", "measurable_outcome"]).is_some() {
            parse_number_like(value_path(
                input.attractor.as_ref(),
                &["weights", "measurable_outcome"],
            ))
            .unwrap_or(0.0)
        } else {
            objective_specificity_weight * 0.35
        };
    let external_weight =
        if value_path(input.attractor.as_ref(), &["weights", "external_grounding"]).is_some() {
            parse_number_like(value_path(
                input.attractor.as_ref(),
                &["weights", "external_grounding"],
            ))
            .unwrap_or(0.0)
        } else {
            objective_specificity_weight * 0.25
        };
    let certainty_weight = js_or_number(
        value_path(input.attractor.as_ref(), &["weights", "certainty"]),
        0.0,
    );
    let trit_alignment_weight = js_or_number(
        value_path(input.attractor.as_ref(), &["weights", "trit_alignment"]),
        0.0,
    );
    let impact_alignment_weight = js_or_number(
        value_path(input.attractor.as_ref(), &["weights", "impact_alignment"]),
        0.0,
    );
    let positive_weight_total = (objective_specificity_weight
        + evidence_backing_weight
        + constraint_weight
        + measurable_weight
        + external_weight
        + certainty_weight
        + trit_alignment_weight
        + impact_alignment_weight)
        .max(0.0001);
    let verbosity_penalty_weight = js_or_number(
        value_path(input.attractor.as_ref(), &["weights", "verbosity_penalty"]),
        0.0,
    );

    let certainty = clamp_number(
        parse_number_like(input.effective_certainty.as_ref()).unwrap_or(0.0),
        0.0,
        1.0,
    );
    let trit = clamp_int_value(input.trit.as_ref(), -1, 1, 0);
    let trit_alignment = if trit == 1 {
        1.0
    } else if trit == 0 {
        0.6
    } else {
        0.15
    };
    let impact = compute_normalize_impact(&NormalizeImpactInput {
        value: input.impact.clone(),
    })
    .value;
    let impact_factor = if impact == "critical" {
        1.0
    } else if impact == "high" {
        0.85
    } else if impact == "medium" {
        0.7
    } else {
        0.55
    };

    let positive_score = ((specificity * objective_specificity_weight)
        + (evidence_backing * evidence_backing_weight)
        + (constraint_evidence * constraint_weight)
        + (measurable_evidence * measurable_weight)
        + (external_grounding * external_weight)
        + (certainty * certainty_weight)
        + (trit_alignment * trit_alignment_weight)
        + (impact_factor * impact_alignment_weight))
        / positive_weight_total;
    let score = clamp_number(
        positive_score - (verbosity_penalty * verbosity_penalty_weight),
        0.0,
        1.0,
    );

    let target = normalize_target_for_key(input.target.as_deref().unwrap_or("tactical"));
    let required = clamp_number(
        parse_number_like(
            input
                .attractor
                .as_ref()
                .and_then(|v| v.as_object())
                .and_then(|m| m.get("min_alignment_by_target"))
                .and_then(|v| v.as_object())
                .and_then(|m| m.get(&target)),
        )
        .unwrap_or(0.0),
        0.0,
        1.0,
    );
    let score_rounded = round6(clamp_number(score, 0.0, 1.0));
    let required_rounded = round6(required);
    ComputeAttractorScoreOutput {
        enabled: true,
        score: score_rounded,
        required: required_rounded,
        pass: score_rounded >= required_rounded,
        components: json!({
            "objective_specificity": round6(specificity),
            "evidence_backing": round6(evidence_backing),
            "constraint_evidence": round6(constraint_evidence),
            "measurable_outcome": round6(measurable_evidence),
            "external_grounding": round6(external_grounding),
            "certainty": round6(certainty),
            "trit_alignment": round6(trit_alignment),
            "impact_alignment": round6(impact_factor),
            "verbosity_penalty": round6(verbosity_penalty),
            "lexical_diversity": round6(lexical_diversity),
            "word_count": word_count
        }),
    }
}
