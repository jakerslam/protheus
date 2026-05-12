fn validate_finding(input: &Value) -> (bool, String) {
    let finding = if input.is_object() {
        input
    } else {
        return (false, "finding_invalid_type".to_string());
    };

    for key in [
        "audit_id",
        "item_id",
        "severity",
        "status",
        "location",
        "evidence",
        "timestamp",
    ] {
        if finding.get(key).is_none() {
            return (false, format!("finding_missing_{key}"));
        }
    }

    let severity = to_clean_string(finding.get("severity")).to_ascii_lowercase();
    if severity_order(&severity) == 0 {
        return (false, "finding_invalid_severity".to_string());
    }

    let status = to_clean_string(finding.get("status")).to_ascii_lowercase();
    if status_order(&status) == 0 {
        return (false, "finding_invalid_status".to_string());
    }

    if to_clean_string(finding.get("audit_id")).is_empty() {
        return (false, "finding_invalid_audit_id".to_string());
    }
    if to_clean_string(finding.get("item_id")).is_empty() {
        return (false, "finding_invalid_item_id".to_string());
    }
    if to_clean_string(finding.get("location")).is_empty() {
        return (false, "finding_invalid_location".to_string());
    }

    let evidence = finding.get("evidence").and_then(Value::as_array);
    if evidence.map(|rows| rows.is_empty()).unwrap_or(true) {
        return (false, "finding_invalid_evidence".to_string());
    }
    for row in evidence.unwrap_or(&Vec::new()) {
        if !row.is_object() {
            return (false, "finding_invalid_evidence_row".to_string());
        }
        if to_clean_string(row.get("type")).is_empty() {
            return (false, "finding_invalid_evidence_type".to_string());
        }
        if to_clean_string(row.get("value")).is_empty() {
            return (false, "finding_invalid_evidence_value".to_string());
        }
    }

    let timestamp = to_clean_string(finding.get("timestamp"));
    if !is_datetime(&timestamp) {
        return (false, "finding_invalid_timestamp".to_string());
    }

    (true, "finding_valid".to_string())
}

fn finding_metadata_string(finding: &Value, key: &str) -> String {
    finding
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get(key))
        .map(|value| to_clean_string(Some(value)))
        .unwrap_or_default()
}

fn finding_idempotency_key(task_id: &str, finding: &Value) -> String {
    let explicit = get_string_any(
        finding,
        &[
            "idempotency_key",
            "idempotencyKey",
            "finding_id",
            "findingId",
        ],
    );
    if !explicit.is_empty() {
        return explicit;
    }

    let workflow_id = {
        let direct = get_string_any(finding, &["workflow_id", "workflowId"]);
        if direct.is_empty() {
            finding_metadata_string(finding, "workflow_id")
        } else {
            direct
        }
    };
    let finding_id = finding_metadata_string(finding, "finding_id");
    let raw = format!(
        "task_id={}|workflow_id={}|audit_id={}|finding_id={}|item_id={}|location={}",
        task_id,
        workflow_id,
        to_clean_string(finding.get("audit_id")),
        finding_id,
        to_clean_string(finding.get("item_id")),
        to_clean_string(finding.get("location"))
    );
    format!("finding-{}", stable_hash_short(&raw))
}

fn normalize_finding_with_idempotency_key(
    task_id: &str,
    finding: &Value,
) -> Result<Value, (String, Value)> {
    let mut normalized = normalize_finding(finding);
    let (ok, reason) = validate_finding(&normalized);
    if !ok {
        return Err((reason, normalized));
    }

    let idempotency_key = finding_idempotency_key(task_id, &normalized);
    if let Value::Object(map) = &mut normalized {
        if to_clean_string(map.get("idempotency_key")).is_empty() {
            map.insert("idempotency_key".to_string(), Value::String(idempotency_key));
        }
    }
    Ok(normalized)
}

fn merge_finding_versions(task_id: &str, existing: &Value, incoming: &Value) -> Value {
    let merged = merge_findings(&[existing.clone(), incoming.clone()]);
    let merged_finding = merged
        .get("merged")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .cloned()
        .unwrap_or_else(|| incoming.clone());
    normalize_finding_with_idempotency_key(task_id, &merged_finding)
        .unwrap_or_else(|_| incoming.clone())
}

fn append_finding(root: &Path, task_id: &str, finding: &Value, root_dir: Option<&str>) -> Value {
    let normalized = match normalize_finding_with_idempotency_key(task_id, finding) {
        Ok(value) => value,
        Err((reason, _normalized)) => {
            return json!({
                "ok": false,
                "type": "orchestration_scratchpad_append_finding",
                "reason_code": reason,
                "task_id": task_id
            });
        }
    };

    let mut out = append_normalized_findings_batch(root, task_id, &[normalized], None, root_dir);
    if let Value::Object(map) = &mut out {
        map.insert(
            "type".to_string(),
            Value::String("orchestration_scratchpad_append_finding".to_string()),
        );
        let deduped = map
            .get("deduped_count")
            .and_then(Value::as_u64)
            .unwrap_or_default()
            > 0;
        map.insert("deduped".to_string(), Value::Bool(deduped));
    }
    out
}

fn append_findings_batch(
    root: &Path,
    task_id: &str,
    findings: &[Value],
    progress: Option<Value>,
    root_dir: Option<&str>,
) -> Value {
    let mut normalized_findings = Vec::new();
    let mut staged_keys = Vec::new();
    for (index, finding) in findings.iter().enumerate() {
        let normalized = match normalize_finding_with_idempotency_key(task_id, finding) {
            Ok(value) => value,
            Err((reason, normalized)) => {
                return json!({
                    "ok": false,
                    "type": "orchestration_scratchpad_append_findings",
                    "reason_code": reason,
                    "task_id": task_id,
                    "failed_index": index,
                    "failed_finding": normalized,
                    "reconciliation": {
                        "write_strategy": "single_atomic_scratchpad_write",
                        "committed_before_failure": [],
                        "staged_idempotency_keys": staged_keys
                    }
                });
            }
        };
        staged_keys.push(Value::String(finding_idempotency_key(task_id, &normalized)));
        normalized_findings.push(normalized);
    }

    append_normalized_findings_batch(root, task_id, &normalized_findings, progress, root_dir)
}

fn append_normalized_findings_batch(
    root: &Path,
    task_id: &str,
    normalized_findings: &[Value],
    progress: Option<Value>,
    root_dir: Option<&str>,
) -> Value {
    let staged_keys = normalized_findings
        .iter()
        .map(|finding| Value::String(finding_idempotency_key(task_id, finding)))
        .collect::<Vec<_>>();

    let loaded = match load_scratchpad(root, task_id, root_dir) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_scratchpad_append_findings",
                "reason_code": err,
                "task_id": task_id,
                "reconciliation": {
                    "write_strategy": "single_atomic_scratchpad_write",
                    "committed_before_failure": [],
                    "staged_idempotency_keys": staged_keys
                }
            });
        }
    };

    let mut findings = loaded
        .scratchpad
        .get("findings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut seen = findings
        .iter()
        .enumerate()
        .map(|(index, existing)| (finding_idempotency_key(task_id, existing), index))
        .collect::<HashMap<_, _>>();
    let mut appended_count = 0usize;
    let mut deduped_count = 0usize;
    let mut reconciled_count = 0usize;
    for finding in normalized_findings {
        let key = finding_idempotency_key(task_id, finding);
        if let Some(index) = seen.get(&key).copied() {
            let merged = merge_finding_versions(task_id, &findings[index], finding);
            if findings[index] != merged {
                findings[index] = merged;
                reconciled_count += 1;
            }
            deduped_count += 1;
        } else {
            seen.insert(key, findings.len());
            findings.push(finding.clone());
            appended_count += 1;
        }
    }

    let mut patch = Map::new();
    patch.insert("findings".to_string(), Value::Array(findings));
    if let Some(progress_value) = progress {
        patch.insert("progress".to_string(), progress_value);
    }

    let out = match write_scratchpad(root, task_id, &Value::Object(patch), root_dir) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_scratchpad_append_findings",
                "reason_code": err,
                "task_id": task_id,
                "reconciliation": {
                    "write_strategy": "single_atomic_scratchpad_write",
                    "committed_before_failure": [],
                    "staged_idempotency_keys": staged_keys
                }
            });
        }
    };

    let count = out
        .get("scratchpad")
        .and_then(|v| v.get("findings"))
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);

    json!({
        "ok": true,
        "type": "orchestration_scratchpad_append_findings",
        "task_id": task_id,
        "file_path": out.get("file_path").cloned().unwrap_or(Value::Null),
        "scratchpad": out.get("scratchpad").cloned().unwrap_or(Value::Null),
        "finding_count": count,
        "appended_count": appended_count,
        "deduped_count": deduped_count,
        "reconciled_count": reconciled_count,
        "idempotency_keys": staged_keys
    })
}

fn append_checkpoint(
    root: &Path,
    task_id: &str,
    checkpoint: &Value,
    root_dir: Option<&str>,
) -> Value {
    let loaded = match load_scratchpad(root, task_id, root_dir) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_scratchpad_append_checkpoint",
                "reason_code": err,
                "task_id": task_id
            });
        }
    };

    let mut rows = loaded
        .scratchpad
        .get("checkpoints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut next_checkpoint = get_object(checkpoint);
    if to_clean_string(next_checkpoint.get("created_at")).is_empty() {
        next_checkpoint.insert("created_at".to_string(), Value::String(now_iso()));
    }
    rows.push(Value::Object(next_checkpoint));

    let out = match write_scratchpad(root, task_id, &json!({ "checkpoints": rows }), root_dir) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_scratchpad_append_checkpoint",
                "reason_code": err,
                "task_id": task_id
            });
        }
    };

    let count = out
        .get("scratchpad")
        .and_then(|v| v.get("checkpoints"))
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);

    json!({
        "ok": true,
        "type": "orchestration_scratchpad_append_checkpoint",
        "task_id": task_id,
        "file_path": out.get("file_path").cloned().unwrap_or(Value::Null),
        "scratchpad": out.get("scratchpad").cloned().unwrap_or(Value::Null),
        "checkpoint_count": count
    })
}

fn cleanup_scratchpad(root: &Path, task_id: &str, root_dir: Option<&str>) -> Value {
    let file_path = match scratchpad_path(root, task_id, root_dir) {
        Ok(path) => path,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_scratchpad_cleanup",
                "reason_code": err,
                "task_id": task_id
            });
        }
    };

    let _ = fs::remove_file(&file_path);
    json!({
        "ok": true,
        "type": "orchestration_scratchpad_cleanup",
        "task_id": task_id,
        "file_path": file_path,
        "removed": !file_path.exists()
    })
}

fn parse_scope_list(input: Option<&Value>, upper: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    let source = match input {
        Some(Value::Array(rows)) => rows.clone(),
        Some(Value::String(text)) => text
            .split(',')
            .map(|v| Value::String(v.trim().to_string()))
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    for row in source {
        let mut token = to_clean_string(Some(&row));
        if token.is_empty() {
            continue;
        }
        if upper {
            token = token.to_ascii_uppercase();
        } else {
            token = token.replace('\\', "/");
        }
        if seen.insert(token.clone()) {
            out.push(token);
        }
    }

    out
}

fn normalize_path_pattern(raw: &str) -> String {
    raw.trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn path_pattern_overlaps(left_raw: &str, right_raw: &str) -> bool {
    let left = normalize_path_pattern(left_raw);
    let right = normalize_path_pattern(right_raw);
    if left.is_empty() || right.is_empty() {
        return false;
    }
    if left == right {
        return true;
    }

    let left_prefix = if left.ends_with('*') {
        left.trim_end_matches('*')
    } else {
        ""
    };
    let right_prefix = if right.ends_with('*') {
        right.trim_end_matches('*')
    } else {
        ""
    };

    (!left_prefix.is_empty() && right.starts_with(left_prefix))
        || (!right_prefix.is_empty() && left.starts_with(right_prefix))
        || (left_prefix.is_empty() && !right_prefix.is_empty() && left.starts_with(right_prefix))
        || (right_prefix.is_empty() && !left_prefix.is_empty() && right.starts_with(left_prefix))
}

fn finding_matches_path_scope(finding: &Value, path_scopes: &[String]) -> bool {
    if path_scopes.is_empty() {
        return true;
    }

    let location = normalize_path_pattern(&to_clean_string(finding.get("location")));
    if location.is_empty() {
        return false;
    }

    for pattern_raw in path_scopes {
        let pattern = normalize_path_pattern(pattern_raw);
        if pattern.is_empty() {
            continue;
        }
        if pattern.ends_with('*') {
            let prefix = pattern.trim_end_matches('*');
            if location.starts_with(prefix) {
                return true;
            }
            continue;
        }
        if location == pattern
            || location.starts_with(&format!("{pattern}:"))
            || location.starts_with(&format!("{pattern}#"))
        {
            return true;
        }
    }
    false
}

fn finding_matches_series_scope(finding: &Value, series_scopes: &[String]) -> bool {
    if series_scopes.is_empty() {
        return true;
    }
    let item_id = to_clean_string(finding.get("item_id")).to_ascii_uppercase();
    if item_id.is_empty() {
        return false;
    }
    series_scopes
        .iter()
        .any(|series| item_id.starts_with(&series.to_ascii_uppercase()))
}

fn normalize_scope(raw_scope: &Value, index: usize) -> Value {
    let scope = if raw_scope.is_object() {
        raw_scope.clone()
    } else {
        Value::Object(Map::new())
    };

    let scope_id_raw = {
        let id = get_string_any(&scope, &["scope_id", "scopeId"]);
        if id.is_empty() {
            format!("scope-{}", index + 1)
        } else {
            id.to_ascii_lowercase()
        }
    };

    let scope_id = if validate_group_id(&scope_id_raw) {
        scope_id_raw
    } else {
        format!("scope-{}", index + 1)
    };

    let series = parse_scope_list(scope.get("series"), true);
    let paths = parse_scope_list(scope.get("paths"), false)
        .into_iter()
        .map(|row| normalize_path_pattern(&row))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();

    if series.is_empty() && paths.is_empty() {
        return json!({
            "ok": false,
            "reason_code": "scope_missing_series_and_paths",
            "scope_id": scope_id
        });
    }

    json!({
        "ok": true,
        "scope": {
            "scope_id": scope_id,
            "series": series,
            "paths": paths
        }
    })
}
