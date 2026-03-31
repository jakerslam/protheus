fn normalize_delete_key(raw: Option<&Value>, fallback: &str) -> Value {
    let token = normalize_token(as_text(raw), 220);
    if !token.is_empty() {
        return Value::String(token);
    }
    let fallback = normalize_token(fallback, 220);
    if fallback.is_empty() {
        Value::Null
    } else {
        Value::String(fallback)
    }
}

fn validate_training_conduit_metadata(
    metadata: &Value,
    policy_input: Option<&Value>,
    root_dir: &Path,
) -> Value {
    let policy = normalize_policy(policy_input, root_dir);
    let defaults = policy
        .get("defaults")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let constraints = policy
        .get("constraints")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let m = metadata.as_object().cloned().unwrap_or_default();
    let source = m
        .get("source")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let owner = m
        .get("owner")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let license = m
        .get("license")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let consent = m
        .get("consent")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let retention = m
        .get("retention")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let deletion = m
        .get("delete")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut errors = Vec::<String>::new();
    if constraints
        .get("require_source")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        if normalize_token(as_text(source.get("system")), 120).is_empty() {
            errors.push("missing_source_system".to_string());
        }
        if normalize_token(as_text(source.get("channel")), 120).is_empty() {
            errors.push("missing_source_channel".to_string());
        }
    }
    if constraints
        .get("require_owner")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && normalize_token(as_text(owner.get("id")), 120).is_empty()
    {
        errors.push("missing_owner_id".to_string());
    }
    if constraints
        .get("require_license")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && normalize_token(as_text(license.get("id")), 160).is_empty()
    {
        errors.push("missing_license_id".to_string());
    }
    if constraints
        .get("require_consent")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        if normalize_consent_status(as_text(consent.get("status")), "").is_empty() {
            errors.push("missing_consent_status".to_string());
        }
        if normalize_consent_mode(as_text(consent.get("mode")), "").is_empty() {
            errors.push("missing_consent_mode".to_string());
        }
    }
    let min_retention = constraints
        .get("min_retention_days")
        .and_then(Value::as_i64)
        .unwrap_or(1);
    let max_retention = constraints
        .get("max_retention_days")
        .and_then(Value::as_i64)
        .unwrap_or(3650);
    let retention_days = clamp_int(retention.get("days"), min_retention, max_retention, -1);
    if retention_days < min_retention || retention_days > max_retention {
        errors.push("retention_days_out_of_range".to_string());
    }
    if constraints
        .get("require_delete_key")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && normalize_token(as_text(deletion.get("key")), 220).is_empty()
    {
        errors.push("missing_delete_key".to_string());
    }

    json!({
        "ok": errors.is_empty(),
        "errors": errors,
        "policy_version": as_text(policy.get("version")),
        "defaults_owner_id": as_text(defaults.get("owner_id"))
    })
}

fn build_training_conduit_metadata(
    input: Option<&Value>,
    policy_input: Option<&Value>,
    root_dir: &Path,
) -> Value {
    let policy = if let Some(policy) = policy_input {
        normalize_policy(Some(policy), root_dir)
    } else {
        load_policy(root_dir, &Map::new())
    };
    let defaults = policy
        .get("defaults")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let constraints = policy
        .get("constraints")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let schema = policy
        .get("schema")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let input = input
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let ts = {
        let raw = clean_text(as_text(input.get("ts")), 64);
        if raw.is_empty() {
            now_iso()
        } else {
            raw
        }
    };
    let source_system = {
        let value = normalize_token(
            if input.get("source_system").is_some() {
                as_text(input.get("source_system"))
            } else if input.get("system").is_some() {
                as_text(input.get("system"))
            } else {
                "unknown".to_string()
            },
            120,
        );
        if value.is_empty() {
            "unknown".to_string()
        } else {
            value
        }
    };
    let source_channel = {
        let value = normalize_token(
            if input.get("source_channel").is_some() {
                as_text(input.get("source_channel"))
            } else if input.get("channel").is_some() {
                as_text(input.get("channel"))
            } else {
                "unknown".to_string()
            },
            120,
        );
        if value.is_empty() {
            "unknown".to_string()
        } else {
            value
        }
    };
    let source_path = rel_path(
        root_dir,
        if input.get("source_path").is_some() {
            as_text(input.get("source_path"))
        } else {
            as_text(input.get("path"))
        },
    )
    .map(Value::String)
    .unwrap_or(Value::Null);
    let datum_id = {
        let value = normalize_token(
            if input.get("datum_id").is_some() {
                as_text(input.get("datum_id"))
            } else {
                as_text(input.get("record_id"))
            },
            180,
        );
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let provider = {
        let value = normalize_token(as_text(input.get("provider")), 120);
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let owner_id = {
        let value = normalize_token(
            if input.get("owner_id").is_some() {
                as_text(input.get("owner_id"))
            } else {
                as_text(defaults.get("owner_id"))
            },
            120,
        );
        if value.is_empty() {
            as_text(defaults.get("owner_id"))
        } else {
            value
        }
    };
    let owner_type = {
        let value = normalize_token(
            if input.get("owner_type").is_some() {
                as_text(input.get("owner_type"))
            } else {
                as_text(defaults.get("owner_type"))
            },
            80,
        );
        if value.is_empty() {
            as_text(defaults.get("owner_type"))
        } else {
            value
        }
    };
    let license_id = {
        let value = normalize_token(
            if input.get("license_id").is_some() {
                as_text(input.get("license_id"))
            } else {
                as_text(defaults.get("license_id"))
            },
            160,
        );
        if value.is_empty() {
            as_text(defaults.get("license_id"))
        } else {
            value
        }
    };
    let consent_status = normalize_consent_status(
        if input.get("consent_status").is_some() {
            as_text(input.get("consent_status"))
        } else {
            as_text(defaults.get("consent_status"))
        },
        &as_text(defaults.get("consent_status")),
    );
    let consent_mode = normalize_consent_mode(
        if input.get("consent_mode").is_some() {
            as_text(input.get("consent_mode"))
        } else {
            as_text(defaults.get("consent_mode"))
        },
        &as_text(defaults.get("consent_mode")),
    );
    let consent_evidence_ref = rel_path(
        root_dir,
        if input.get("consent_evidence_ref").is_some() {
            as_text(input.get("consent_evidence_ref"))
        } else {
            as_text(defaults.get("consent_evidence_ref"))
        },
    )
    .map(Value::String)
    .unwrap_or(Value::Null);
    let retention_days = clamp_int(
        input.get("retention_days"),
        constraints
            .get("min_retention_days")
            .and_then(Value::as_i64)
            .unwrap_or(1),
        constraints
            .get("max_retention_days")
            .and_then(Value::as_i64)
            .unwrap_or(3650),
        defaults
            .get("retention_days")
            .and_then(Value::as_i64)
            .unwrap_or(365),
    );
    let delete_scope = {
        let value = normalize_token(
            if input.get("delete_scope").is_some() {
                as_text(input.get("delete_scope"))
            } else {
                as_text(defaults.get("delete_scope"))
            },
            120,
        );
        if value.is_empty() {
            as_text(defaults.get("delete_scope"))
        } else {
            value
        }
    };
    let fallback_delete_key = format!(
        "{}:{}:{}",
        source_system,
        source_channel,
        datum_id
            .as_str()
            .unwrap_or(&Utc::now().timestamp_millis().to_string())
    );
    let delete_key = normalize_delete_key(input.get("delete_key"), &fallback_delete_key);
    let classification = {
        let value = normalize_token(
            if input.get("classification").is_some() {
                as_text(input.get("classification"))
            } else {
                as_text(defaults.get("classification"))
            },
            80,
        );
        if value.is_empty() {
            as_text(defaults.get("classification"))
        } else {
            value
        }
    };

    let metadata = json!({
        "schema_id": as_text(schema.get("id")),
        "schema_version": as_text(schema.get("version")),
        "policy_version": as_text(policy.get("version")),
        "ts": ts,
        "source": {
            "system": source_system,
            "channel": source_channel,
            "path": source_path,
            "datum_id": datum_id,
            "provider": provider
        },
        "owner": {
            "id": owner_id,
            "type": owner_type
        },
        "license": {
            "id": license_id
        },
        "consent": {
            "status": consent_status,
            "mode": consent_mode,
            "evidence_ref": consent_evidence_ref
        },
        "retention": {
            "days": retention_days,
            "expires_ts": retention_expiry(&ts, retention_days)
        },
        "delete": {
            "key": delete_key,
            "scope": delete_scope
        },
        "classification": classification
    });

    let mut metadata_obj = metadata.as_object().cloned().unwrap_or_default();
    metadata_obj.insert(
        "validation".to_string(),
        validate_training_conduit_metadata(&metadata, Some(&policy), root_dir),
    );
    Value::Object(metadata_obj)
}

