
fn validate_lensmap_annotation(annotation: Option<&Value>) -> Value {
    let Some(annotation) = annotation else {
        return json!({ "ok": true, "reason_code": "lensmap_annotation_not_provided" });
    };
    let Some(obj) = annotation.as_object() else {
        return json!({ "ok": false, "reason_code": "lensmap_annotation_invalid_type" });
    };

    let node_id = obj
        .get("node_id")
        .or_else(|| obj.get("nodeId"))
        .map(value_as_text)
        .unwrap_or_default();
    if node_id.is_empty() {
        return json!({ "ok": false, "reason_code": "lensmap_annotation_missing_node_id" });
    }

    let tags = obj
        .get("tags")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let jots = obj
        .get("jots")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if tags.is_empty() && jots.is_empty() {
        return json!({ "ok": false, "reason_code": "lensmap_annotation_missing_tags_or_jots" });
    }

    let mut seen = BTreeSet::<String>::new();
    for tag in tags {
        let normalized = value_as_text(&tag).to_ascii_lowercase();
        if normalized.is_empty() {
            return json!({ "ok": false, "reason_code": "lensmap_annotation_empty_tag" });
        }
        if !seen.insert(normalized) {
            return json!({ "ok": false, "reason_code": "lensmap_annotation_duplicate_tag" });
        }
    }

    json!({ "ok": true, "reason_code": "lensmap_annotation_valid" })
}

fn merged_policy(raw: Option<&Value>) -> Policy {
    let mut policy = Policy::default();
    let Some(obj) = raw.and_then(Value::as_object) else {
        return policy;
    };

    if let Some(value) = obj.get("index_first_required").and_then(Value::as_bool) {
        policy.index_first_required = value;
    }
    if let Some(value) = obj.get("max_burn_slo_tokens").and_then(Value::as_i64) {
        policy.max_burn_slo_tokens = value;
    }
    if let Some(value) = obj.get("max_recall_top").and_then(Value::as_i64) {
        policy.max_recall_top = value;
    }
    if let Some(value) = obj.get("max_max_files").and_then(Value::as_i64) {
        policy.max_max_files = value;
    }
    if let Some(value) = obj.get("max_expand_lines").and_then(Value::as_i64) {
        policy.max_expand_lines = value;
    }
    if let Some(value) = obj
        .get("bootstrap_hydration_token_cap")
        .and_then(Value::as_i64)
    {
        policy.bootstrap_hydration_token_cap = value;
    }
    if let Some(value) = obj.get("block_stale_override").and_then(Value::as_bool) {
        policy.block_stale_override = value;
    }
    policy
}

fn validate_memory_policy(args: &[String], options: Option<&Value>) -> Value {
    let policy = merged_policy(options.and_then(|value| value.get("policy")));
    let parsed = parse_cli_args(args);
    let command = options
        .and_then(|value| value.get("command"))
        .map(value_as_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            parsed
                .positional
                .first()
                .cloned()
                .unwrap_or_else(|| "status".to_string())
        })
        .trim()
        .to_ascii_lowercase();

    if NON_EXECUTING_COMMANDS.contains(&command.as_str()) {
        return json!({
            "ok": true,
            "type": "memory_policy_validation",
            "reason_code": "policy_not_required_for_status_command",
            "policy": policy,
        });
    }

    if policy.index_first_required {
        if read_boolean(&parsed.flags, INDEX_BYPASS_FLAGS, false) {
            return build_failure(
                "index_first_bypass_forbidden",
                json!({ "command": command }),
            );
        }
        if DIRECT_READ_FLAGS.iter().any(|flag| {
            parsed
                .flags
                .get(*flag)
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
        }) {
            return build_failure("direct_file_read_forbidden", json!({ "command": command }));
        }
    }

    let bootstrap = read_boolean(&parsed.flags, &["bootstrap"], false);
    let lazy_hydration = read_boolean(&parsed.flags, &["lazy-hydration", "lazy_hydration"], true);
    let hydration_tokens = read_numeric(
        &parsed.flags,
        &["estimated-hydration-tokens", "estimated_hydration_tokens"],
        0,
    );
    if bootstrap && !lazy_hydration {
        return build_failure(
            "bootstrap_requires_lazy_hydration",
            json!({ "command": command }),
        );
    }
    if bootstrap && hydration_tokens > policy.bootstrap_hydration_token_cap {
        return build_failure(
            "bootstrap_hydration_token_cap_exceeded",
            json!({
                "cap": policy.bootstrap_hydration_token_cap,
                "hydration_tokens": hydration_tokens,
            }),
        );
    }

    let burn_threshold = read_numeric(
        &parsed.flags,
        &[
            "burn-threshold",
            "burn_threshold",
            "burn-slo-threshold",
            "burn_slo_threshold",
        ],
        policy.max_burn_slo_tokens,
    );
    if burn_threshold > policy.max_burn_slo_tokens {
        return build_failure(
            "burn_slo_threshold_exceeded",
            json!({
                "configured_threshold": burn_threshold,
                "max_burn_slo_tokens": policy.max_burn_slo_tokens,
            }),
        );
    }

    if !read_boolean(&parsed.flags, &["fail-closed", "fail_closed"], true) {
        return build_failure("fail_closed_required", json!({ "command": command }));
    }

    let top = read_numeric(&parsed.flags, &["top", "recall-top", "recall_top"], 5);
    let max_files = read_numeric(&parsed.flags, &["max-files", "max_files"], 1);
    let expand_lines = read_numeric(&parsed.flags, &["expand-lines", "expand_lines"], 0);
    if top > policy.max_recall_top
        || max_files > policy.max_max_files
        || expand_lines > policy.max_expand_lines
    {
        return build_failure(
            "recall_budget_exceeded",
            json!({
                "top": top,
                "max_files": max_files,
                "expand_lines": expand_lines,
                "policy": {
                    "max_recall_top": policy.max_recall_top,
                    "max_max_files": policy.max_max_files,
                    "max_expand_lines": policy.max_expand_lines,
                }
            }),
        );
    }

    if policy.block_stale_override && read_boolean(&parsed.flags, STALE_OVERRIDE_FLAGS, false) {
        return build_failure("stale_override_forbidden", json!({ "command": command }));
    }

    let scores = read_json_flag(&parsed.flags, &["scores-json", "scores_json"]);
    let ids = read_json_flag(&parsed.flags, &["ids-json", "ids_json"]);
    if scores.is_some() || ids.is_some() {
        let scores_array = scores
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        let ids_array = ids
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        let ranking = validate_descending_ranking(&scores_array, &ids_array);
        if !ranking.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return build_failure(
                ranking
                    .get("reason_code")
                    .and_then(Value::as_str)
                    .unwrap_or("ranking_validation_failed"),
                json!({ "command": command }),
            );
        }
    }

    let annotation = read_json_flag(
        &parsed.flags,
        &["lensmap-annotation-json", "lensmap_annotation_json"],
    );
    if annotation.is_some() {
        let validation = validate_lensmap_annotation(annotation.as_ref());
        if !validation
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return build_failure(
                validation
                    .get("reason_code")
                    .and_then(Value::as_str)
                    .unwrap_or("lensmap_annotation_invalid"),
                json!({ "command": command }),
            );
        }
    }

    json!({
        "ok": true,
        "type": "memory_policy_validation",
        "reason_code": "policy_ok",
        "command": command,
        "policy": policy,
        "effective_budget": {
            "top": top,
            "max_files": max_files,
            "expand_lines": expand_lines,
            "burn_threshold": burn_threshold,
        }
    })
}
