fn run_ticket(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        TICKET_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "company_ticket_audit_contract",
            "allowed_ops": ["create", "assign", "transition", "handoff", "close", "status"],
            "require_tool_call_trace_link": true
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("company_ticket_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "company_ticket_audit_contract"
    {
        errors.push("company_ticket_contract_kind_invalid".to_string());
    }

    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let allowed = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .any(|v| v == op);
    if strict && !allowed {
        errors.push("company_ticket_op_invalid".to_string());
    }
    let runtime_web_tooling = company_ticket_runtime_web_tooling_snapshot();
    let runtime_web_tooling_auth_present = runtime_web_tooling
        .get("auth_present")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let requires_web_tooling = parse_bool(parsed.flags.get("requires-web-tooling"), false);
    if strict && requires_web_tooling && !runtime_web_tooling_auth_present {
        errors.push("company_ticket_runtime_web_tooling_auth_missing".to_string());
    }

    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let mut ledger = read_json(&ticket_state_path(root, &team)).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "teams": {}
        })
    });
    ensure_ticket_ledger_shape(&mut ledger);
    if !ledger["teams"].get(&team).is_some() {
        ledger["teams"][&team] = json!({
            "tickets": {},
            "updated_at": crate::now_iso()
        });
    }
    if !ledger["teams"][&team]
        .get("tickets")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        ledger["teams"][&team]["tickets"] = Value::Object(serde_json::Map::new());
    }
    let tickets_obj = ledger["teams"][&team]["tickets"]
        .as_object()
        .cloned()
        .unwrap_or_default();

    let ticket_id = clean(
        parsed
            .flags
            .get("ticket-id")
            .cloned()
            .or_else(|| parsed.flags.get("id").cloned())
            .or_else(|| parsed.positional.get(2).cloned())
            .unwrap_or_default(),
        80,
    );
    if strict && op != "create" && op != "status" && ticket_id.is_empty() {
        errors.push("company_ticket_id_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "company_plane_ticket",
            "errors": errors
        });
    }

    if op == "status" {
        let ticket = if ticket_id.is_empty() {
            Value::Null
        } else {
            tickets_obj.get(&ticket_id).cloned().unwrap_or(Value::Null)
        };
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "company_plane_ticket",
            "lane": "core/layer0/ops",
            "team": team,
            "op": op,
            "ticket_id": if ticket_id.is_empty() { Value::Null } else { Value::String(ticket_id) },
            "ticket": ticket,
            "runtime_web_tooling": runtime_web_tooling.clone(),
            "claim_evidence": [
                {
                    "id": "V6-COMPANY-001.3",
                    "claim": "ticket_status_returns_receipted_task_chain_state",
                    "evidence": {
                        "team": team
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    let existing = if ticket_id.is_empty() {
        Value::Null
    } else {
        tickets_obj.get(&ticket_id).cloned().unwrap_or(Value::Null)
    };
    let existing_state = existing
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("open")
        .to_string();
    let existing_assignee = existing
        .get("assignee")
        .and_then(Value::as_str)
        .unwrap_or("unassigned")
        .to_string();

    let require_trace = contract
        .get("require_tool_call_trace_link")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let tool_call_id = clean(
        parsed
            .flags
            .get("tool-call-id")
            .cloned()
            .or_else(|| parsed.flags.get("trace-id").cloned())
            .unwrap_or_else(|| {
                format!("tool_{}", &sha256_hex_str(&format!("{}:{op}", team))[..10])
            }),
        120,
    );
    if strict && require_trace && tool_call_id.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "company_plane_ticket",
            "errors": ["company_ticket_tool_call_trace_required"]
        });
    }

    let resolved_ticket_id = if op == "create" {
        if !ticket_id.is_empty() {
            ticket_id.clone()
        } else {
            format!(
                "TKT-{}",
                &sha256_hex_str(&format!("{}:{}", team, crate::now_iso()))[..12]
            )
        }
    } else {
        ticket_id.clone()
    };
    if strict && op == "create" && tickets_obj.contains_key(&resolved_ticket_id) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "company_plane_ticket",
            "errors": ["company_ticket_already_exists"]
        });
    }
    if strict && op != "create" && !tickets_obj.contains_key(&resolved_ticket_id) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "company_plane_ticket",
            "errors": ["company_ticket_not_found"]
        });
    }

    let mut ticket = existing
        .as_object()
        .cloned()
        .map(Value::Object)
        .unwrap_or_else(|| {
            json!({
                "ticket_id": resolved_ticket_id,
                "team": team,
                "title": clean(parsed.flags.get("title").cloned().unwrap_or_else(|| "Untitled Ticket".to_string()), 200),
                "state": "open",
                "assignee": parsed.flags.get("assignee").cloned().unwrap_or_else(|| "unassigned".to_string()),
                "created_at": crate::now_iso(),
                "updated_at": crate::now_iso(),
                "trace_chain_length": 0u64,
                "last_event_hash": "genesis"
            })
        });

    let mut event_details = json!({});
    match op.as_str() {
        "create" => {
            ticket["title"] = Value::String(clean(
                parsed
                    .flags
                    .get("title")
                    .cloned()
                    .unwrap_or_else(|| "Untitled Ticket".to_string()),
                200,
            ));
            if let Some(assignee) = parsed.flags.get("assignee") {
                ticket["assignee"] = Value::String(clean(assignee, 120));
            }
            ticket["state"] = Value::String("open".to_string());
            ticket["created_at"] = Value::String(crate::now_iso());
            event_details = json!({
                "title": ticket.get("title").cloned().unwrap_or(Value::Null),
                "initial_assignee": ticket.get("assignee").cloned().unwrap_or(Value::Null)
            });
        }
        "assign" => {
            let assignee = clean(
                parsed
                    .flags
                    .get("assignee")
                    .cloned()
                    .or_else(|| parsed.flags.get("to").cloned())
                    .unwrap_or_else(|| existing_assignee.clone()),
                120,
            );
            ticket["assignee"] = Value::String(assignee.clone());
            event_details = json!({
                "from_assignee": existing_assignee,
                "to_assignee": assignee
            });
        }
        "transition" => {
            let to_state = clean(
                parsed
                    .flags
                    .get("to")
                    .cloned()
                    .or_else(|| parsed.flags.get("state").cloned())
                    .unwrap_or_else(|| existing_state.clone()),
                80,
            );
            ticket["state"] = Value::String(to_state.clone());
            event_details = json!({
                "from_state": existing_state,
                "to_state": to_state
            });
        }
        "handoff" => {
            let from_assignee = clean(
                parsed
                    .flags
                    .get("from")
                    .cloned()
                    .unwrap_or_else(|| existing_assignee.clone()),
                120,
            );
            let to_assignee = clean(
                parsed
                    .flags
                    .get("to")
                    .cloned()
                    .or_else(|| parsed.flags.get("assignee").cloned())
                    .unwrap_or_else(|| existing_assignee.clone()),
                120,
            );
            ticket["assignee"] = Value::String(to_assignee.clone());
            event_details = json!({
                "from_assignee": from_assignee,
                "to_assignee": to_assignee
            });
        }
        "close" => {
            ticket["state"] = Value::String("closed".to_string());
            ticket["closed_at"] = Value::String(crate::now_iso());
            event_details = json!({
                "from_state": existing_state,
                "to_state": "closed"
            });
        }
        _ => {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "company_plane_ticket",
                "errors": ["company_ticket_op_invalid"]
            });
        }
    }

    let prev_hash = ticket
        .get("last_event_hash")
        .and_then(Value::as_str)
        .unwrap_or("genesis")
        .to_string();
    let mut event = json!({
        "version": "v1",
        "team": team,
        "ticket_id": resolved_ticket_id,
        "op": op,
        "tool_call_id": tool_call_id,
        "prev_event_hash": prev_hash,
        "ts": crate::now_iso(),
        "details": event_details
    });
    let event_hash = sha256_hex_str(&event.to_string());
    event["event_hash"] = Value::String(event_hash.clone());
    let event_path = ticket_history_path(root, &team);
    let _ = append_jsonl(&event_path, &event);
    let history_rows = read_json_lines(&event_path)
        .into_iter()
        .filter(|row| row.get("ticket_id").and_then(Value::as_str) == Some(&resolved_ticket_id))
        .collect::<Vec<_>>();
    let (chain_valid, chain_issues) = validate_ticket_history_rows(&history_rows);
    if strict && !chain_valid {
        return json!({
            "ok": false,
            "strict": true,
            "type": "company_plane_ticket",
            "errors": ["company_ticket_chain_validation_failed"],
            "chain_issues": chain_issues
        });
    }

    let chain_len = ticket
        .get("trace_chain_length")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(1);
    ticket["trace_chain_length"] = Value::Number(serde_json::Number::from(chain_len));
    ticket["last_event_hash"] = Value::String(event_hash.clone());
    ticket["updated_at"] = Value::String(crate::now_iso());

    ledger["teams"][&team]["tickets"][&resolved_ticket_id] = ticket.clone();
    ledger["teams"][&team]["updated_at"] = Value::String(crate::now_iso());
    let ledger_path = ticket_state_path(root, &team);
    let _ = write_json(&ledger_path, &ledger);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "company_plane_ticket",
        "lane": "core/layer0/ops",
        "team": team,
        "op": op,
        "ticket_id": resolved_ticket_id,
        "ticket": ticket,
        "runtime_web_tooling": runtime_web_tooling.clone(),
        "audit_event": event,
        "artifact": {
            "ledger_path": ledger_path.display().to_string(),
            "history_path": event_path.display().to_string(),
            "ledger_sha256": sha256_hex_str(&ledger.to_string()),
            "event_sha256": event_hash,
            "chain_valid": chain_valid,
            "chain_issues": chain_issues
        },
        "claim_evidence": [
            {
                "id": "V6-COMPANY-001.3",
                "claim": "ticket_lifecycle_ops_emit_immutable_audit_chain_with_tool_trace_linkage",
                "evidence": {
                    "team": team,
                    "ticket_id": resolved_ticket_id,
                    "chain_length": chain_len,
                    "chain_valid": chain_valid
                }
            },
            {
                "id": "V6-COMPANY-001.4",
                "claim": "ticket_ops_surface_runtime_web_tooling_auth_readiness_for_operator_triage",
                "evidence": {
                    "auth_present": runtime_web_tooling_auth_present,
                    "auth_sources_count": runtime_web_tooling.get("auth_sources").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn company_ticket_runtime_web_tooling_snapshot() -> Value {
    let env_candidates = [
        "BRAVE_API_KEY",
        "EXA_API_KEY",
        "TAVILY_API_KEY",
        "PERPLEXITY_API_KEY",
        "SERPAPI_API_KEY",
        "GOOGLE_SEARCH_API_KEY",
        "GOOGLE_CSE_ID",
        "FIRECRAWL_API_KEY",
        "XAI_API_KEY",
        "MOONSHOT_API_KEY",
        "OPENAI_API_KEY",
    ];
    let mut auth_sources = Vec::<String>::new();
    for env_name in env_candidates {
        let present = std::env::var(env_name)
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        if present {
            auth_sources.push(format!("env:{env_name}"));
        }
    }
    json!({
        "strict_auth_required": std::env::var("INFRING_WEB_TOOLING_STRICT_AUTH")
            .ok()
            .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "y" | "on"))
            .unwrap_or(true),
        "auth_present": !auth_sources.is_empty(),
        "auth_sources": auth_sources
    })
}
