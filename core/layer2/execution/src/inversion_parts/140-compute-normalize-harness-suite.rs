fn normalize_harness_difficulty(value: &str) -> String {
    let token = normalize_token_runtime(value, 24);
    match token.as_str() {
        "" => "medium".to_string(),
        "low" | "easy" | "novice" => "low".to_string(),
        "medium" | "med" | "mid" | "developing" => "medium".to_string(),
        "high" | "hard" | "advanced" | "expert" => "high".to_string(),
        _ => token,
    }
}

pub fn compute_normalize_harness_suite(
    input: &NormalizeHarnessSuiteInput,
) -> NormalizeHarnessSuiteOutput {
    let src = input
        .raw_suite
        .as_ref()
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let fallback = input
        .base_suite
        .as_ref()
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let rows = if src.is_empty() { fallback } else { src };
    let mut out = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut seen_objectives: HashSet<String> = HashSet::new();
    for (idx, row) in rows.iter().enumerate() {
        let item = row.as_object();
        let default_id = format!("imh_{}", idx + 1);
        let id_raw = value_to_string(item.and_then(|m| m.get("id")));
        let mut id = normalize_token_runtime(
            if id_raw.is_empty() {
                default_id.as_str()
            } else {
                id_raw.as_str()
            },
            80,
        );
        if id.is_empty() {
            id = default_id;
        }
        let objective =
            clean_text_runtime(&value_to_string(item.and_then(|m| m.get("objective"))), 280);
        if objective.is_empty() {
            continue;
        }
        if !seen_ids.insert(id.clone()) {
            continue;
        }
        let objective_key = normalize_token_runtime(objective.as_str(), 120);
        if !objective_key.is_empty() && !seen_objectives.insert(objective_key) {
            continue;
        }
        let impact = compute_normalize_impact(&NormalizeImpactInput {
            value: Some(value_to_string(item.and_then(|m| m.get("impact")))),
        })
        .value;
        let target = compute_normalize_target(&NormalizeTargetInput {
            value: Some(value_to_string(item.and_then(|m| m.get("target")))),
        })
        .value;
        let difficulty =
            normalize_harness_difficulty(&value_to_string(item.and_then(|m| m.get("difficulty"))));
        out.push(json!({
            "id": id,
            "objective": objective,
            "impact": impact,
            "target": target,
            "difficulty": difficulty
        }));
    }
    NormalizeHarnessSuiteOutput { suite: out }
}
pub fn compute_load_harness_state(input: &LoadHarnessStateInput) -> LoadHarnessStateOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let src = compute_read_json(&ReadJsonInput {
        file_path: input.file_path.clone(),
        fallback: Some(Value::Null),
    })
    .value;
    let row = src.as_object();
    let updated_at = {
        let value = value_to_string(row.and_then(|m| m.get("updated_at")));
        if value.is_empty() {
            now_iso.clone()
        } else {
            value
        }
    };
    let last_run_ts = {
        let value = value_to_string(row.and_then(|m| m.get("last_run_ts")));
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let cursor = parse_number_like(row.and_then(|m| m.get("cursor")))
        .unwrap_or(0.0)
        .floor() as i64;
    LoadHarnessStateOutput {
        state: json!({
            "schema_id": "inversion_maturity_harness_state",
            "schema_version": "1.0",
            "updated_at": updated_at,
            "last_run_ts": last_run_ts,
            "cursor": cursor.clamp(0, 1_000_000)
        }),
    }
}

pub fn compute_save_harness_state(input: &SaveHarnessStateInput) -> SaveHarnessStateOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let src = input.state.as_ref().and_then(|v| v.as_object());
    let last_run_ts = {
        let value = value_to_string(src.and_then(|m| m.get("last_run_ts")));
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let cursor = parse_number_like(src.and_then(|m| m.get("cursor")))
        .unwrap_or(0.0)
        .floor() as i64;
    let out = json!({
        "schema_id": "inversion_maturity_harness_state",
        "schema_version": "1.0",
        "updated_at": now_iso,
        "last_run_ts": last_run_ts,
        "cursor": cursor.clamp(0, 1_000_000)
    });
    let _ = compute_write_json_atomic(&WriteJsonAtomicInput {
        file_path: input.file_path.clone(),
        value: Some(out.clone()),
    });
    SaveHarnessStateOutput { state: out }
}

pub fn compute_load_first_principle_lock_state(
    input: &LoadFirstPrincipleLockStateInput,
) -> LoadFirstPrincipleLockStateOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let src = compute_read_json(&ReadJsonInput {
        file_path: input.file_path.clone(),
        fallback: Some(Value::Null),
    })
    .value;
    let row = src.as_object();
    let updated_at = {
        let value = value_to_string(row.and_then(|m| m.get("updated_at")));
        if value.is_empty() {
            now_iso.clone()
        } else {
            value
        }
    };
    let locks = row
        .and_then(|m| m.get("locks"))
        .and_then(|v| v.as_object())
        .map(|m| Value::Object(m.clone()))
        .unwrap_or_else(|| json!({}));
    LoadFirstPrincipleLockStateOutput {
        state: json!({
            "schema_id": "inversion_first_principle_lock_state",
            "schema_version": "1.0",
            "updated_at": updated_at,
            "locks": locks
        }),
    }
}

pub fn compute_save_first_principle_lock_state(
    input: &SaveFirstPrincipleLockStateInput,
) -> SaveFirstPrincipleLockStateOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let src = input.state.as_ref().and_then(|v| v.as_object());
    let locks = src
        .and_then(|m| m.get("locks"))
        .and_then(|v| v.as_object())
        .map(|m| Value::Object(m.clone()))
        .unwrap_or_else(|| json!({}));
    let out = json!({
        "schema_id": "inversion_first_principle_lock_state",
        "schema_version": "1.0",
        "updated_at": now_iso,
        "locks": locks
    });
    let _ = compute_write_json_atomic(&WriteJsonAtomicInput {
        file_path: input.file_path.clone(),
        value: Some(out.clone()),
    });
    SaveFirstPrincipleLockStateOutput { state: out }
}

pub fn compute_load_observer_approvals(
    input: &LoadObserverApprovalsInput,
) -> LoadObserverApprovalsOutput {
    let mut seen: HashSet<String> = HashSet::new();
    let mut rows = compute_read_jsonl(&ReadJsonlInput {
        file_path: input.file_path.clone(),
    })
    .rows
    .into_iter()
    .filter_map(|row| {
        let item = row.as_object()?;
        let ts = clean_text_runtime(&value_to_string(item.get("ts")), 64);
        let target = compute_normalize_target(&NormalizeTargetInput {
            value: Some(value_to_string(item.get("target"))),
        })
        .value;
        let observer_id = compute_normalize_observer_id(&NormalizeObserverIdInput {
            value: Some(if item.get("observer_id").is_some() {
                value_to_string(item.get("observer_id"))
            } else {
                value_to_string(item.get("observerId"))
            }),
        })
        .value;
        let note = clean_text_runtime(&value_to_string(item.get("note")), 280);
        if ts.is_empty() || observer_id.is_empty() {
            return None;
        }
        let dedupe_key = format!("{ts}|{target}|{observer_id}");
        if !seen.insert(dedupe_key) {
            return None;
        }
        Some(json!({
            "ts": ts,
            "target": target,
            "observer_id": observer_id,
            "note": note
        }))
    })
    .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        let a_obj = a.as_object();
        let b_obj = b.as_object();
        let a_ts = value_to_string(a_obj.and_then(|m| m.get("ts")));
        let b_ts = value_to_string(b_obj.and_then(|m| m.get("ts")));
        let a_observer = value_to_string(a_obj.and_then(|m| m.get("observer_id")));
        let b_observer = value_to_string(b_obj.and_then(|m| m.get("observer_id")));
        let a_target = value_to_string(a_obj.and_then(|m| m.get("target")));
        let b_target = value_to_string(b_obj.and_then(|m| m.get("target")));
        a_ts.cmp(&b_ts)
            .then_with(|| a_observer.cmp(&b_observer))
            .then_with(|| a_target.cmp(&b_target))
    });
    LoadObserverApprovalsOutput { rows }
}

pub fn compute_append_observer_approval(
    input: &AppendObserverApprovalInput,
) -> AppendObserverApprovalOutput {
    let row = json!({
        "ts": input.now_iso.clone().unwrap_or_else(now_iso_runtime),
        "type": "inversion_live_graduation_observer_approval",
        "target": compute_normalize_target(&NormalizeTargetInput {
            value: input.target.clone()
        }).value,
        "observer_id": compute_normalize_observer_id(&NormalizeObserverIdInput {
            value: input.observer_id.clone()
        }).value,
        "note": clean_text_runtime(input.note.as_deref().unwrap_or(""), 280)
    });
    let _ = compute_append_jsonl(&AppendJsonlInput {
        file_path: input.file_path.clone(),
        row: Some(row.clone()),
    });
    AppendObserverApprovalOutput { row }
}

pub fn compute_count_observer_approvals(
    input: &CountObserverApprovalsInput,
) -> CountObserverApprovalsOutput {
    let window_days = parse_number_like(input.window_days.as_ref())
        .unwrap_or(90.0)
        .floor() as i64;
    let window_days = window_days.clamp(1, 3650);
    let cutoff = Utc::now().timestamp_millis() - (window_days * 24 * 60 * 60 * 1000);
    let target = compute_normalize_target(&NormalizeTargetInput {
        value: input.target.clone(),
    })
    .value;
    let rows = compute_load_observer_approvals(&LoadObserverApprovalsInput {
        file_path: input.file_path.clone(),
    })
    .rows;
    let mut seen: HashSet<String> = HashSet::new();
    for row in rows {
        let item = row.as_object();
        let row_target = compute_normalize_target(&NormalizeTargetInput {
            value: Some(value_to_string(item.and_then(|m| m.get("target")))),
        })
        .value;
        if row_target != target {
            continue;
        }
        let ts = value_to_string(item.and_then(|m| m.get("ts")));
        if parse_ts_ms_runtime(&ts) < cutoff {
            continue;
        }
        let observer_id = compute_normalize_observer_id(&NormalizeObserverIdInput {
            value: Some(value_to_string(item.and_then(|m| m.get("observer_id")))),
        })
        .value;
        if observer_id.is_empty() {
            continue;
        }
        seen.insert(observer_id);
    }
    CountObserverApprovalsOutput {
        count: seen.len() as i64,
    }
}

pub fn compute_ensure_correspondence_file(
    input: &EnsureCorrespondenceFileInput,
) -> EnsureCorrespondenceFileOutput {
    let file_path = input.file_path.as_deref().unwrap_or("").trim();
    if file_path.is_empty() {
        return EnsureCorrespondenceFileOutput { ok: true };
    }
    let path = Path::new(file_path);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if path.exists() {
        return EnsureCorrespondenceFileOutput { ok: true };
    }
    let header = input
        .header
        .clone()
        .unwrap_or_else(|| "# Shadow Conclave Correspondence\n\n".to_string());
    let _ = fs::write(path, header);
    EnsureCorrespondenceFileOutput { ok: true }
}

fn compute_maturity_score_runtime(state: &Value, policy: &Value) -> Value {
    let stats = state
        .as_object()
        .and_then(|m| m.get("stats"))
        .and_then(|v| v.as_object());
    let maturity = policy
        .as_object()
        .and_then(|m| m.get("maturity"))
        .and_then(|v| v.as_object());
    let weights = maturity
        .and_then(|m| m.get("score_weights"))
        .and_then(|v| v.as_object());
    let bands = maturity
        .and_then(|m| m.get("bands"))
        .and_then(|v| v.as_object());

    let total = parse_number_like(stats.and_then(|m| m.get("total_tests")))
        .unwrap_or(0.0)
        .max(0.0);
    let passed = parse_number_like(stats.and_then(|m| m.get("passed_tests")))
        .unwrap_or(0.0)
        .max(0.0);
    let destructive = parse_number_like(stats.and_then(|m| m.get("destructive_failures")))
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
    let target_test_count = parse_number_like(maturity.and_then(|m| m.get("target_test_count")))
        .unwrap_or(40.0)
        .max(1.0);
    let experience = (total / target_test_count).min(1.0);

    let weight_pass = parse_number_like(weights.and_then(|m| m.get("pass_rate"))).unwrap_or(0.0);
    let weight_non_destructive =
        parse_number_like(weights.and_then(|m| m.get("non_destructive_rate"))).unwrap_or(0.0);
    let weight_experience =
        parse_number_like(weights.and_then(|m| m.get("experience"))).unwrap_or(0.0);
    let weight_total = (weight_pass + weight_non_destructive + weight_experience).max(0.0001);
    let score = clamp_number(
        ((pass_rate * weight_pass)
            + (non_destructive_rate * weight_non_destructive)
            + (experience * weight_experience))
            / weight_total,
        0.0,
        1.0,
    );

    let novice = parse_number_like(bands.and_then(|m| m.get("novice"))).unwrap_or(0.25);
    let developing = parse_number_like(bands.and_then(|m| m.get("developing"))).unwrap_or(0.45);
    let mature = parse_number_like(bands.and_then(|m| m.get("mature"))).unwrap_or(0.65);
    let seasoned = parse_number_like(bands.and_then(|m| m.get("seasoned"))).unwrap_or(0.82);
    let band = if score < novice {
        "novice"
    } else if score < developing {
        "developing"
    } else if score < mature {
        "mature"
    } else if score < seasoned {
        "seasoned"
    } else {
        "legendary"
    };
    json!({
        "score": (score * 1_000_000.0).round() / 1_000_000.0,
        "band": band,
        "pass_rate": (pass_rate * 1_000_000.0).round() / 1_000_000.0,
        "non_destructive_rate": (non_destructive_rate * 1_000_000.0).round() / 1_000_000.0,
        "experience": (experience * 1_000_000.0).round() / 1_000_000.0
    })
}

pub fn compute_load_maturity_state(input: &LoadMaturityStateInput) -> LoadMaturityStateOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let src = compute_read_json(&ReadJsonInput {
        file_path: input.file_path.clone(),
        fallback: Some(Value::Null),
    })
    .value;
    let policy = input.policy.clone().unwrap_or_else(|| json!({}));
    let mut state = if src.is_object() {
        src
    } else {
        compute_default_maturity_state(&DefaultMaturityStateInput {}).state
    };
    if !state.is_object() {
        state = compute_default_maturity_state(&DefaultMaturityStateInput {}).state;
    }
    let computed = compute_maturity_score_runtime(&state, &policy);
    if let Some(obj) = state.as_object_mut() {
        let updated_at_value = {
            let value = value_to_string(obj.get("updated_at"))
                .chars()
                .take(64)
                .collect::<String>();
            if value.is_empty() {
                now_iso.clone()
            } else {
                value
            }
        };
        obj.insert("updated_at".to_string(), Value::String(updated_at_value));
        obj.insert(
            "score".to_string(),
            computed.get("score").cloned().unwrap_or_else(|| json!(0)),
        );
        obj.insert(
            "band".to_string(),
            computed
                .get("band")
                .cloned()
                .unwrap_or_else(|| json!("novice")),
        );
    }
    LoadMaturityStateOutput { state, computed }
}

pub fn compute_save_maturity_state(input: &SaveMaturityStateInput) -> SaveMaturityStateOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let mut state = if input.state.as_ref().map(|v| v.is_object()).unwrap_or(false) {
        input.state.clone().unwrap_or_else(|| json!({}))
    } else {
        compute_default_maturity_state(&DefaultMaturityStateInput {}).state
    };
    let policy = input.policy.clone().unwrap_or_else(|| json!({}));
    let computed = compute_maturity_score_runtime(&state, &policy);
    if let Some(obj) = state.as_object_mut() {
        obj.insert("updated_at".to_string(), Value::String(now_iso));
        obj.insert(
            "score".to_string(),
            computed.get("score").cloned().unwrap_or_else(|| json!(0)),
        );
        obj.insert(
            "band".to_string(),
            computed
                .get("band")
                .cloned()
                .unwrap_or_else(|| json!("novice")),
        );
    }
    let _ = compute_write_json_atomic(&WriteJsonAtomicInput {
        file_path: input.file_path.clone(),
        value: Some(state.clone()),
    });
    SaveMaturityStateOutput { state, computed }
}
