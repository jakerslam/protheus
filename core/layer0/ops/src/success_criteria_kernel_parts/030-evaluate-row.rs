fn evaluate_row(row: &SuccessCriteriaCompiledRow, context: &Value) -> EvaluationVerdict {
    let metric = row.metric.to_ascii_lowercase();
    let target = row.target.clone();
    let text = format!("{} {}", metric, target).to_ascii_lowercase();
    let text_words = text.replace(['_', '-'], " ");
    let metric_norm = metric.replace([' ', '-'], "_");
    let top = context.as_object();
    let outcome = top
        .and_then(|map| map.get("outcome"))
        .map(js_like_string)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let exec_ok = top
        .and_then(|map| map.get("exec_ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let dod_passed = top
        .and_then(|map| map.get("dod_passed"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let postconditions_ok = top
        .and_then(|map| map.get("postconditions_ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let queue_outcome_logged = top
        .and_then(|map| map.get("queue_outcome_logged"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let web_tooling_ready = top
        .and_then(|map| map.get("web_tooling_ready"))
        .and_then(Value::as_bool)
        .or_else(|| {
            top.and_then(|map| map.get("web_tooling_health"))
                .and_then(|v| v.get("ok"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(false);
    let duration_ms = read_numeric_metric(context, &["duration_ms"]);
    let token_usage = top
        .and_then(|map| map.get("token_usage"))
        .and_then(Value::as_object);
    let effective_tokens = token_usage
        .and_then(|map| map.get("effective_tokens").and_then(|v| as_f64(Some(v))))
        .or_else(|| {
            token_usage.and_then(|map| map.get("actual_total_tokens").and_then(|v| as_f64(Some(v))))
        })
        .or_else(|| {
            token_usage.and_then(|map| map.get("estimated_tokens").and_then(|v| as_f64(Some(v))))
        });
    let artifacts_delta = read_numeric_metric(
        context,
        &["artifacts_delta", "artifacts_count", "artifact_count"],
    );
    let entries_delta = read_numeric_metric(context, &["entries_delta", "entries_count"]);
    let revenue_delta =
        read_numeric_metric(context, &["revenue_actions_delta", "revenue_actions_count"]);
    let has_any = |tokens: &[&str]| tokens.iter().any(|token| text_words.contains(token));

    let numeric_verdict = |reason: &str,
                           comparator: &str,
                           value: Option<f64>,
                           threshold: Option<f64>,
                           unavailable: &str,
                           unit: Option<&str>| {
        let pass = compare_numeric(value, threshold, comparator);
        match pass {
            Some(v) => EvaluationVerdict {
                evaluated: true,
                pass: Some(v),
                reason: reason.to_string(),
                comparator: Some(comparator.to_string()),
                value: value.map(|n| json!(n)),
                target: threshold.map(|n| json!(n)),
                unit: unit.map(|u| u.to_string()),
            },
            None => EvaluationVerdict {
                evaluated: false,
                pass: None,
                reason: unavailable.to_string(),
                comparator: None,
                value: None,
                target: None,
                unit: None,
            },
        }
    };

    match metric_norm.as_str() {
        "execution_success" => {
            return bool_verdict(
                "requires_execution_success",
                exec_ok,
                Value::Bool(exec_ok),
                Value::Bool(true),
            )
        }
        "postconditions_ok" => {
            return bool_verdict(
                "requires_postconditions_pass",
                postconditions_ok,
                Value::Bool(postconditions_ok),
                Value::Bool(true),
            )
        }
        "queue_outcome_logged" => {
            return bool_verdict(
                "requires_receipt_or_outcome_log",
                queue_outcome_logged,
                Value::Bool(queue_outcome_logged),
                Value::Bool(true),
            )
        }
        "web_tooling_ready" => {
            return bool_verdict(
                "requires_web_tooling_ready",
                web_tooling_ready,
                Value::Bool(web_tooling_ready),
                Value::Bool(true),
            )
        }
        "artifact_count" => {
            let threshold = parse_first_int(&text, 1) as f64;
            let comparator = parse_comparator(&text, "gte");
            return numeric_verdict(
                "artifact_delta_check",
                &comparator,
                artifacts_delta,
                Some(threshold),
                "artifact_delta_unavailable",
                None,
            );
        }
        "entries_count" => {
            let threshold = parse_first_int(&text, 1) as f64;
            let comparator = parse_comparator(&text, "gte");
            return numeric_verdict(
                "entry_delta_check",
                &comparator,
                entries_delta,
                Some(threshold),
                "entry_delta_unavailable",
                None,
            );
        }
        "revenue_actions_count" => {
            let threshold = parse_first_int(&text, 1) as f64;
            let comparator = parse_comparator(&text, "gte");
            return numeric_verdict(
                "revenue_delta_check",
                &comparator,
                revenue_delta,
                Some(threshold),
                "revenue_delta_unavailable",
                None,
            );
        }
        "token_usage" => {
            let limit = parse_token_limit(&text).map(|v| v as f64);
            if limit.is_none() {
                return EvaluationVerdict {
                    evaluated: false,
                    pass: None,
                    reason: "token_limit_missing".to_string(),
                    comparator: None,
                    value: None,
                    target: None,
                    unit: None,
                };
            }
            let comparator = parse_comparator(&text, "lte");
            return numeric_verdict(
                "token_limit_check",
                &comparator,
                effective_tokens,
                limit,
                "token_usage_unavailable",
                None,
            );
        }
        "duration_ms" => {
            let limit = parse_duration_limit_ms(&text).map(|v| v as f64);
            if limit.is_none() {
                return EvaluationVerdict {
                    evaluated: false,
                    pass: None,
                    reason: "duration_limit_missing".to_string(),
                    comparator: None,
                    value: None,
                    target: None,
                    unit: None,
                };
            }
            let comparator = parse_comparator(&text, "lte");
            return numeric_verdict(
                "duration_limit_check",
                &comparator,
                duration_ms,
                limit,
                "duration_unavailable",
                Some("ms"),
            );
        }
        "outreach_artifact" => {
            let threshold = parse_first_int(&text, 1) as f64;
            let comparator = parse_comparator(&text, "gte");
            let value = read_numeric_metric(
                context,
                &[
                    "outreach_artifact",
                    "outreach_artifact_count",
                    "offer_draft_count",
                    "proposal_draft_count",
                ],
            )
            .or(artifacts_delta);
            return numeric_verdict(
                "outreach_artifact_check",
                &comparator,
                value,
                Some(threshold),
                "outreach_artifact_unavailable",
                None,
            );
        }
        "reply_or_interview_count" => {
            let threshold = parse_first_int(&text, 1) as f64;
            let comparator = parse_comparator(&text, "gte");
            let value = read_numeric_metric(context, &["reply_or_interview_count"]).or_else(|| {
                let reply = read_numeric_metric(context, &["reply_count", "outreach_reply_count"])
                    .unwrap_or(0.0);
                let interview =
                    read_numeric_metric(context, &["interview_count", "outreach_interview_count"])
                        .unwrap_or(0.0);
                if reply > 0.0 || interview > 0.0 {
                    Some(reply + interview)
                } else {
                    None
                }
            });
            return numeric_verdict(
                "reply_or_interview_count_check",
                &comparator,
                value,
                Some(threshold),
                "reply_or_interview_count_unavailable",
                None,
            );
        }
        _ => {}
    }

    if has_any(&[
        "ship",
        "shipped",
        "publish",
        "posted",
        "merged",
        "applied",
        "delivered",
    ]) {
        return EvaluationVerdict {
            evaluated: true,
            pass: Some(outcome == "shipped"),
            reason: "requires_shipped_outcome".to_string(),
            comparator: None,
            value: Some(Value::String(outcome.clone())),
            target: Some(Value::String("shipped".to_string())),
            unit: None,
        };
    }
    if has_any(&["no change", "nochange"]) {
        return EvaluationVerdict {
            evaluated: true,
            pass: Some(outcome == "no_change"),
            reason: "requires_no_change_outcome".to_string(),
            comparator: None,
            value: Some(Value::String(outcome.clone())),
            target: Some(Value::String("no_change".to_string())),
            unit: None,
        };
    }
    if has_any(&["web tooling ready", "web_tooling_ready", "web search ready"]) {
        return EvaluationVerdict {
            evaluated: true,
            pass: Some(web_tooling_ready),
            reason: "requires_web_tooling_ready".to_string(),
            comparator: None,
            value: Some(Value::Bool(web_tooling_ready)),
            target: Some(Value::Bool(true)),
            unit: None,
        };
    }
    if has_any(&["revert", "rollback", "undo"]) && has_any(&["no", "without", "avoid", "prevent"]) {
        return EvaluationVerdict {
            evaluated: true,
            pass: Some(outcome != "reverted"),
            reason: "requires_non_reverted_outcome".to_string(),
            comparator: None,
            value: Some(Value::String(outcome.clone())),
            target: Some(Value::String("!=reverted".to_string())),
            unit: None,
        };
    }
    if has_any(&[
        "execute",
        "executed",
        "execution",
        "run",
        "runnable",
        "exit 0",
        "success",
    ]) {
        return bool_verdict(
            "requires_execution_success",
            exec_ok,
            Value::Bool(exec_ok),
            Value::Bool(true),
        );
    }
    if has_any(&[
        "postcondition",
        "contract",
        "verify",
        "verification",
        "validated",
        "check pass",
        "checks pass",
    ]) {
        return bool_verdict(
            "requires_postconditions_pass",
            postconditions_ok,
            Value::Bool(postconditions_ok),
            Value::Bool(true),
        );
    }
    if has_any(&["dod", "impact", "delta"]) {
        return bool_verdict(
            "requires_dod_pass",
            dod_passed,
            Value::Bool(dod_passed),
            Value::Bool(true),
        );
    }
    if has_any(&["artifact", "artifacts"]) {
        let threshold = parse_first_int(&text, 1) as f64;
        let comparator = parse_comparator(&text, "gte");
        return numeric_verdict(
            "artifact_delta_check",
            &comparator,
            artifacts_delta,
            Some(threshold),
            "artifact_delta_unavailable",
            None,
        );
    }
    if has_any(&["entries", "entry", "notes"]) {
        let threshold = parse_first_int(&text, 1) as f64;
        let comparator = parse_comparator(&text, "gte");
        return numeric_verdict(
            "entry_delta_check",
            &comparator,
            entries_delta,
            Some(threshold),
            "entry_delta_unavailable",
            None,
        );
    }
    if has_any(&["revenue"]) {
        let threshold = parse_first_int(&text, 1) as f64;
        let comparator = parse_comparator(&text, "gte");
        return numeric_verdict(
            "revenue_delta_check",
            &comparator,
            revenue_delta,
            Some(threshold),
            "revenue_delta_unavailable",
            None,
        );
    }
    if metric_norm == "outreach_artifact"
        || (has_any(&["outreach"]) && has_any(&["artifact", "draft", "offer", "proposal"]))
        || (has_any(&["draft", "offer", "proposal"])
            && has_any(&[
                "build",
                "generate",
                "generated",
                "create",
                "created",
                "artifact",
            ]))
    {
        let threshold = parse_first_int(&text, 1) as f64;
        let comparator = parse_comparator(&text, "gte");
        let value = read_numeric_metric(
            context,
            &[
                "outreach_artifact",
                "outreach_artifact_count",
                "offer_draft_count",
                "proposal_draft_count",
            ],
        )
        .or(artifacts_delta);
        return numeric_verdict(
            "outreach_artifact_check",
            &comparator,
            value,
            Some(threshold),
            "outreach_artifact_unavailable",
            None,
        );
    }
    if metric_norm == "reply_or_interview_count"
        || (has_any(&["reply", "interview"]) && has_any(&["count", "signal", "response", "kpi"]))
    {
        let threshold = parse_first_int(&text, 1) as f64;
        let comparator = parse_comparator(&text, "gte");
        let value = read_numeric_metric(context, &["reply_or_interview_count"]).or_else(|| {
            let reply = read_numeric_metric(context, &["reply_count", "outreach_reply_count"])
                .unwrap_or(0.0);
            let interview =
                read_numeric_metric(context, &["interview_count", "outreach_interview_count"])
                    .unwrap_or(0.0);
            if reply > 0.0 || interview > 0.0 {
                Some(reply + interview)
            } else {
                None
            }
        });
        return numeric_verdict(
            "reply_or_interview_count_check",
            &comparator,
            value,
            Some(threshold),
            "reply_or_interview_count_unavailable",
            None,
        );
    }
    if has_any(&["token", "tokens"]) {
        let limit = parse_token_limit(&text).map(|v| v as f64);
        if limit.is_none() {
            return EvaluationVerdict {
                evaluated: false,
                pass: None,
                reason: "token_limit_missing".to_string(),
                comparator: None,
                value: None,
                target: None,
                unit: None,
            };
        }
        let comparator = parse_comparator(&text, "lte");
        return numeric_verdict(
            "token_limit_check",
            &comparator,
            effective_tokens,
            limit,
            "token_usage_unavailable",
            None,
        );
    }
    if has_any(&[
        "latency",
        "duration",
        "time",
        " ms",
        "msec",
        "millisecond",
        "second",
        " sec",
        " min",
        "minute",
    ]) {
        let limit = parse_duration_limit_ms(&text).map(|v| v as f64);
        if limit.is_none() {
            return EvaluationVerdict {
                evaluated: false,
                pass: None,
                reason: "duration_limit_missing".to_string(),
                comparator: None,
                value: None,
                target: None,
                unit: None,
            };
        }
        let comparator = parse_comparator(&text, "lte");
        return numeric_verdict(
            "duration_limit_check",
            &comparator,
            duration_ms,
            limit,
            "duration_unavailable",
            Some("ms"),
        );
    }
    if has_any(&["receipt", "evidence", "queue outcome", "logged"]) {
        return bool_verdict(
            "requires_receipt_or_outcome_log",
            queue_outcome_logged,
            Value::Bool(queue_outcome_logged),
            Value::Bool(true),
        );
    }

    EvaluationVerdict {
        evaluated: false,
        pass: None,
        reason: "unsupported_metric".to_string(),
        comparator: None,
        value: None,
        target: None,
        unit: None,
    }
}
