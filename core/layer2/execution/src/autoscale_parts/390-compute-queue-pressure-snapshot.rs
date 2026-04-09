pub fn compute_queue_pressure_snapshot(
    input: &QueuePressureSnapshotInput,
) -> QueuePressureSnapshotOutput {
    let mut total: u32 = 0;
    let mut pending: u32 = 0;
    let mut accepted: u32 = 0;
    let mut closed: u32 = 0;
    let mut rejected: u32 = 0;
    let mut parked: u32 = 0;

    for status in &input.statuses {
        total += 1;
        let normalized = status.trim().to_ascii_lowercase();
        if normalized == "pending" {
            pending += 1;
        } else if normalized == "accepted" {
            accepted += 1;
        } else if normalized == "closed" {
            closed += 1;
        } else if normalized == "rejected" {
            rejected += 1;
        } else if normalized == "parked" {
            parked += 1;
        }
    }

    let pending_ratio = if total > 0 {
        round6((pending as f64) / (total as f64))
    } else {
        0.0
    };
    let warn_ratio = round6(clamp_ratio(input.warn_ratio));
    let critical_ratio = round6(clamp_ratio(input.critical_ratio));
    let warn_count = autoscale_non_negative(input.warn_count);
    let critical_count = autoscale_non_negative(input.critical_count);

    let mut pressure = "normal".to_string();
    if (pending as f64) >= critical_count || pending_ratio >= critical_ratio {
        pressure = "critical".to_string();
    } else if (pending as f64) >= warn_count || pending_ratio >= warn_ratio {
        pressure = "warning".to_string();
    }

    QueuePressureSnapshotOutput {
        total,
        pending,
        accepted,
        closed,
        rejected,
        parked,
        pending_ratio,
        pressure,
        warn_ratio,
        critical_ratio,
        warn_count,
        critical_count,
    }
}

fn push_parse_success_criteria_text(
    rows: &mut Vec<ParseSuccessCriteriaRowOutput>,
    text: &str,
    source: &str,
    success_metric_re: &Regex,
    success_timebound_re: &Regex,
    success_relaxed_horizon_re: &Regex,
    success_comparator_re: &Regex,
) {
    let clean = normalize_spaces(text);
    if clean.is_empty() {
        return;
    }
    let has_timebound =
        success_timebound_re.is_match(&clean) || success_relaxed_horizon_re.is_match(&clean);
    let metric = success_metric_re
        .captures(&clean)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_ascii_lowercase())
        .unwrap_or_default();
    let measurable = success_metric_re.is_match(&clean)
        && (has_timebound
            || clean.chars().any(|ch| ch.is_ascii_digit())
            || success_comparator_re.is_match(&clean));
    rows.push(ParseSuccessCriteriaRowOutput {
        source: source.to_string(),
        metric,
        target: clean.chars().take(140).collect(),
        measurable,
    });
}

pub fn compute_parse_success_criteria_rows(
    input: &ParseSuccessCriteriaRowsInput,
) -> ParseSuccessCriteriaRowsOutput {
    let success_metric_re = Regex::new(
        r"(?i)\b(metric|kpi|target|rate|count|latency|error|uptime|throughput|conversion|artifact|receipt|coverage|reply|interview|pass|fail|delta|percent|%|run|runs|check|checks|items_collected)\b",
    )
    .expect("valid success metric regex");
    let success_timebound_re = Regex::new(
        r"(?i)\b(\d+\s*(h|hr|hour|hours|d|day|days|w|week|weeks|min|mins|minute|minutes)|daily|weekly|monthly|quarterly)\b",
    )
    .expect("valid success timebound regex");
    let success_relaxed_horizon_re =
        Regex::new(r"(?i)\b(next|this)\s+(run|cycle)\b").expect("valid success relaxed regex");
    let success_comparator_re =
        Regex::new(r"(?i)\b(>=|<=|>|<|at least|at most|less than|more than|within|under|over)\b")
            .expect("valid success comparator regex");

    let has_timebound = |text: &str| -> bool {
        let clean = normalize_spaces(text);
        if clean.is_empty() {
            return false;
        }
        success_timebound_re.is_match(&clean) || success_relaxed_horizon_re.is_match(&clean)
    };
    let structured_measurable = |metric: &str, target: &str, horizon: &str| -> bool {
        let m = normalize_spaces(metric);
        let t = normalize_spaces(target);
        let h = normalize_spaces(horizon);
        if m.is_empty() || t.is_empty() {
            return false;
        }
        let metric_like = success_metric_re.is_match(&m) || m.contains('_') || m.contains('-');
        let quantified_target = t.chars().any(|ch| ch.is_ascii_digit())
            || success_comparator_re.is_match(&t)
            || success_metric_re.is_match(&t);
        let timebound = has_timebound(&format!("{h} {t}"));
        metric_like && (quantified_target || timebound)
    };

    let mut rows: Vec<ParseSuccessCriteriaRowOutput> = Vec::new();

    for row in &input.action_rows {
        if let Some(raw) = row.as_str() {
            push_parse_success_criteria_text(
                &mut rows,
                raw,
                "action_spec.success_criteria",
                &success_metric_re,
                &success_timebound_re,
                &success_relaxed_horizon_re,
                &success_comparator_re,
            );
            continue;
        }
        let Some(obj) = row.as_object() else {
            continue;
        };
        let metric_raw = obj
            .get("metric")
            .or_else(|| obj.get("name"))
            .map(js_like_string)
            .unwrap_or_default();
        let target_raw = obj
            .get("target")
            .or_else(|| obj.get("threshold"))
            .or_else(|| obj.get("description"))
            .or_else(|| obj.get("goal"))
            .map(js_like_string)
            .unwrap_or_default();
        let horizon_raw = obj
            .get("horizon")
            .or_else(|| obj.get("window"))
            .or_else(|| obj.get("by"))
            .map(js_like_string)
            .unwrap_or_default();

        let metric = normalize_spaces(&metric_raw);
        let target = normalize_spaces(&target_raw);
        let horizon = normalize_spaces(&horizon_raw);
        let merged = normalize_spaces(
            &[metric.clone(), target.clone(), horizon.clone()]
                .into_iter()
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
                .join(" | "),
        );
        if merged.is_empty() {
            continue;
        }
        rows.push(ParseSuccessCriteriaRowOutput {
            source: "action_spec.success_criteria".to_string(),
            metric: metric.to_ascii_lowercase(),
            target: merged.chars().take(140).collect(),
            measurable: structured_measurable(&metric, &target, &horizon),
        });
    }

    for row in &input.verify_rows {
        let text = js_like_string(row);
        push_parse_success_criteria_text(
            &mut rows,
            &text,
            "action_spec.verify",
            &success_metric_re,
            &success_timebound_re,
            &success_relaxed_horizon_re,
            &success_comparator_re,
        );
    }
    for row in &input.validation_rows {
        let text = js_like_string(row);
        push_parse_success_criteria_text(
            &mut rows,
            &text,
            "validation",
            &success_metric_re,
            &success_timebound_re,
            &success_relaxed_horizon_re,
            &success_comparator_re,
        );
    }

    let mut dedupe = std::collections::HashSet::<String>::new();
    let mut out: Vec<ParseSuccessCriteriaRowOutput> = Vec::new();
    for row in rows {
        if row.target.is_empty() {
            continue;
        }
        let key = format!("{}|{}", row.metric, row.target).to_ascii_lowercase();
        if !dedupe.insert(key) {
            continue;
        }
        out.push(row);
    }

    ParseSuccessCriteriaRowsOutput { rows: out }
}

pub fn compute_collect_outcome_stats(
    input: &CollectOutcomeStatsInput,
) -> CollectOutcomeStatsOutput {
    let normalize_bucket = |row: &CollectOutcomeStatsBucketInput| CollectOutcomeStatsBucketInput {
        shipped: autoscale_non_negative(row.shipped),
        no_change: autoscale_non_negative(row.no_change),
        reverted: autoscale_non_negative(row.reverted),
    };
    let to_bias_output = |row: &CollectOutcomeStatsBucketInput, min_total: f64| {
        let normalized = normalize_bucket(row);
        let derived = compute_derive_entity_bias(&DeriveEntityBiasInput {
            shipped: normalized.shipped,
            no_change: normalized.no_change,
            reverted: normalized.reverted,
            min_total: autoscale_non_negative(min_total),
        });
        (
            derived.bias,
            CollectOutcomeStatsBiasOutput {
                shipped: normalized.shipped,
                no_change: normalized.no_change,
                reverted: normalized.reverted,
                total: derived.total,
                bias: derived.bias,
            },
        )
    };

    let global_normalized = normalize_bucket(&input.global);
    let global_total = compute_total_outcomes(&TotalOutcomesInput {
        shipped: global_normalized.shipped,
        no_change: global_normalized.no_change,
        reverted: global_normalized.reverted,
    })
    .total;
    let global = CollectOutcomeStatsGlobalOutput {
        shipped: global_normalized.shipped,
        no_change: global_normalized.no_change,
        reverted: global_normalized.reverted,
        total: global_total,
    };

    let mut eye_biases = std::collections::BTreeMap::<String, CollectOutcomeStatsBiasOutput>::new();
    for (key, row) in &input.by_eye {
        let (bias, output) = to_bias_output(row, input.eye_min_samples);
        if bias != 0.0 {
            eye_biases.insert(key.clone(), output);
        }
    }

    let mut topic_biases =
        std::collections::BTreeMap::<String, CollectOutcomeStatsBiasOutput>::new();
    for (key, row) in &input.by_topic {
        let (bias, output) = to_bias_output(row, input.topic_min_samples);
        if bias != 0.0 {
            topic_biases.insert(key.clone(), output);
        }
    }

    CollectOutcomeStatsOutput {
        global,
        eye_biases,
        topic_biases,
    }
}

pub fn compute_subdirective_v2_signals(
    input: &SubdirectiveV2SignalsInput,
) -> SubdirectiveV2SignalsOutput {
    SubdirectiveV2SignalsOutput {
        required: input.required,
        has_concrete_target: input.has_concrete_target,
        has_expected_delta: input.has_expected_delta,
        has_verification_step: input.has_verification_step,
        target_count: autoscale_non_negative(input.target_count),
        verify_count: autoscale_non_negative(input.verify_count),
        success_criteria_count: autoscale_non_negative(input.success_criteria_count),
    }
}
