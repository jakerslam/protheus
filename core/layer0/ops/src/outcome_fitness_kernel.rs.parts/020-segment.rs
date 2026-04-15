fn normalize_value_currency_policy_overrides(value: Option<&Value>) -> Value {
    let Some(obj) = value.and_then(Value::as_object) else {
        return json!({
            "default_currency": Value::Null,
            "currency_overrides": {},
            "objective_overrides": {}
        });
    };
    let default_currency = normalize_value_currency_token(&as_text(obj.get("default_currency")));
    let currency_src = obj
        .get("currency_overrides")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let objective_src = obj
        .get("objective_overrides")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut currency_overrides = Map::new();
    for (raw_key, row) in currency_src {
        let currency = normalize_value_currency_token(&raw_key);
        if currency.is_empty() {
            continue;
        }
        let row_obj = row.as_object().cloned().unwrap_or_default();
        let ranking = normalize_ranking_weights(
            row_obj
                .get("ranking_weights")
                .map(|v| v as &Value)
                .or(Some(&row)),
        );
        let Some(weights) = ranking else {
            continue;
        };
        currency_overrides.insert(currency, json!({ "ranking_weights": weights }));
    }

    let mut objective_overrides = Map::new();
    for (raw_key, row) in objective_src {
        let objective_id = clean_text(raw_key, 200);
        if objective_id.is_empty() {
            continue;
        }
        let row_obj = row.as_object().cloned().unwrap_or_default();
        let ranking = normalize_ranking_weights(
            row_obj
                .get("ranking_weights")
                .map(|v| v as &Value)
                .or(Some(&row)),
        );
        let primary_currency =
            normalize_value_currency_token(&as_text(row_obj.get("primary_currency")));
        if ranking.is_none() && primary_currency.is_empty() {
            continue;
        }
        objective_overrides.insert(
            objective_id,
            json!({
                "primary_currency": if primary_currency.is_empty() { Value::Null } else { Value::String(primary_currency) },
                "ranking_weights": ranking.map(Value::Object).unwrap_or(Value::Null)
            }),
        );
    }

    json!({
        "default_currency": if default_currency.is_empty() { Value::Null } else { Value::String(default_currency) },
        "currency_overrides": currency_overrides,
        "objective_overrides": objective_overrides,
    })
}

fn read_json_safe(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn default_policy() -> Value {
    json!({
        "version": "1.0",
        "schema": {
            "id": "protheus_outcome_fitness_policy",
            "version": "1.0.0"
        },
        "strategy_policy": {
            "strategy_id": Value::Null,
            "threshold_overrides": {},
            "ranking_weights_override": Value::Null,
            "proposal_type_threshold_offsets": {},
            "promotion_policy_overrides": {},
            "promotion_policy_audit": {
                "quality_lock": {
                    "active": false,
                    "was_locked": false,
                    "stable_window_streak": 0,
                    "unstable_window_streak": 0,
                    "min_stable_windows": 0,
                    "release_unstable_windows": 0,
                    "min_realized_score": 0.0,
                    "min_quality_receipts": 0,
                    "max_insufficient_rate": 1.0
                }
            },
            "value_currency_policy_overrides": {
                "default_currency": Value::Null,
                "currency_overrides": {},
                "objective_overrides": {}
            }
        },
        "focus_policy": {
            "min_focus_score_delta": 0
        },
        "proposal_filter_policy": {
            "require_success_criteria": true,
            "min_success_criteria_count": 1
        }
    })
}

fn default_policy_path(repo_root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let root_dir = clean_text(as_text(payload.get("root_dir")), 400);
    let base_root = if root_dir.is_empty() {
        repo_root.to_path_buf()
    } else if Path::new(&root_dir).is_absolute() {
        PathBuf::from(root_dir)
    } else {
        repo_root.join(root_dir)
    };

    let override_path = clean_text(as_text(payload.get("override_path")), 400);
    if !override_path.is_empty() {
        if Path::new(&override_path).is_absolute() {
            return PathBuf::from(override_path);
        }
        return repo_root.join(override_path);
    }

    if let Ok(env_override) = std::env::var("OUTCOME_FITNESS_POLICY_PATH") {
        let env_override = clean_text(env_override, 400);
        if !env_override.is_empty() {
            if Path::new(&env_override).is_absolute() {
                return PathBuf::from(env_override);
            }
            return repo_root.join(env_override);
        }
    }

    base_root
        .join("local")
        .join("state")
        .join("adaptive")
        .join("strategy")
        .join("outcome_fitness.json")
}

fn load_outcome_fitness_policy(repo_root: &Path, payload: &Map<String, Value>) -> Value {
    let path = default_policy_path(repo_root, payload);
    let raw = read_json_safe(&path);
    let base = default_policy();
    let src = raw
        .as_ref()
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let schema = src
        .get("schema")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let strategy_policy = src
        .get("strategy_policy")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let focus_policy = src
        .get("focus_policy")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let filter_policy = src
        .get("proposal_filter_policy")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let ts_value = src
        .get("ts")
        .map(|v| clean_text(as_text(Some(v)), 120))
        .filter(|v| !v.is_empty())
        .map(Value::String)
        .unwrap_or(Value::Null);
    let realized_outcome_score = to_number(src.get("realized_outcome_score"))
        .map(|v| clamp_number(Some(&json_number(v)), 0.0, 100.0, 0.0))
        .map(json_number)
        .unwrap_or(Value::Null);
    let strategy_id = {
        let value = clean_text(as_text(strategy_policy.get("strategy_id")), 160);
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let schema_id = clean_text(as_text(schema.get("id")), 120);
    let schema_version = clean_text(as_text(schema.get("version")), 40);
    let default_version = base.get("version").and_then(Value::as_str).unwrap_or("1.0");
    let version = {
        let value = clean_text(as_text(src.get("version")), 40);
        if value.is_empty() {
            default_version.to_string()
        } else {
            value
        }
    };

    json!({
        "found": raw.is_some(),
        "path": path.to_string_lossy(),
        "ts": ts_value,
        "realized_outcome_score": realized_outcome_score,
        "strategy_policy": {
            "strategy_id": strategy_id,
            "threshold_overrides": normalize_threshold_overrides(strategy_policy.get("threshold_overrides")),
            "ranking_weights_override": normalize_ranking_weights(strategy_policy.get("ranking_weights_override")).map(Value::Object).unwrap_or(Value::Null),
            "proposal_type_threshold_offsets": normalize_proposal_type_threshold_offsets(strategy_policy.get("proposal_type_threshold_offsets")),
            "promotion_policy_overrides": normalize_promotion_policy_overrides(strategy_policy.get("promotion_policy_overrides")),
            "promotion_policy_audit": normalize_promotion_policy_audit(strategy_policy.get("promotion_policy_audit")),
            "value_currency_policy_overrides": normalize_value_currency_policy_overrides(strategy_policy.get("value_currency_policy_overrides"))
        },
        "focus_policy": {
            "min_focus_score_delta": clamp_int(focus_policy.get("min_focus_score_delta"), -20, 20, 0)
        },
        "proposal_filter_policy": {
            "require_success_criteria": filter_policy.get("require_success_criteria").and_then(Value::as_bool).unwrap_or(true),
            "min_success_criteria_count": clamp_int(filter_policy.get("min_success_criteria_count"), 0, 5, 1)
        },
        "schema": {
            "id": schema_id,
            "version": schema_version
        },
        "version": version
    })
}

fn proposal_type_threshold_offsets_for(policy: &Value, proposal_type: &str) -> Map<String, Value> {
    let type_key = normalize_proposal_type_key(proposal_type);
    if type_key.is_empty() {
        return Map::new();
    }
    let strategy_policy = policy
        .get("strategy_policy")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let table = strategy_policy
        .get("proposal_type_threshold_offsets")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let normalized_table =
        normalize_proposal_type_threshold_offsets(Some(&Value::Object(table.clone())));
    let row = normalized_table
        .get(&type_key)
        .cloned()
        .unwrap_or_else(|| json!({}));
    normalize_threshold_overrides(Some(&row))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());

    match cmd.as_str() {
        "help" | "--help" | "-h" => {
            usage();
            0
        }
        "load-policy" => {
            run_payload_command(argv, "outcome_fitness_kernel_load_policy", |payload| {
                load_outcome_fitness_policy(root, payload_obj(&payload))
            })
        }
        "normalize-threshold-overrides" => run_payload_command(
            argv,
            "outcome_fitness_kernel_normalize_threshold_overrides",
            |payload| json!({ "normalized": Value::Object(normalize_threshold_overrides(Some(&payload))) }),
        ),
        "normalize-ranking-weights" => run_payload_command(
            argv,
            "outcome_fitness_kernel_normalize_ranking_weights",
            |payload| {
                json!({
                    "normalized": normalize_ranking_weights(Some(&payload))
                        .map(Value::Object)
                        .unwrap_or(Value::Null)
                })
            },
        ),
        "normalize-proposal-type-threshold-offsets" => run_payload_command(
            argv,
            "outcome_fitness_kernel_normalize_proposal_type_threshold_offsets",
            |payload| {
                json!({
                    "normalized": Value::Object(normalize_proposal_type_threshold_offsets(Some(&payload)))
                })
            },
        ),
        "normalize-promotion-policy-overrides" => run_payload_command(
            argv,
            "outcome_fitness_kernel_normalize_promotion_policy_overrides",
            |payload| {
                json!({
                    "normalized": Value::Object(normalize_promotion_policy_overrides(Some(&payload)))
                })
            },
        ),
        "normalize-value-currency-policy-overrides" => run_payload_command(
            argv,
            "outcome_fitness_kernel_normalize_value_currency_policy_overrides",
            |payload| json!({ "normalized": normalize_value_currency_policy_overrides(Some(&payload)) }),
        ),
        "normalize-proposal-type-key" => run_payload_command(
            argv,
            "outcome_fitness_kernel_normalize_proposal_type_key",
            |payload| {
                json!({
                    "normalized": normalize_proposal_type_key(&as_text(payload_obj(&payload).get("value")))
                })
            },
        ),
        "normalize-value-currency-token" => run_payload_command(
            argv,
            "outcome_fitness_kernel_normalize_value_currency_token",
            |payload| {
                json!({
                    "normalized": normalize_value_currency_token(&as_text(payload_obj(&payload).get("value")))
                })
            },
        ),
        "proposal-type-threshold-offsets-for" => run_payload_command(
            argv,
            "outcome_fitness_kernel_proposal_type_threshold_offsets_for",
            |payload| {
                let obj = payload_obj(&payload);
                let empty = json!({});
                let policy = obj.get("policy").unwrap_or(&empty);
                let offsets =
                    proposal_type_threshold_offsets_for(policy, &as_text(obj.get("proposal_type")));
                json!({ "offsets": offsets })
            },
        ),
        _ => {
            usage();
            emit_cli_error("outcome_fitness_kernel", "unknown_command")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ranking_weights_scales_to_unit_sum() {
        let payload = json!({
            "composite": 2,
            "actionability": 1,
            "directive_fit": 1
        });
        let out = normalize_ranking_weights(Some(&payload)).expect("weights");
        let sum = out
            .values()
            .filter_map(|v| v.as_f64())
            .fold(0.0, |acc, v| acc + v);
        assert!((sum - 1.0).abs() < 0.00001);
        assert_eq!(
            out.get("composite")
                .and_then(Value::as_f64)
                .unwrap_or_default(),
            0.5
        );
    }

    #[test]
    fn load_policy_normalizes_thresholds_and_offsets() {
        let repo_root = PathBuf::from("/tmp/fake-repo");
        let payload = json!({
            "root_dir": "/tmp/fake-repo/client",
            "override_path": "/tmp/fake-repo/client/local/state/adaptive/strategy/outcome_fitness.json"
        });
        let path = default_policy_path(&repo_root, payload_obj(&payload));
        assert!(path.ends_with("client/local/state/adaptive/strategy/outcome_fitness.json"));
        let normalized = proposal_type_threshold_offsets_for(
            &json!({
                "strategy_policy": {
                    "proposal_type_threshold_offsets": {
                        "Code Change!!": {
                            "min_signal_quality": 0.8
                        }
                    }
                }
            }),
            "Code Change!!",
        );
        assert_eq!(
            normalized
                .get("min_signal_quality")
                .and_then(Value::as_f64)
                .unwrap_or_default(),
            0.8
        );
    }
}
