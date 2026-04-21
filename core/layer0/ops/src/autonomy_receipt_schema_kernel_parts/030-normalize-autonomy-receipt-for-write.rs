
fn normalize_autonomy_receipt_for_write(receipt: Option<&Value>) -> Value {
    let mut src = receipt
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let intent = src
        .get("intent")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let verification_src = src
        .get("verification")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut checks = verification_src
        .get("checks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let name = short_text(row.get("name"), 80);
            if name.is_empty() {
                None
            } else {
                Some(json!({
                    "name": name,
                    "pass": row.get("pass").and_then(Value::as_bool).unwrap_or(false)
                }))
            }
        })
        .collect::<Vec<_>>();
    let mut failed_set = std::collections::BTreeSet::new();
    for row in verification_src
        .get("failed")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let token = short_text(Some(&row), 80);
        if !token.is_empty() {
            failed_set.insert(token);
        }
    }

    let policy = intent
        .get("success_criteria_policy")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let required = policy
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let min_count = {
        let raw = clamp_count(policy.get("min_count"));
        if raw > 0 {
            raw
        } else if required {
            1
        } else {
            0
        }
    };
    let success_idx = checks
        .iter()
        .position(|row| row.get("name").and_then(Value::as_str) == Some("success_criteria_met"));
    let success_check_pass = success_idx
        .and_then(|idx| checks.get(idx))
        .and_then(|row| row.get("pass"))
        .and_then(Value::as_bool);
    let criteria_in = verification_src.get("success_criteria");
    let criteria = if criteria_in.and_then(Value::as_object).is_some() {
        to_success_criteria_record(
            criteria_in,
            &Map::from_iter([
                ("required".to_string(), Value::Bool(required)),
                ("min_count".to_string(), Value::from(min_count)),
            ]),
        )
    } else {
        synthesize_success_criteria(required, min_count, success_check_pass)
    };
    let criteria_obj = criteria.as_object().cloned().unwrap_or_default();
    let criteria_pass = if criteria_obj
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        criteria_obj
            .get("passed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || criteria_obj
                .get("deferred_pending")
                .and_then(Value::as_bool)
                .unwrap_or(false)
    } else {
        true
    };
    if let Some(idx) = success_idx {
        checks[idx] = json!({ "name": "success_criteria_met", "pass": criteria_pass });
    } else {
        checks.push(json!({ "name": "success_criteria_met", "pass": criteria_pass }));
    }
    if criteria_pass {
        failed_set.remove("success_criteria_met");
    } else {
        failed_set.insert("success_criteria_met".to_string());
    }
    let primary_failure_raw = if !criteria_pass {
        let from_criteria = short_text(criteria_obj.get("primary_failure"), 180);
        if !from_criteria.is_empty() {
            from_criteria
        } else {
            let from_verification = short_text(verification_src.get("primary_failure"), 180);
            if !from_verification.is_empty() {
                from_verification
            } else {
                "success_criteria_failed".to_string()
            }
        }
    } else {
        short_text(verification_src.get("primary_failure"), 180)
    };
    let mut reasons = failed_set
        .iter()
        .cloned()
        .map(Value::String)
        .collect::<Vec<_>>();
    if !primary_failure_raw.is_empty() {
        reasons.push(Value::String(primary_failure_raw.clone()));
    }
    let taxonomy = normalize_reason_list(&reasons);
    let passed = failed_set.is_empty();
    let normalized_verification = json!({
        "checks": checks,
        "failed": failed_set.into_iter().collect::<Vec<_>>(),
        "passed": passed,
        "primary_failure": if primary_failure_raw.is_empty() { Value::Null } else { Value::String(primary_failure_raw.clone()) },
        "primary_failure_taxonomy": taxonomy.first().cloned(),
        "failed_reason_taxonomy": taxonomy,
        "success_criteria": criteria,
    });
    src.insert("verification".to_string(), normalized_verification);
    Value::Object(src)
}

fn success_criteria_from_receipt(receipt: Option<&Value>) -> Value {
    let normalized = normalize_autonomy_receipt_for_write(receipt);
    normalized
        .get("verification")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("success_criteria"))
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    match cmd.as_str() {
        "help" | "--help" | "-h" => {
            usage();
            0
        }
        other => {
            let payload = match payload_json(argv) {
                Ok(value) => value,
                Err(err) => {
                    print_json_line(&cli_error("autonomy_receipt_schema_kernel", &err));
                    return 1;
                }
            };
            let obj = payload_obj(&payload);
            let out = match other {
                "to-success-criteria-record" => {
                    let fallback = obj
                        .get("fallback")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    json!({ "record": to_success_criteria_record(obj.get("criteria"), &fallback) })
                }
                "with-success-criteria-verification" => {
                    let options = obj
                        .get("options")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    json!({ "verification": with_success_criteria_verification(obj.get("baseVerification").or_else(|| obj.get("base_verification")), obj.get("successCriteria").or_else(|| obj.get("success_criteria")), &options) })
                }
                "normalize-receipt" => {
                    json!({ "receipt": normalize_autonomy_receipt_for_write(obj.get("receipt")) })
                }
                "success-criteria-from-receipt" => {
                    json!({ "success_criteria": success_criteria_from_receipt(obj.get("receipt")) })
                }
                _ => {
                    usage();
                    print_json_line(&cli_error(
                        "autonomy_receipt_schema_kernel",
                        "unknown_command",
                    ));
                    return 1;
                }
            };
            print_json_line(&cli_receipt(
                &format!("autonomy_receipt_schema_kernel_{other}"),
                out,
            ));
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_criteria_record_uses_fallbacks() {
        let out = to_success_criteria_record(
            None,
            &Map::from_iter([
                ("required".to_string(), Value::Bool(true)),
                ("min_count".to_string(), Value::from(2)),
            ]),
        );
        assert_eq!(out.get("required").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("min_count").and_then(Value::as_i64), Some(2));
    }

    #[test]
    fn normalize_receipt_synthesizes_missing_success_criteria() {
        let out = normalize_autonomy_receipt_for_write(Some(&json!({
            "intent": { "success_criteria_policy": { "required": true, "min_count": 1 } },
            "verification": { "checks": [], "failed": [] }
        })));
        let criteria = out
            .get("verification")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("success_criteria"))
            .cloned()
            .unwrap_or(Value::Null);
        assert_eq!(
            criteria.get("synthesized").and_then(Value::as_bool),
            Some(true)
        );
    }
}
