pub fn evaluate_success_criteria_value(
    proposal: Option<&Value>,
    context: Option<&Value>,
    policy: Option<&Value>,
) -> Value {
    let policy = policy.unwrap_or(&Value::Null);
    let context = context.unwrap_or(&Value::Null);
    let policy_obj = policy.as_object();
    let context_obj = context.as_object();
    let capability_key = policy_obj
        .and_then(|map| map.get("capability_key"))
        .map(js_like_string)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            context_obj
                .and_then(|map| map.get("capability_key"))
                .map(js_like_string)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default();
    let required = policy_obj
        .and_then(|map| map.get("required"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let min_count = policy_obj
        .and_then(|map| map.get("min_count"))
        .and_then(|v| as_f64(Some(v)))
        .map(|v| v.floor() as i64)
        .unwrap_or(1)
        .clamp(0, 10);
    let max_unknown_count = policy_obj
        .and_then(|map| map.get("max_unknown_count"))
        .and_then(|v| as_f64(Some(v)))
        .map(|v| v.floor() as i64)
        .unwrap_or(i64::MAX)
        .clamp(0, 10_000);
    let max_unknown_ratio = policy_obj
        .and_then(|map| map.get("max_unknown_ratio"))
        .and_then(|v| as_f64(Some(v)))
        .map(|v| v.clamp(0.0, 1.0));
    let contract = capability_metric_contract(&capability_key);
    let enable_contract_backfill = policy_obj
        .and_then(|map| map.get("enable_contract_backfill"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let fail_on_contract_violation = policy_obj
        .and_then(|map| map.get("fail_on_contract_violation"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let enforce_contract = contract.enforced
        && policy_obj
            .and_then(|map| map.get("enforce_contract"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
    let enforce_min_supported = contract.enforced
        && policy_obj
            .and_then(|map| map.get("enforce_min_supported"))
            .and_then(Value::as_bool)
            .unwrap_or(true);

    let rows_raw = parse_success_criteria_rows_from_proposal(proposal, &capability_key);
    let (rows, contract_backfill_count) = if enable_contract_backfill {
        backfill_contract_safe_rows(&rows_raw, &contract, min_count)
    } else {
        (rows_raw, 0)
    };

    let mut results = Vec::<EvaluateCheck>::new();
    for (idx, row) in rows.iter().enumerate() {
        let metric_norm = row.metric.to_ascii_lowercase().replace([' ', '-'], "_");
        let blocked_by_contract = enforce_contract
            && !metric_norm.is_empty()
            && contract
                .allowed_metrics
                .as_ref()
                .map(|allowed| !allowed.contains(&metric_norm))
                .unwrap_or(false);
        let verdict = if blocked_by_contract {
            EvaluationVerdict {
                evaluated: false,
                pass: None,
                reason: "metric_not_allowed_for_capability".to_string(),
                comparator: None,
                value: None,
                target: None,
                unit: None,
            }
        } else {
            evaluate_row(row, context)
        };
        results.push(EvaluateCheck {
            index: (idx + 1) as u32,
            source: row.source.clone(),
            metric: row.metric.clone(),
            target: row.target.chars().take(180).collect(),
            evaluated: verdict.evaluated,
            pass: verdict.pass,
            reason: verdict.reason,
            comparator: verdict.comparator,
            value: verdict.value,
            threshold: verdict.target,
            unit: verdict.unit,
        });
    }

    let evaluated_count = results.iter().filter(|row| row.evaluated).count() as i64;
    let passed_count = results.iter().filter(|row| row.pass == Some(true)).count() as i64;
    let failed_rows = results
        .iter()
        .filter(|row| row.pass == Some(false))
        .collect::<Vec<_>>();
    let failed_count = failed_rows.len() as i64;
    let unknown_count = results.len() as i64 - evaluated_count;
    let unknown_ratio = if results.is_empty() {
        0.0
    } else {
        unknown_count as f64 / results.len() as f64
    };
    let unsupported_count = results
        .iter()
        .filter(|row| row.reason == "unsupported_metric")
        .count() as i64;
    let contract_not_allowed_count = results
        .iter()
        .filter(|row| row.reason == "metric_not_allowed_for_capability")
        .count() as i64;
    let structurally_supported_count =
        (results.len() as i64 - unsupported_count - contract_not_allowed_count).max(0);

    let mut passed = true;
    let mut primary_failure: Option<String> = None;
    if required {
        if rows.len() < min_count as usize {
            passed = false;
            primary_failure = Some("success_criteria_count_below_min".to_string());
        } else if passed_count < min_count {
            passed = false;
            primary_failure = Some(if let Some(first) = failed_rows.first() {
                format!("success_criteria_failed:{}", first.reason)
            } else {
                "success_criteria_pass_count_below_min".to_string()
            });
        } else if failed_count > 0 {
            passed = false;
            if let Some(first) = failed_rows.first() {
                primary_failure = Some(format!("success_criteria_failed:{}", first.reason));
            }
        }
    } else if failed_count > 0 {
        passed = false;
        if let Some(first) = failed_rows.first() {
            primary_failure = Some(format!("success_criteria_failed:{}", first.reason));
        }
    }

    if enforce_contract && fail_on_contract_violation && contract_not_allowed_count > 0 {
        passed = false;
        primary_failure =
            Some("success_criteria_failed:metric_not_allowed_for_capability".to_string());
    } else if enforce_min_supported && required && structurally_supported_count < min_count {
        passed = false;
        primary_failure =
            Some("success_criteria_failed:insufficient_supported_metrics".to_string());
    }
    if passed && required && unknown_count > max_unknown_count {
        passed = false;
        primary_failure = Some("success_criteria_failed:unknown_metric_budget_exceeded".to_string());
    }
    if passed
        && required
        && max_unknown_ratio
            .map(|budget| unknown_ratio > budget)
            .unwrap_or(false)
    {
        passed = false;
        primary_failure =
            Some("success_criteria_failed:unknown_metric_ratio_budget_exceeded".to_string());
    }

    let violation_rows = results
        .iter()
        .filter(|row| {
            row.reason == "unsupported_metric" || row.reason == "metric_not_allowed_for_capability"
        })
        .take(12)
        .map(|row| {
            json!({
                "index": row.index,
                "metric": row.metric,
                "reason": row.reason,
            })
        })
        .collect::<Vec<_>>();

    let allowed_metrics = contract
        .allowed_metrics
        .as_ref()
        .map(|set| {
            let mut rows = set.iter().cloned().collect::<Vec<_>>();
            rows.sort();
            rows
        })
        .unwrap_or_default();

    json!({
        "required": required,
        "min_count": min_count,
        "total_count": rows.len(),
        "evaluated_count": evaluated_count,
        "passed_count": passed_count,
        "failed_count": failed_count,
        "unknown_count": unknown_count,
        "unknown_ratio": round3(unknown_ratio),
        "unsupported_count": unsupported_count,
        "contract_not_allowed_count": contract_not_allowed_count,
        "structurally_supported_count": structurally_supported_count,
        "contract_backfill_count": contract_backfill_count,
        "pass_rate": if evaluated_count > 0 { Some(((passed_count as f64 / evaluated_count as f64) * 1000.0).round() / 1000.0) } else { None },
        "passed": passed,
        "primary_failure": primary_failure,
        "contract": {
            "capability_key": contract.capability_key,
            "enforced": enforce_contract,
            "fail_on_violation": fail_on_contract_violation,
            "min_supported_enforced": enforce_min_supported,
            "backfill_enabled": enable_contract_backfill,
            "backfill_count": contract_backfill_count,
            "allowed_metrics": allowed_metrics,
            "unsupported_count": unsupported_count,
            "not_allowed_count": contract_not_allowed_count,
            "structurally_supported_count": structurally_supported_count,
            "unknown_count": unknown_count,
            "unknown_ratio": round3(unknown_ratio),
            "unknown_budget_count": if max_unknown_count == i64::MAX { Value::Null } else { Value::from(max_unknown_count) },
            "unknown_budget_ratio": max_unknown_ratio.map(round3),
            "violation_count": violation_rows.len(),
            "violations": violation_rows,
        },
        "checks": results.into_iter().take(12).collect::<Vec<_>>(),
    })
}

pub fn run(_root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    match command.as_str() {
        "status" => {
            print_json_line(&cli_receipt(
                "success_criteria_kernel_status",
                json!({
                    "domain": "success-criteria-kernel",
                    "commands": ["status", "parse-rows", "evaluate"],
                }),
            ));
            0
        }
        "parse-rows" => {
            let payload = match load_payload(argv) {
                Ok(payload) => payload,
                Err(err) => {
                    print_json_line(&cli_error("success_criteria_kernel_parse_rows_error", &err));
                    return 1;
                }
            };
            let input = match serde_json::from_value::<ParseRowsPayload>(payload) {
                Ok(value) => value,
                Err(err) => {
                    print_json_line(&cli_error(
                        "success_criteria_kernel_parse_rows_error",
                        &format!("success_criteria_kernel_parse_rows_payload_invalid:{err}"),
                    ));
                    return 1;
                }
            };
            let rows = parse_success_criteria_rows_from_proposal(
                input.proposal.as_ref(),
                &input.capability_key.unwrap_or_default(),
            );
            print_json_line(&cli_receipt(
                "success_criteria_kernel_parse_rows",
                json!({ "rows": rows }),
            ));
            0
        }
        "evaluate" => {
            let payload = match load_payload(argv) {
                Ok(payload) => payload,
                Err(err) => {
                    print_json_line(&cli_error("success_criteria_kernel_evaluate_error", &err));
                    return 1;
                }
            };
            let input = match serde_json::from_value::<EvaluatePayload>(payload) {
                Ok(value) => value,
                Err(err) => {
                    print_json_line(&cli_error(
                        "success_criteria_kernel_evaluate_error",
                        &format!("success_criteria_kernel_evaluate_payload_invalid:{err}"),
                    ));
                    return 1;
                }
            };
            let result = evaluate_success_criteria_value(
                input.proposal.as_ref(),
                input.context.as_ref(),
                input.policy.as_ref(),
            );
            print_json_line(&cli_receipt(
                "success_criteria_kernel_evaluate",
                json!({ "result": result }),
            ));
            0
        }
        _ => {
            usage();
            print_json_line(&cli_error(
                "success_criteria_kernel_error",
                &format!("success_criteria_kernel_unknown_command:{command}"),
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_success_criteria_rows_supports_capability_remap_and_dedupe() {
        let proposal = json!({
            "success_criteria": [
                {"metric": "reply_or_interview_count", "target": ">=2"},
                {"metric": "reply_or_interview_count", "target": ">=2"},
                "postconditions pass within 24h"
            ],
            "action_spec": {
                "verify": ["receipt logged"]
            }
        });
        let rows =
            parse_success_criteria_rows_from_proposal(Some(&proposal), "proposal:internal_patch");
        assert!(rows.iter().any(|row| row.metric == "artifact_count"));
        assert!(rows.iter().any(|row| row.metric == "postconditions_ok"));
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn evaluate_success_criteria_fails_closed_on_numeric_overrun() {
        let proposal = json!({
            "success_criteria": [
                {"metric": "artifact_count", "target": ">=1 artifact"},
                {"metric": "token_usage", "target": "tokens <= 500"}
            ]
        });
        let context = json!({
            "exec_ok": true,
            "postconditions_ok": true,
            "queue_outcome_logged": true,
            "dod_diff": {
                "artifacts_delta": 2,
                "entries_delta": 0,
                "revenue_actions_delta": 0
            },
            "token_usage": {
                "effective_tokens": 900
            }
        });
        let policy = json!({
            "capability_key": "proposal:internal_patch",
            "required": true,
            "min_count": 2
        });
        let out = evaluate_success_criteria_value(Some(&proposal), Some(&context), Some(&policy));
        assert_eq!(out.get("passed").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("primary_failure").and_then(Value::as_str),
            Some("success_criteria_failed:token_limit_check")
        );
        assert_eq!(
            out.get("contract_not_allowed_count")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(out.get("passed_count").and_then(Value::as_i64), Some(1));
    }
}
