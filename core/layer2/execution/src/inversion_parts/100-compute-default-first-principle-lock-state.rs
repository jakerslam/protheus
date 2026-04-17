fn normalize_objective_seed(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_ws = false;
    for ch in clean_text_runtime(raw, 240).chars() {
        if ch.is_whitespace() {
            if prev_ws {
                continue;
            }
            prev_ws = true;
            out.push(' ');
        } else {
            prev_ws = false;
            out.push(ch.to_ascii_lowercase());
        }
    }
    let out = out.trim().to_string();
    if out.is_empty() {
        "unknown_objective".to_string()
    } else {
        out
    }
}

fn has_nonempty_flag(value: Option<&str>) -> bool {
    value.map(|v| !v.trim().is_empty()).unwrap_or(false)
}

pub fn compute_default_first_principle_lock_state(
    _input: &DefaultFirstPrincipleLockStateInput,
) -> DefaultFirstPrincipleLockStateOutput {
    DefaultFirstPrincipleLockStateOutput {
        state: json!({
            "schema_id": "inversion_first_principle_lock_state",
            "schema_version": "1.0",
            "updated_at": now_iso_runtime(),
            "locks": {}
        }),
    }
}

pub fn compute_default_maturity_state(
    _input: &DefaultMaturityStateInput,
) -> DefaultMaturityStateOutput {
    DefaultMaturityStateOutput {
        state: json!({
            "schema_id": "inversion_maturity_state",
            "schema_version": "1.0",
            "updated_at": now_iso_runtime(),
            "stats": {
                "total_tests": 0,
                "passed_tests": 0,
                "failed_tests": 0,
                "safe_failures": 0,
                "destructive_failures": 0
            },
            "recent_tests": [],
            "score": 0,
            "band": "novice"
        }),
    }
}

pub fn compute_principle_key_for_session(
    input: &PrincipleKeyForSessionInput,
) -> PrincipleKeyForSessionOutput {
    let objective_part = normalize_objective_seed(
        input
            .objective_id
            .as_deref()
            .or(input.objective.as_deref())
            .unwrap_or(""),
    );
    let mut hasher = Sha256::new();
    hasher.update(objective_part.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    let key = format!(
        "{}::{}",
        compute_normalize_target(&NormalizeTargetInput {
            value: Some(
                input
                    .target
                    .clone()
                    .unwrap_or_else(|| "tactical".to_string())
            ),
        })
        .value,
        &digest[..16]
    );
    PrincipleKeyForSessionOutput { key }
}

pub fn compute_check_first_principle_downgrade(
    input: &CheckFirstPrincipleDowngradeInput,
) -> CheckFirstPrincipleDowngradeOutput {
    let session = input
        .session
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let key = compute_principle_key_for_session(&PrincipleKeyForSessionInput {
        objective_id: session
            .get("objective_id")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        objective: session
            .get("objective")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        target: session
            .get("target")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
    })
    .key;
    let anti = value_path(
        input.policy.as_ref(),
        &["first_principles", "anti_downgrade"],
    )
    .and_then(|v| v.as_object())
    .cloned()
    .unwrap_or_default();
    if !to_bool_like(anti.get("enabled"), false) {
        return CheckFirstPrincipleDowngradeOutput {
            allowed: true,
            reason: None,
            key,
            lock_state: None,
        };
    }

    let lock_state = compute_load_first_principle_lock_state(&LoadFirstPrincipleLockStateInput {
        file_path: input.file_path.clone(),
        now_iso: input.now_iso.clone(),
    })
    .state;
    let existing =
        value_path(Some(&lock_state), &["locks", key.as_str()]).and_then(|v| v.as_object());
    if existing.is_none() {
        return CheckFirstPrincipleDowngradeOutput {
            allowed: true,
            reason: None,
            key,
            lock_state: Some(lock_state),
        };
    }
    let existing_obj = existing.cloned().unwrap_or_default();

    let existing_band = compute_normalize_token(&NormalizeTokenInput {
        value: Some(value_to_string(existing_obj.get("maturity_band"))),
        max_len: Some(24),
    })
    .value;
    let session_band = compute_normalize_token(&NormalizeTokenInput {
        value: Some(value_to_string(session.get("maturity_band"))),
        max_len: Some(24),
    })
    .value;
    let existing_idx = compute_band_to_index(&BandToIndexInput {
        band: Some(if existing_band.is_empty() {
            "novice".to_string()
        } else {
            existing_band
        }),
    })
    .index;
    let session_idx = compute_band_to_index(&BandToIndexInput {
        band: Some(if session_band.is_empty() {
            "novice".to_string()
        } else {
            session_band
        }),
    })
    .index;

    if to_bool_like(anti.get("require_same_or_higher_maturity"), false)
        && session_idx < existing_idx
    {
        return CheckFirstPrincipleDowngradeOutput {
            allowed: false,
            reason: Some("first_principle_downgrade_blocked_lower_maturity".to_string()),
            key,
            lock_state: Some(lock_state),
        };
    }

    if to_bool_like(anti.get("prevent_lower_confidence_same_band"), false)
        && session_idx == existing_idx
    {
        let floor_ratio = compute_clamp_number(&ClampNumberInput {
            value: anti.get("same_band_confidence_floor_ratio").cloned(),
            lo: Some(0.1),
            hi: Some(1.0),
            fallback: Some(0.92),
        })
        .value;
        let existing_confidence =
            js_number_for_extract(existing_obj.get("confidence")).unwrap_or(0.0);
        let floor = existing_confidence * floor_ratio;
        let confidence = if input.confidence.unwrap_or(0.0).is_finite() {
            input.confidence.unwrap_or(0.0)
        } else {
            0.0
        };
        if confidence < floor {
            return CheckFirstPrincipleDowngradeOutput {
                allowed: false,
                reason: Some("first_principle_downgrade_blocked_lower_confidence".to_string()),
                key,
                lock_state: Some(lock_state),
            };
        }
    }

    CheckFirstPrincipleDowngradeOutput {
        allowed: true,
        reason: None,
        key,
        lock_state: Some(lock_state),
    }
}

pub fn compute_upsert_first_principle_lock(
    input: &UpsertFirstPrincipleLockInput,
) -> UpsertFirstPrincipleLockOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let session = input
        .session
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let principle = input
        .principle
        .as_ref()
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let key = compute_principle_key_for_session(&PrincipleKeyForSessionInput {
        objective_id: session
            .get("objective_id")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        objective: session
            .get("objective")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
        target: session
            .get("target")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string()),
    })
    .key;

    let mut lock_state =
        compute_load_first_principle_lock_state(&LoadFirstPrincipleLockStateInput {
            file_path: input.file_path.clone(),
            now_iso: Some(now_iso.clone()),
        })
        .state;

    let existing = value_path(Some(&lock_state), &["locks", key.as_str()])
        .and_then(|v| v.as_object())
        .cloned();
    let next_band = compute_normalize_token(&NormalizeTokenInput {
        value: Some(value_to_string(session.get("maturity_band"))),
        max_len: Some(24),
    })
    .value;
    let next_band = if next_band.is_empty() {
        "novice".to_string()
    } else {
        next_band
    };
    let next_idx = compute_band_to_index(&BandToIndexInput {
        band: Some(next_band.clone()),
    })
    .index;

    let confidence_raw = js_number_for_extract(principle.get("confidence")).unwrap_or(0.0);
    let confidence = if confidence_raw.is_finite() {
        confidence_raw
    } else {
        0.0
    };
    let prev_idx = existing
        .as_ref()
        .map(|row| {
            compute_band_to_index(&BandToIndexInput {
                band: Some(value_to_string(row.get("maturity_band"))),
            })
            .index
        })
        .unwrap_or(-1);
    let merged_band = if prev_idx > next_idx {
        compute_normalize_token(&NormalizeTokenInput {
            value: Some(
                existing
                    .as_ref()
                    .and_then(|row| row.get("maturity_band"))
                    .map(|v| value_to_string(Some(v)))
                    .unwrap_or_else(|| next_band.clone()),
            ),
            max_len: Some(24),
        })
        .value
    } else {
        next_band.clone()
    };
    let existing_confidence = existing
        .as_ref()
        .and_then(|row| js_number_for_extract(row.get("confidence")))
        .unwrap_or(0.0);
    let merged_confidence = existing_confidence.max(confidence);
    let clamped_confidence = compute_clamp_number(&ClampNumberInput {
        value: Some(json!(merged_confidence)),
        lo: Some(0.0),
        hi: Some(1.0),
        fallback: Some(0.0),
    })
    .value;
    let rounded_confidence = (clamped_confidence * 1_000_000.0).round() / 1_000_000.0;

    let lock_row = json!({
        "key": key.clone(),
        "principle_id": clean_text_runtime(
            principle.get("id").and_then(|v| v.as_str()).unwrap_or(""),
            120
        ),
        "maturity_band": if merged_band.is_empty() { "novice".to_string() } else { merged_band },
        "confidence": rounded_confidence,
        "ts": now_iso.clone()
    });
    if lock_state
        .get("locks")
        .and_then(|v| v.as_object())
        .is_none()
    {
        if let Some(obj) = lock_state.as_object_mut() {
            obj.insert("locks".to_string(), json!({}));
        }
    }
    if let Some(locks) = lock_state.get_mut("locks").and_then(|v| v.as_object_mut()) {
        locks.insert(key.clone(), lock_row);
    }
    let saved = compute_save_first_principle_lock_state(&SaveFirstPrincipleLockStateInput {
        file_path: input.file_path.clone(),
        state: Some(lock_state),
        now_iso: Some(now_iso),
    });
    UpsertFirstPrincipleLockOutput {
        state: saved.state,
        key,
    }
}

pub fn compute_normalize_objective_arg(
    input: &NormalizeObjectiveArgInput,
) -> NormalizeObjectiveArgOutput {
    NormalizeObjectiveArgOutput {
        value: clean_text_runtime(input.value.as_deref().unwrap_or(""), 420),
    }
}

pub fn compute_maturity_band_order(_input: &MaturityBandOrderInput) -> MaturityBandOrderOutput {
    MaturityBandOrderOutput {
        bands: vec![
            "novice".to_string(),
            "developing".to_string(),
            "mature".to_string(),
            "seasoned".to_string(),
            "legendary".to_string(),
        ],
    }
}

pub fn compute_current_runtime_mode(input: &CurrentRuntimeModeInput) -> CurrentRuntimeModeOutput {
    let env_mode = compute_normalize_mode(&NormalizeModeInput {
        value: input.env_mode.clone(),
    })
    .value;
    if has_nonempty_flag(input.env_mode.as_deref()) {
        return CurrentRuntimeModeOutput { mode: env_mode };
    }
    let args_mode = compute_normalize_mode(&NormalizeModeInput {
        value: input.args_mode.clone(),
    })
    .value;
    if has_nonempty_flag(input.args_mode.as_deref()) {
        return CurrentRuntimeModeOutput { mode: args_mode };
    }
    let mode = compute_normalize_mode(&NormalizeModeInput {
        value: input.policy_runtime_mode.clone(),
    })
    .value;
    CurrentRuntimeModeOutput { mode }
}

pub fn compute_read_drift_from_state_file(
    input: &ReadDriftFromStateFileInput,
) -> ReadDriftFromStateFileOutput {
    let payload = input.payload.as_ref().and_then(|v| v.as_object());
    let source = clean_text_runtime(
        input
            .source_path
            .as_deref()
            .filter(|row| !row.is_empty())
            .or(input.file_path.as_deref())
            .unwrap_or("none"),
        260,
    );
    if payload.is_none() {
        return ReadDriftFromStateFileOutput { value: 0.0, source };
    }
    let payload_value = input.payload.as_ref();
    let value = [
        value_path(payload_value, &["drift_rate"]),
        value_path(payload_value, &["predicted_drift"]),
        value_path(payload_value, &["effective_drift_rate"]),
        value_path(payload_value, &["checks_effective", "drift_rate", "value"]),
        value_path(payload_value, &["checks", "drift_rate", "value"]),
        value_path(payload_value, &["last_decision", "drift_rate"]),
        value_path(payload_value, &["last_decision", "effective_drift_rate"]),
        value_path(
            payload_value,
            &["last_decision", "checks_effective", "drift_rate", "value"],
        ),
    ]
    .iter()
    .find_map(|row| parse_number_like(*row))
    .unwrap_or(0.0);
    ReadDriftFromStateFileOutput {
        value: round6(clamp_number(value, 0.0, 1.0)),
        source,
    }
}

pub fn compute_resolve_lens_gate_drift(
    input: &ResolveLensGateDriftInput,
) -> ResolveLensGateDriftOutput {
    let arg_value = input
        .arg_candidates
        .iter()
        .find_map(|row| compute_extract_numeric(&ExtractNumericInput { value: row.clone() }).value);
    if let Some(value) = arg_value {
        return ResolveLensGateDriftOutput {
            value: round6(clamp_number(value, 0.0, 1.0)),
            source: "arg".to_string(),
        };
    }
    let probe_path = input.probe_path.clone().unwrap_or_default();
    if probe_path.is_empty() {
        return ResolveLensGateDriftOutput {
            value: 0.0,
            source: "none".to_string(),
        };
    }
    let out = compute_read_drift_from_state_file(&ReadDriftFromStateFileInput {
        file_path: Some(probe_path),
        source_path: input.probe_source.clone(),
        payload: input.probe_payload.clone(),
    });
    ResolveLensGateDriftOutput {
        value: out.value,
        source: out.source,
    }
}
