fn canonical_backlog_status(status: &str) -> String {
    status
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch)
            } else if ch == '-' || ch == '_' {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>()
}

fn status_allowed(status: &str, allowed: &std::collections::BTreeSet<String>) -> bool {
    let token = canonical_backlog_status(status);
    !token.is_empty() && allowed.contains(&token)
}

fn compile_backlog(policy: &Policy) -> Result<CompiledBacklog, String> {
    let raw = fs::read_to_string(&policy.paths.backlog_path)
        .map_err(|e| format!("read_backlog_failed:{}", e))?;
    let parsed = parse_backlog_rows(&raw);
    let (rows, mut conflicts) = resolve_rows(parsed);
    let generated_at = now_iso();

    let active_statuses = policy
        .active_statuses
        .iter()
        .map(|status| canonical_backlog_status(status))
        .filter(|status| !status.is_empty())
        .collect::<std::collections::BTreeSet<_>>();
    let archive_statuses = policy
        .archive_statuses
        .iter()
        .map(|status| canonical_backlog_status(status))
        .filter(|status| !status.is_empty())
        .collect::<std::collections::BTreeSet<_>>();
    let mut known_statuses = active_statuses.clone();
    known_statuses.extend(archive_statuses.iter().cloned());

    let active_rows = rows
        .iter()
        .filter(|r| status_allowed(&r.status, &active_statuses))
        .cloned()
        .collect::<Vec<_>>();
    let archive_rows = rows
        .iter()
        .filter(|r| status_allowed(&r.status, &archive_statuses))
        .cloned()
        .collect::<Vec<_>>();
    for row in &rows {
        let status = canonical_backlog_status(&row.status);
        if status.is_empty() || !known_statuses.contains(&status) {
            conflicts.push(json!({
                "row_id": row.id,
                "reason": "status_not_in_policy",
                "status": row.status
            }));
        }
    }

    Ok(CompiledBacklog {
        generated_at: generated_at.clone(),
        rows: rows.clone(),
        conflicts,
        active_view: render_table_view("Backlog Active View", &active_rows, &generated_at),
        archive_view: render_table_view("Backlog Archive View", &archive_rows, &generated_at),
        priority_view: render_priority_queue(&rows, &policy.active_statuses),
        reviewed_view: render_reviewed(&rows, &policy.active_statuses),
        execution_view: render_execution_path(&rows, &policy.active_statuses),
    })
}

fn sync(policy: &Policy) -> Result<Value, String> {
    let compiled = compile_backlog(policy)?;

    let registry_json = json!({
        "schema_id": "backlog_registry",
        "schema_version": policy.version,
        "generated_at": compiled.generated_at,
        "row_count": compiled.rows.len(),
        "rows": compiled.rows,
        "conflicts": compiled.conflicts,
    });

    let registry_text = serde_json::to_string_pretty(&registry_json)
        .map(|s| format!("{}\n", s))
        .map_err(|e| format!("encode_registry_failed:{}", e))?;

    write_text_atomic(&policy.paths.registry_path, &registry_text)?;
    write_text_atomic(
        &policy.paths.active_view_path,
        &format!("{}\n", compiled.active_view),
    )?;
    write_text_atomic(
        &policy.paths.archive_view_path,
        &format!("{}\n", compiled.archive_view),
    )?;
    write_text_atomic(
        &policy.paths.priority_view_path,
        &format!("{}\n", compiled.priority_view),
    )?;
    write_text_atomic(
        &policy.paths.reviewed_view_path,
        &format!("{}\n", compiled.reviewed_view),
    )?;
    write_text_atomic(
        &policy.paths.execution_path_view_path,
        &format!("{}\n", compiled.execution_view),
    )?;

    let payload = json!({
        "ok": true,
        "type": "backlog_registry_sync",
        "ts": now_iso(),
        "backlog_path": policy.paths.backlog_path,
        "registry_path": policy.paths.registry_path,
        "rows": registry_json.get("row_count").and_then(Value::as_u64).unwrap_or(0),
        "conflicts": registry_json.get("conflicts").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
        "rows_hash": canonical_rows_hash(&compiled.rows),
        "claim_evidence": [
            {
                "id": "backlog_registry_sync",
                "claim": "backlog_views_generated_from_srs",
                "evidence": {
                    "backlog": policy.paths.backlog_path,
                    "active_view": policy.paths.active_view_path,
                    "archive_view": policy.paths.archive_view_path,
                    "priority_view": policy.paths.priority_view_path,
                    "reviewed_view": policy.paths.reviewed_view_path,
                    "execution_view": policy.paths.execution_path_view_path
                }
            }
        ]
    });

    let mut latest = payload.clone();
    latest["receipt_hash"] = Value::String(deterministic_receipt_hash(&latest));

    write_text_atomic(
        &policy.paths.latest_path,
        &format!(
            "{}\n",
            serde_json::to_string_pretty(&latest)
                .map_err(|e| format!("encode_latest_failed:{}", e))?
        ),
    )?;

    write_text_atomic(
        &policy.paths.state_path,
        &format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({
                "schema_id": "backlog_registry_state",
                "schema_version": "1.0",
                "updated_at": now_iso(),
                "rows_hash": latest.get("rows_hash").cloned().unwrap_or(Value::Null),
                "row_count": latest.get("rows").cloned().unwrap_or(Value::Null)
            }))
            .map_err(|e| format!("encode_state_failed:{}", e))?
        ),
    )?;

    append_jsonl(&policy.paths.receipts_path, &latest)?;
    Ok(latest)
}

fn check(policy: &Policy, strict: bool) -> Result<(Value, i32), String> {
    let compiled = compile_backlog(policy)?;

    let expected_registry_json = json!({
        "schema_id": "backlog_registry",
        "schema_version": policy.version,
        "generated_at": compiled.generated_at,
        "row_count": compiled.rows.len(),
        "rows": compiled.rows,
        "conflicts": compiled.conflicts,
    });

    let expected_rows = expected_registry_json
        .get("rows")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let expected_hash = deterministic_receipt_hash(&expected_rows);

    let actual_registry = fs::read_to_string(&policy.paths.registry_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok());

    let actual_hash = actual_registry
        .as_ref()
        .and_then(|v| v.get("rows").cloned())
        .map(|v| deterministic_receipt_hash(&v));

    let mut mismatches = Vec::new();
    if actual_hash.as_deref() != Some(expected_hash.as_str()) {
        mismatches.push(json!({
            "path": policy.paths.registry_path,
            "reason": "rows_hash_mismatch",
            "expected": expected_hash,
            "actual": actual_hash
        }));
    }

    let checks: Vec<(PathBuf, String)> = vec![
        (
            policy.paths.active_view_path.clone(),
            format!("{}\n", compiled.active_view),
        ),
        (
            policy.paths.archive_view_path.clone(),
            format!("{}\n", compiled.archive_view),
        ),
        (
            policy.paths.priority_view_path.clone(),
            format!("{}\n", compiled.priority_view),
        ),
        (
            policy.paths.reviewed_view_path.clone(),
            format!("{}\n", compiled.reviewed_view),
        ),
        (
            policy.paths.execution_path_view_path.clone(),
            format!("{}\n", compiled.execution_view),
        ),
    ];

    for (path, expected) in checks {
        let actual = fs::read_to_string(&path).unwrap_or_default();
        if normalize_text_compare(&actual) != normalize_text_compare(&expected) {
            mismatches.push(json!({
                "path": path,
                "reason": "view_mismatch"
            }));
        }
    }

    let ok = mismatches.is_empty();
    let mut payload = json!({
        "ok": ok,
        "type": "backlog_registry_check",
        "ts": now_iso(),
        "strict": strict,
        "mismatch_count": mismatches.len(),
        "mismatches": mismatches,
        "expected_rows_hash": expected_hash,
        "claim_evidence": [
            {
                "id": "backlog_consistency_gate",
                "claim": "all_generated_backlog_views_match_srs_compiler",
                "evidence": {
                    "backlog": policy.paths.backlog_path,
                    "strict": strict,
                    "checks": [
                        policy.paths.registry_path,
                        policy.paths.active_view_path,
                        policy.paths.archive_view_path,
                        policy.paths.priority_view_path,
                        policy.paths.reviewed_view_path,
                        policy.paths.execution_path_view_path
                    ]
                }
            }
        ]
    });
    payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));

    let code = if strict && !ok { 1 } else { 0 };
    Ok((payload, code))
}

fn status(policy: &Policy) -> Value {
    let latest = fs::read_to_string(&policy.paths.latest_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| {
            json!({
                "ok": false,
                "type": "backlog_registry_status",
                "error": "latest_missing"
            })
        });

    let mut out = json!({
        "ok": latest.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "type": "backlog_registry_status",
        "ts": now_iso(),
        "latest": latest,
        "backlog_path": policy.paths.backlog_path,
        "registry_path": policy.paths.registry_path,
        "active_view_path": policy.paths.active_view_path,
        "archive_view_path": policy.paths.archive_view_path,
        "priority_view_path": policy.paths.priority_view_path,
        "reviewed_view_path": policy.paths.reviewed_view_path,
        "execution_view_path": policy.paths.execution_path_view_path
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn cli_error_receipt(argv: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "backlog_registry_cli_error",
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": code
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let policy = load_policy(root, parsed.flags.get("policy"));
    if let Err(err) = enforce_canonical_backlog_path(root, &policy) {
        print_json_line(&cli_error_receipt(argv, &err, 1));
        return 1;
    }
    let strict = parse_bool(parsed.flags.get("strict"), policy.strict_default);

    match cmd.as_str() {
        "sync" => match sync(&policy) {
            Ok(payload) => {
                print_json_line(&payload);
                0
            }
            Err(err) => {
                print_json_line(&cli_error_receipt(argv, &format!("sync_failed:{err}"), 1));
                1
            }
        },
        "check" => match check(&policy, strict) {
            Ok((payload, code)) => {
                print_json_line(&payload);
                code
            }
            Err(err) => {
                print_json_line(&cli_error_receipt(argv, &format!("check_failed:{err}"), 1));
                1
            }
        },
        "status" => {
            print_json_line(&status(&policy));
            0
        }
        _ => {
            usage();
            print_json_line(&cli_error_receipt(argv, "unknown_command", 2));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_prefers_done_over_queued_conflict() {
        let parsed = vec![
            ParsedRow {
                row: RegistryRow {
                    id: "V6-TEST-001".to_string(),
                    class: "backlog".to_string(),
                    wave: "V6".to_string(),
                    status: "queued".to_string(),
                    title: "X".to_string(),
                    problem: "p".to_string(),
                    acceptance: "a".to_string(),
                    dependencies: vec![],
                },
                canonical: true,
                source_index: 2,
            },
            ParsedRow {
                row: RegistryRow {
                    id: "V6-TEST-001".to_string(),
                    class: "backlog".to_string(),
                    wave: "V6".to_string(),
                    status: "done".to_string(),
                    title: "X".to_string(),
                    problem: "p".to_string(),
                    acceptance: "a".to_string(),
                    dependencies: vec![],
                },
                canonical: false,
                source_index: 1,
            },
        ];

        let (rows, conflicts) = resolve_rows(parsed);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, "done");
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn parse_dependencies_extracts_ids() {
        let deps = parse_dependencies("V6-AAA-001, req V6-BBB-010 and [V6-CCC-999]");
        assert_eq!(deps, vec!["V6-AAA-001", "V6-BBB-010", "V6-CCC-999"]);
    }
}
