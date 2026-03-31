fn run_command(cli_root: &Path, argv: &[String]) -> CommandResult {
    let root = resolve_root(cli_root);
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return CommandResult {
            exit_code: 0,
            payload: with_hash(json!({"ok":true,"type":"public_api_catalog_usage"})),
        };
    }
    let policy = load_policy(&root, argv);
    let mut state = match load_state(&policy.state_path) {
        Ok(v) => v,
        Err(e) => return err(&root, &policy, &command, argv, "state_load_failed", &e, 2),
    };

    let actions = state
        .get("actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    match command.as_str() {
        "status" => {
            let stale_count = actions
                .iter()
                .filter(|a| action_is_stale(a, now_epoch_ms(), policy.max_age_days))
                .count();
            let platform_count = actions
                .iter()
                .filter_map(|a| a.get("platform").and_then(Value::as_str))
                .map(ToString::to_string)
                .collect::<BTreeSet<_>>()
                .len();
            let out = lane_receipt(
                "public_api_catalog_status",
                "status",
                argv,
                json!({
                    "ok": true,
                    "action_count": actions.len(),
                    "platform_count": platform_count,
                    "connection_count": state.get("connections").and_then(Value::as_array).map(|r| r.len()).unwrap_or(0),
                    "workflow_count": state.get("workflows").and_then(Value::as_array).map(|r| r.len()).unwrap_or(0),
                    "stale_action_count": stale_count,
                    "max_age_days": policy.max_age_days,
                    "synced_epoch_ms": state.get("synced_epoch_ms").cloned().unwrap_or_else(|| json!(0)),
                    "last_verified_epoch_ms": state.get("last_verified_epoch_ms").cloned().unwrap_or_else(|| json!(0)),
                    "source_ref": state.get("source_ref").cloned().unwrap_or_else(|| json!("")),
                    "routed_via": "conduit"
                }),
                &root,
                &policy,
            );
            CommandResult {
                exit_code: 0,
                payload: out,
            }
        }
        "sync" | "run" => {
            let (incoming, source_ref) = match parse_actions(&root, argv, &policy) {
                Ok(v) => v,
                Err(e) => return err(&root, &policy, &command, argv, "sync_source_invalid", &e, 2),
            };
            if policy.strict && incoming.len() < policy.min_sync_actions {
                return err(
                    &root,
                    &policy,
                    &command,
                    argv,
                    "sync_minimum_not_met",
                    "ingested_actions_below_minimum",
                    3,
                );
            }
            let mut inserted = 0usize;
            let mut updated = 0usize;
            let total_actions = {
                let rows = state
                    .get_mut("actions")
                    .and_then(Value::as_array_mut)
                    .expect("actions array ensured");
                let mut index = BTreeMap::new();
                for (i, row) in rows.iter().enumerate() {
                    if let Some(id) = row.get("id").and_then(Value::as_str) {
                        index.insert(id.to_string(), i);
                    }
                }
                for action in incoming {
                    let id = action
                        .get("id")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    if let Some(idx) = index.get(&id).copied() {
                        rows[idx] = action;
                        updated += 1;
                    } else {
                        index.insert(id, rows.len());
                        rows.push(action);
                        inserted += 1;
                    }
                }
                rows.len()
            };
            state["synced_epoch_ms"] = json!(now_epoch_ms());
            state["source_ref"] = Value::String(source_ref.clone());
            push_event(
                &mut state,
                "sync",
                json!({"inserted":inserted,"updated":updated,"total_actions":total_actions,"source_ref":source_ref}),
            );
            if let Err(e) = save_state(&policy.state_path, &state) {
                return err(&root, &policy, &command, argv, "state_write_failed", &e, 2);
            }
            let _ = append_jsonl(
                &policy.history_path,
                &json!({
                    "ok": true,
                    "type": "public_api_catalog_sync_event",
                    "ts_epoch_ms": now_epoch_ms(),
                    "inserted": inserted,
                    "updated": updated,
                    "total_actions": total_actions,
                    "source_ref": source_ref
                }),
            );
            let out = lane_receipt(
                "public_api_catalog_sync",
                &command,
                argv,
                json!({"ok":true,"inserted":inserted,"updated":updated,"total_actions":total_actions,"source_ref":state.get("source_ref").cloned().unwrap_or_else(|| json!("")),"routed_via":"conduit"}),
                &root,
                &policy,
            );
            CommandResult {
                exit_code: 0,
                payload: out,
            }
        }
        "search" => {
            let query = parse_flag(argv, "query")
                .or_else(|| first_positional(argv, 1))
                .unwrap_or_default();
            let q = query.trim().to_ascii_lowercase();
            let limit = parse_usize(parse_flag(argv, "limit"), 10, 1, 200);
            let mut scored = actions
                .iter()
                .map(|a| {
                    let text = format!(
                        "{} {} {} {} {}",
                        a.get("id").and_then(Value::as_str).unwrap_or(""),
                        a.get("platform").and_then(Value::as_str).unwrap_or(""),
                        a.get("title").and_then(Value::as_str).unwrap_or(""),
                        a.get("description").and_then(Value::as_str).unwrap_or(""),
                        a.get("url").and_then(Value::as_str).unwrap_or("")
                    )
                    .to_ascii_lowercase();
                    let mut score = 0i64;
                    if !q.is_empty() && text.contains(&q) {
                        score += 100;
                    }
                    for token in q.split_whitespace() {
                        if token.len() > 1 && text.contains(token) {
                            score += 10;
                        }
                    }
                    (score, a)
                })
                .filter(|(score, _)| q.is_empty() || *score > 0)
                .collect::<Vec<_>>();
            scored.sort_by(|(s1, a1), (s2, a2)| {
                s2.cmp(s1).then_with(|| {
                    a1.get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .cmp(a2.get("id").and_then(Value::as_str).unwrap_or(""))
                })
            });
            let results = scored
                .into_iter()
                .take(limit)
                .map(|(score, action)| {
                    json!({
                        "id": action.get("id").cloned().unwrap_or(Value::Null),
                        "platform": action.get("platform").cloned().unwrap_or(Value::Null),
                        "title": action.get("title").cloned().unwrap_or(Value::Null),
                        "description": action.get("description").cloned().unwrap_or(Value::Null),
                        "method": action.get("method").cloned().unwrap_or(Value::Null),
                        "url": action.get("url").cloned().unwrap_or(Value::Null),
                        "tags": action.get("tags").cloned().unwrap_or_else(|| json!([])),
                        "score": score,
                        "template": action_template(action)
                    })
                })
                .collect::<Vec<_>>();
            let out = lane_receipt(
                "public_api_catalog_search",
                "search",
                argv,
                json!({"ok":true,"query":query,"result_count":results.len(),"results":results,"routed_via":"conduit"}),
                &root,
                &policy,
            );
            CommandResult {
                exit_code: 0,
                payload: out,
            }
        }
        "integrate" => {
            let action_id = parse_flag(argv, "action-id")
                .or_else(|| parse_flag(argv, "id"))
                .or_else(|| first_positional(argv, 1))
                .map(|v| clean_id(&v))
                .unwrap_or_default();
            if action_id.is_empty() {
                return err(
                    &root,
                    &policy,
                    "integrate",
                    argv,
                    "missing_action_id",
                    "expected --action-id=<id>",
                    2,
                );
            }
            let action = actions
                .iter()
                .find(|a| a.get("id").and_then(Value::as_str) == Some(action_id.as_str()));
            let Some(action) = action else {
                return err(
                    &root,
                    &policy,
                    "integrate",
                    argv,
                    "action_not_found",
                    "no_action_schema_found_for_id",
                    3,
                );
            };
            let stale = action_is_stale(action, now_epoch_ms(), policy.max_age_days);
            if policy.strict && stale {
                return err(
                    &root,
                    &policy,
                    "integrate",
                    argv,
                    "action_schema_stale",
                    "action_schema_is_stale_and_blocked_in_strict_mode",
                    4,
                );
            }
            let out = lane_receipt(
                "public_api_catalog_integrate",
                "integrate",
                argv,
                json!({"ok":true,"action_id":action_id,"platform":action.get("platform").cloned().unwrap_or(Value::Null),"stale":stale,"request_template":action_template(action),"routed_via":"conduit"}),
                &root,
                &policy,
            );
            CommandResult {
                exit_code: 0,
                payload: out,
            }
        }
        "connect" => {
            let platform = parse_flag(argv, "platform")
                .or_else(|| first_positional(argv, 1))
                .map(|v| clean_id(&v))
                .unwrap_or_default();
            if platform.is_empty() {
                return err(
                    &root,
                    &policy,
                    "connect",
                    argv,
                    "missing_platform",
                    "expected --platform=<name>",
                    2,
                );
            }
            let now_ms = now_epoch_ms();
            let key = parse_flag(argv, "connection-key")
                .map(|v| clean_id(&v))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| {
                    format!(
                        "ok_{}_{}",
                        platform,
                        &hash_fingerprint(&format!("{platform}:{now_ms}"))[7..15]
                    )
                });
            let token_fp = parse_flag(argv, "access-token")
                .filter(|v| !v.trim().is_empty())
                .map(|v| hash_fingerprint(&v));
            let refresh_fp = parse_flag(argv, "refresh-token")
                .filter(|v| !v.trim().is_empty())
                .map(|v| hash_fingerprint(&v));
            let oauth_passthrough = parse_bool(parse_flag(argv, "oauth-passthrough"), true);
            let expires_epoch_ms = parse_u64(parse_flag(argv, "expires-epoch-ms"));
            let metadata = parse_json_flag(argv, "metadata-json").unwrap_or_else(|| json!({}));
            let connections = state
                .get_mut("connections")
                .and_then(Value::as_array_mut)
                .expect("connections array ensured");
            if let Some(existing) = connections.iter_mut().find(|row| {
                row.get("connection_key").and_then(Value::as_str) == Some(key.as_str())
                    || row.get("platform").and_then(Value::as_str) == Some(platform.as_str())
            }) {
                existing["platform"] = Value::String(platform.clone());
                existing["connection_key"] = Value::String(key.clone());
                existing["oauth_passthrough"] = Value::Bool(oauth_passthrough);
                existing["expires_epoch_ms"] =
                    expires_epoch_ms.map(Value::from).unwrap_or(Value::Null);
                existing["updated_epoch_ms"] = Value::from(now_ms);
                existing["metadata"] = metadata.clone();
                if let Some(fp) = token_fp.clone() {
                    existing["token_fingerprint"] = Value::String(fp);
                }
                if let Some(fp) = refresh_fp.clone() {
                    existing["refresh_token_fingerprint"] = Value::String(fp);
                }
            } else {
                connections.push(json!({
                    "platform": platform,
                    "connection_key": key,
                    "token_fingerprint": token_fp,
                    "refresh_token_fingerprint": refresh_fp,
                    "oauth_passthrough": oauth_passthrough,
                    "expires_epoch_ms": expires_epoch_ms,
                    "created_epoch_ms": now_ms,
                    "updated_epoch_ms": now_ms,
                    "metadata": metadata
                }));
            }
            push_event(
                &mut state,
                "connect",
                json!({"platform":platform,"connection_key":key}),
            );
            if let Err(e) = save_state(&policy.state_path, &state) {
                return err(&root, &policy, "connect", argv, "state_write_failed", &e, 2);
            }
            let out = lane_receipt(
                "public_api_catalog_connect",
                "connect",
                argv,
                json!({
                    "ok": true,
                    "connection": {
                        "platform": platform,
                        "connection_key": key,
                        "token_fingerprint": token_fp,
                        "refresh_token_fingerprint": refresh_fp,
                        "oauth_passthrough": oauth_passthrough,
                        "expires_epoch_ms": expires_epoch_ms
                    },
                    "connection_count": state.get("connections").and_then(Value::as_array).map(|r| r.len()).unwrap_or(0),
                    "routed_via": "conduit"
                }),
                &root,
                &policy,
            );
            CommandResult {
                exit_code: 0,
                payload: out,
            }
        }
        "import-flow" => {
            let flow_path = parse_flag(argv, "flow-path").map(PathBuf::from);
            let flow_json = parse_json_flag(argv, "flow-json");
            let workflow_id_override = parse_flag(argv, "workflow-id").map(|v| clean_id(&v));
            let (flow, source) = if let Some(v) = flow_json {
                (v, "flow_json".to_string())
            } else if let Some(path) = flow_path {
                let resolved = if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                };
                let Some(parsed) = read_json(&resolved) else {
                    return err(
                        &root,
                        &policy,
                        "import-flow",
                        argv,
                        "flow_read_failed",
                        "flow_path_read_or_parse_failed",
                        2,
                    );
                };
                (parsed, rel(&root, &resolved))
            } else {
                return err(
                    &root,
                    &policy,
                    "import-flow",
                    argv,
                    "missing_flow",
                    "expected --flow-path=<path> or --flow-json=<json>",
                    2,
                );
            };
            let mut workflow_id = workflow_id_override
                .or_else(|| flow.get("id").and_then(Value::as_str).map(clean_id))
                .unwrap_or_else(|| format!("flow_{}", &hash_fingerprint(&source)[7..15]));
            if workflow_id.is_empty() {
                workflow_id = format!("flow_{}", now_epoch_ms());
            }
            let steps = flow
                .get("steps")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if steps.is_empty() {
                return err(
                    &root,
                    &policy,
                    "import-flow",
                    argv,
                    "workflow_steps_missing",
                    "workflow must include at least one step",
                    2,
                );
            }
            if policy.strict
                && steps.iter().any(|s| {
                    s.get("action_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .trim()
                        .is_empty()
                })
            {
                return err(
                    &root,
                    &policy,
                    "import-flow",
                    argv,
                    "workflow_step_missing_action_id",
                    "strict mode requires action_id on each step",
                    3,
                );
            }
            let normalized = json!({
                "id": workflow_id,
                "name": flow.get("name").and_then(Value::as_str).unwrap_or("").trim(),
                "description": flow.get("description").and_then(Value::as_str).unwrap_or("").trim(),
                "steps": steps,
                "source": source,
                "updated_epoch_ms": now_epoch_ms()
            });
            let workflows = state
                .get_mut("workflows")
                .and_then(Value::as_array_mut)
                .expect("workflows array ensured");
            if let Some(existing) = workflows.iter_mut().find(|row| {
                row.get("id").and_then(Value::as_str)
                    == normalized.get("id").and_then(Value::as_str)
            }) {
                *existing = normalized.clone();
            } else {
                workflows.push(normalized.clone());
            }
            push_event(
                &mut state,
                "import_flow",
                json!({"workflow_id":normalized.get("id").cloned().unwrap_or(Value::Null),"step_count":normalized.get("steps").and_then(Value::as_array).map(|r| r.len()).unwrap_or(0)}),
            );
            if let Err(e) = save_state(&policy.state_path, &state) {
                return err(
                    &root,
                    &policy,
                    "import-flow",
                    argv,
                    "state_write_failed",
                    &e,
                    2,
                );
            }
            let out = lane_receipt(
                "public_api_catalog_import_flow",
                "import-flow",
                argv,
                json!({
                    "ok": true,
                    "workflow": {
                        "id": normalized.get("id").cloned().unwrap_or(Value::Null),
                        "name": normalized.get("name").cloned().unwrap_or(Value::Null),
                        "step_count": normalized.get("steps").and_then(Value::as_array).map(|r| r.len()).unwrap_or(0),
                        "source": normalized.get("source").cloned().unwrap_or(Value::Null)
                    },
                    "workflow_count": state.get("workflows").and_then(Value::as_array).map(|r| r.len()).unwrap_or(0),
                    "routed_via": "conduit"
                }),
                &root,
                &policy,
            );
            CommandResult {
                exit_code: 0,
                payload: out,
            }
        }
        "run-flow" => {
            let workflow = if let Some(flow_path) = parse_flag(argv, "flow-path") {
                let p = PathBuf::from(flow_path);
                let resolved = if p.is_absolute() { p } else { root.join(p) };
                let Some(v) = read_json(&resolved) else {
                    return err(
                        &root,
                        &policy,
                        "run-flow",
                        argv,
                        "flow_read_failed",
                        "flow_path_read_or_parse_failed",
                        2,
                    );
                };
                v
            } else {
                let workflow_id = parse_flag(argv, "workflow-id")
                    .or_else(|| first_positional(argv, 1))
                    .map(|v| clean_id(&v))
                    .unwrap_or_default();
                if workflow_id.is_empty() {
                    return err(
                        &root,
                        &policy,
                        "run-flow",
                        argv,
                        "missing_workflow_id",
                        "expected --workflow-id=<id> or --flow-path=<path>",
                        2,
                    );
                }
                let rows = state
                    .get("workflows")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let Some(found) = rows.into_iter().find(|row| {
                    row.get("id").and_then(Value::as_str) == Some(workflow_id.as_str())
                }) else {
                    return err(
                        &root,
                        &policy,
                        "run-flow",
                        argv,
                        "workflow_not_found",
                        "workflow id not found",
                        3,
                    );
                };
                found
            };
            let context = parse_json_flag(argv, "input-json").unwrap_or_else(|| json!({}));
            let steps = workflow
                .get("steps")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if steps.is_empty() {
                return err(
                    &root,
                    &policy,
                    "run-flow",
                    argv,
                    "workflow_steps_missing",
                    "workflow has no steps",
                    2,
                );
            }
            let connections = state
                .get("connections")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut step_receipts = Vec::new();
            let mut ok_steps = 0usize;
            let mut fail_steps = 0usize;
            let mut skipped_steps = 0usize;
            for (idx, step) in steps.iter().enumerate() {
                let step_id = step
                    .get("id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("step_{:03}", idx + 1));
                let condition = step.get("condition").and_then(Value::as_str);
                if let Some(expr) = condition {
                    if !lookup_json_path(&context, expr)
                        .map(truthy)
                        .unwrap_or(false)
                    {
                        skipped_steps += 1;
                        step_receipts.push(
                            json!({"step_id":step_id,"status":"skipped","reason":"condition_false"}),
                        );
                        continue;
                    }
                }
                let action_id = step
                    .get("action_id")
                    .or_else(|| step.get("actionId"))
                    .and_then(Value::as_str)
                    .map(clean_id)
                    .unwrap_or_default();
                if action_id.is_empty() {
                    fail_steps += 1;
                    step_receipts.push(json!({"step_id":step_id,"status":"failed","errors":["step_missing_action_id"]}));
                    continue;
                }
                let action = actions
                    .iter()
                    .find(|a| a.get("id").and_then(Value::as_str) == Some(action_id.as_str()));
                let Some(action) = action else {
                    fail_steps += 1;
                    step_receipts.push(json!({"step_id":step_id,"status":"failed","errors":[format!("action_not_found:{action_id}")]}));
                    continue;
                };
                if policy.strict && action_is_stale(action, now_epoch_ms(), policy.max_age_days) {
                    fail_steps += 1;
                    step_receipts.push(json!({"step_id":step_id,"status":"failed","errors":[format!("action_stale:{action_id}")]}));
                    continue;
                }
                let platform = action
                    .get("platform")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let has_conn = connections
                    .iter()
                    .any(|c| c.get("platform").and_then(Value::as_str) == Some(platform));
                if policy.strict && !has_conn {
                    fail_steps += 1;
                    step_receipts.push(json!({"step_id":step_id,"status":"failed","errors":[format!("connection_missing:{platform}")]}));
                    continue;
                }
                ok_steps += 1;
                step_receipts.push(json!({
                    "step_id": step_id,
                    "status": "ok",
                    "action_id": action_id,
                    "platform": platform,
                    "template": action_template(action),
                    "input": step.get("input").cloned().unwrap_or_else(|| json!({})),
                    "transform": step.get("transform").cloned().unwrap_or_else(|| json!({})),
                    "routed_via": "conduit"
                }));
            }
            let total = steps.len();
            let completion_percent = if total == 0 {
                0.0
            } else {
                ((ok_steps + skipped_steps) as f64 / total as f64) * 100.0
            };
            let overall_ok = fail_steps == 0;
            if policy.strict && !overall_ok {
                return err(
                    &root,
                    &policy,
                    "run-flow",
                    argv,
                    "workflow_execution_failed",
                    "one_or_more_steps_failed_in_strict_mode",
                    4,
                );
            }
            push_event(
                &mut state,
                "run_flow",
                json!({"workflow_id":workflow.get("id").cloned().unwrap_or(Value::Null),"ok":overall_ok,"ok_steps":ok_steps,"fail_steps":fail_steps,"skipped_steps":skipped_steps,"completion_percent":completion_percent}),
            );
            if let Err(e) = save_state(&policy.state_path, &state) {
                return err(
                    &root,
                    &policy,
                    "run-flow",
                    argv,
                    "state_write_failed",
                    &e,
                    2,
                );
            }
            let _ = append_jsonl(
                &policy.history_path,
                &json!({
                    "ok": overall_ok,
                    "type": "public_api_catalog_flow_run",
                    "ts_epoch_ms": now_epoch_ms(),
                    "workflow_id": workflow.get("id").cloned().unwrap_or(Value::Null),
                    "ok_steps": ok_steps,
                    "fail_steps": fail_steps,
                    "skipped_steps": skipped_steps,
                    "completion_percent": completion_percent
                }),
            );
            let out = lane_receipt(
                "public_api_catalog_run_flow",
                "run-flow",
                argv,
                json!({"ok":overall_ok,"workflow_id":workflow.get("id").cloned().unwrap_or(Value::Null),"completion_percent":completion_percent,"ok_steps":ok_steps,"fail_steps":fail_steps,"skipped_steps":skipped_steps,"step_receipts":step_receipts,"routed_via":"conduit"}),
                &root,
                &policy,
            );
            CommandResult {
                exit_code: if overall_ok { 0 } else { 1 },
                payload: out,
            }
        }
        "verify" => {
            let mut stale = Vec::new();
            let mut invalid = Vec::new();
            for action in &actions {
                let id = action
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let platform = action.get("platform").and_then(Value::as_str).unwrap_or("");
                let url = action.get("url").and_then(Value::as_str).unwrap_or("");
                if id.trim().is_empty() || platform.trim().is_empty() || url.trim().is_empty() {
                    invalid.push(id.clone());
                }
                if action_is_stale(action, now_epoch_ms(), policy.max_age_days) {
                    stale.push(id);
                }
            }
            state["last_verified_epoch_ms"] = json!(now_epoch_ms());
            push_event(
                &mut state,
                "verify",
                json!({"stale_count":stale.len(),"invalid_count":invalid.len(),"max_age_days":policy.max_age_days}),
            );
            if let Err(e) = save_state(&policy.state_path, &state) {
                return err(&root, &policy, "verify", argv, "state_write_failed", &e, 2);
            }
            if policy.strict && (!stale.is_empty() || !invalid.is_empty()) {
                return err(
                    &root,
                    &policy,
                    "verify",
                    argv,
                    "catalog_verification_failed",
                    "stale_or_invalid_actions_detected",
                    3,
                );
            }
            let out = lane_receipt(
                "public_api_catalog_verify",
                "verify",
                argv,
                json!({"ok":true,"max_age_days":policy.max_age_days,"stale_count":stale.len(),"invalid_count":invalid.len(),"stale_actions":stale.into_iter().take(20).collect::<Vec<_>>(),"invalid_actions":invalid.into_iter().take(20).collect::<Vec<_>>(),"routed_via":"conduit"}),
                &root,
                &policy,
            );
            CommandResult {
                exit_code: 0,
                payload: out,
            }
        }
        _ => err(
            &root,
            &policy,
            &command,
            argv,
            "unknown_command",
            "unknown public-api-catalog command",
            1,
        ),
    }
}

pub fn run(cli_root: &Path, argv: &[String]) -> i32 {
    let result = run_command(cli_root, argv);
    print_json_line(&result.payload);
    result.exit_code
}
