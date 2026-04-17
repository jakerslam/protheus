fn finalize_global_status_tool_payload(
    root: &Path,
    tool_name: &str,
    tool_input: &Value,
    payload: &mut Value,
    nexus_connection: Option<Value>,
) {
    crate::dashboard_tool_turn_loop::annotate_tool_payload_tracking(
        root,
        "dashboard-api",
        tool_name,
        payload,
    );
    let audit_receipt = append_tool_decision_audit(
        root,
        "dashboard-api",
        tool_name,
        tool_input,
        payload,
        "none",
    );
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "recovery_strategy".to_string(),
            Value::String("none".to_string()),
        );
        obj.insert("recovery_attempts".to_string(), json!(0));
        obj.insert(
            "decision_audit_receipt".to_string(),
            Value::String(audit_receipt),
        );
        if let Some(meta) = nexus_connection {
            obj.insert("nexus_connection".to_string(), meta);
        }
    }
}

fn first_available_json_payload(root: &Path, rel_candidates: &[&str]) -> Value {
    for rel in rel_candidates {
        if let Some(payload) = read_json_loose(&root.join(rel)) {
            return payload;
        }
    }
    Value::Null
}

fn web_tooling_operator_summary_payload(root: &Path) -> Value {
    let runtime_status = crate::web_conduit::api_status(root);
    let runtime_contract = compat_runtime_web_tooling_contract_snapshot("api_web_tooling_summary");
    let soak_report = first_available_json_payload(
        root,
        &[
            "artifacts/web_tooling_context_soak_report_latest.json",
            "local/state/ops/web_tooling_context_soak/latest.json",
            "core/local/artifacts/web_tooling_context_soak_report_latest.json",
        ],
    );
    let taxonomy = soak_report
        .get("taxonomy")
        .cloned()
        .unwrap_or_else(|| json!({"parse_error": "taxonomy_missing"}));
    let replay_pack = soak_report
        .get("replay_pack")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let taxonomy_parse_error = clean_text(
        taxonomy
            .get("parse_error")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let unresolved_taxonomy_count =
        taxonomy.get("empty_final").and_then(Value::as_i64).unwrap_or(0).max(0)
            + taxonomy
                .get("deferred_final")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0)
            + taxonomy
                .get("placeholder_final")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0)
            + taxonomy
                .get("off_topic_final")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0)
            + taxonomy
                .get("meta_status_tool_leak")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0)
            + taxonomy
                .get("web_missing_tool_attempt")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0);
    let runtime_ok = runtime_status
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let auth_present = runtime_contract
        .get("auth_present")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let readiness = runtime_ok
        && auth_present
        && taxonomy_parse_error.is_empty()
        && unresolved_taxonomy_count == 0;
    json!({
        "ok": true,
        "type": "web_tooling_operator_summary",
        "readiness": readiness,
        "runtime_status": runtime_status,
        "runtime_contract": runtime_contract,
        "taxonomy_snapshot": taxonomy,
        "replay_pack": replay_pack,
        "unresolved_taxonomy_count": unresolved_taxonomy_count,
        "taxonomy_parse_error": taxonomy_parse_error,
        "ts": crate::now_iso()
    })
}

fn dashboard_agent_template_catalog() -> Vec<Value> {
    vec![
        json!({"id":"general-assistant","name":"General Assistant","description":"Balanced helper for everyday questions and planning.","role":"assistant","provider":"auto","model":"auto","system_prompt":"You are a helpful general assistant. Give direct, practical answers and ask clarifying questions only when needed."}),
        json!({"id":"research-analyst","name":"Research Analyst","description":"Evidence-first researcher for synthesis and comparison.","role":"researcher","provider":"openai","model":"gpt-5","system_prompt":"You are a research analyst. Structure findings clearly, separate facts from inference, and call out uncertainty."}),
        json!({"id":"ops-reliability","name":"Ops Reliability","description":"Reliability-focused operator for incidents and hardening.","role":"ops_engineer","provider":"frontier_provider","model":"claude-opus-4-20250514","system_prompt":"You are an operations reliability engineer. Prioritize safety, rollback plans, and verifiable execution."}),
        json!({"id":"travel-assistant","name":"Travel Assistant","description":"Plans itineraries, compares options, and keeps logistics clear.","role":"travel_assistant","provider":"auto","model":"auto","system_prompt":"You are a travel assistant. Build clear itinerary options with tradeoffs, costs, and timing details."}),
        json!({"id":"real-estate-agent","name":"Real Estate Agent","description":"Supports listing analysis, buyer and seller flows, and negotiation prep.","role":"real_estate_agent","provider":"auto","model":"auto","system_prompt":"You are a real estate assistant. Compare properties, surface risk factors, and provide concise next-step guidance."}),
    ]
}

fn toml_basic_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn toml_multiline_escape(value: &str) -> String {
    value.replace("\\", "\\\\").replace("\"\"\"", "\"\"\\\"")
}

fn dashboard_template_manifest_toml(template: &Value) -> String {
    let name = clean_text(template.get("name").and_then(Value::as_str).unwrap_or("Agent"), 120);
    let role = clean_text(
        template.get("role").and_then(Value::as_str).unwrap_or("assistant"),
        80,
    );
    let provider = clean_text(
        template.get("provider").and_then(Value::as_str).unwrap_or("auto"),
        80,
    );
    let model = clean_text(
        template.get("model").and_then(Value::as_str).unwrap_or("auto"),
        160,
    );
    let prompt = clean_text(
        template
            .get("system_prompt")
            .and_then(Value::as_str)
            .unwrap_or("You are a helpful assistant."),
        6_000,
    );
    format!(
        "name = \"{}\"\nrole = \"{}\"\n\n[model]\nprovider = \"{}\"\nmodel = \"{}\"\n\n[prompt]\nsystem = \"\"\"\n{}\n\"\"\"\n",
        toml_basic_escape(&name),
        toml_basic_escape(&role),
        toml_basic_escape(&provider),
        toml_basic_escape(&model),
        toml_multiline_escape(&prompt),
    )
}

fn handle_global_status_get_routes(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    snapshot: &Value,
    request_host: &str,
    usage: &Value,
    runtime: &Value,
    status: &str,
) -> Option<CompatApiResponse> {
    if method == "GET" && path_only.starts_with("/api/templates/") {
        let template_id = clean_text(path_only.trim_start_matches("/api/templates/"), 120);
        let normalized_id = template_id.trim_matches('/').to_ascii_lowercase();
        if normalized_id.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "template_id_required"}),
            });
        }
        if let Some(template) = dashboard_agent_template_catalog().into_iter().find(|row| {
            clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 120)
                .to_ascii_lowercase()
                == normalized_id
        }) {
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "template": template,
                    "manifest_toml": dashboard_template_manifest_toml(&template)
                }),
            });
        }
        return Some(CompatApiResponse {
            status: 404,
            payload: json!({
                "ok": false,
                "error": "template_not_found",
                "template_id": normalized_id
            }),
        });
    }
    if method == "GET" {
        let payload = match path_only {
            "/api/health" => json!({
                "ok": true,
                "status": status,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({"count": 0, "checks": []})),
                "dashboard_metrics": snapshot.pointer("/health/dashboard_metrics").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime,
                "receipt_hash": snapshot.get("receipt_hash").cloned().unwrap_or(Value::Null),
                "ts": crate::now_iso()
            }),
            "/api/usage" => {
                json!({"ok": true, "agents": usage["agents"].clone(), "summary": usage["summary"].clone(), "by_model": usage["models"].clone(), "daily": usage["daily"].clone()})
            }
            "/api/usage/summary" => {
                let mut summary = usage["summary"].clone();
                summary["ok"] = json!(true);
                summary
            }
            "/api/usage/by-model" => json!({"ok": true, "models": usage["models"].clone()}),
            "/api/usage/daily" => json!({
                "ok": true,
                "days": usage["daily"].clone(),
                "today_cost_usd": usage["today_cost_usd"].clone(),
                "first_event_date": usage["first_event_date"].clone()
            }),
            "/api/status" => status_payload(root, snapshot, &request_host),
            "/api/web/status" => crate::web_conduit::api_status(root),
            "/api/web/tooling/summary" => web_tooling_operator_summary_payload(root),
            "/api/web/receipts" => {
                let limit = query_value(path, "limit")
                    .and_then(|raw| raw.parse::<usize>().ok())
                    .unwrap_or(20)
                    .clamp(1, 200);
                crate::web_conduit::api_receipts(root, limit)
            }
            _ if path_only.starts_with("/api/web/media/") => {
                let hosted_id = clean_text(path_only.trim_start_matches("/api/web/media/"), 220);
                crate::web_conduit::api_media_host_read(root, &hosted_id)
            }
            "/api/web/search" => {
                let nexus_connection =
                    match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                        "web_search",
                    ) {
                        Ok(meta) => meta,
                        Err(err) => {
                            return Some(CompatApiResponse {
                                status: 403,
                                payload: json!({
                                    "ok": false,
                                    "error": "web_search_nexus_delivery_denied",
                                    "message": "Web search blocked by hierarchical nexus ingress policy.",
                                    "nexus_error": clean_text(&err, 240)
                                }),
                            })
                        }
                    };
                let query = clean_text(
                    query_value(path, "q")
                        .or_else(|| query_value(path, "query"))
                        .as_deref()
                        .unwrap_or(""),
                    600,
                );
                let args = json!({"query": query, "summary_only": false});
                let trace_id = crate::deterministic_receipt_hash(&json!({
                    "tool": "web_search",
                    "query": args.get("query").cloned().unwrap_or(Value::Null),
                    "route": "api_web_search_get"
                }));
                let task_id = format!(
                    "tool-web-search-{}",
                    trace_id.chars().take(12).collect::<String>()
                );
                let pipeline = tooling_pipeline_execute(
                    &trace_id,
                    &task_id,
                    "web_search",
                    &args,
                    |normalized_args| Ok(crate::web_conduit::api_search(root, normalized_args)),
                );
                let mut payload = pipeline
                    .get("raw_payload")
                    .cloned()
                    .unwrap_or_else(|| json!({"ok": false, "error": "tool_pipeline_failed"}));
                if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    attach_tool_pipeline(&mut payload, &pipeline);
                }
                finalize_global_status_tool_payload(
                    root,
                    "web_search",
                    &args,
                    &mut payload,
                    nexus_connection,
                );
                payload
            }
            "/api/batch-query" => {
                let nexus_connection =
                    match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                        "batch_query",
                    ) {
                        Ok(meta) => meta,
                        Err(err) => {
                            return Some(CompatApiResponse {
                                status: 403,
                                payload: json!({
                                    "ok": false,
                                    "error": "batch_query_nexus_delivery_denied",
                                    "message": "Batch query blocked by hierarchical nexus ingress policy.",
                                    "nexus_error": clean_text(&err, 240)
                                }),
                            })
                        }
                    };
                let source =
                    clean_text(query_value(path, "source").as_deref().unwrap_or("web"), 40);
                let query = clean_text(
                    query_value(path, "q")
                        .or_else(|| query_value(path, "query"))
                        .as_deref()
                        .unwrap_or(""),
                    600,
                );
                let aperture = clean_text(
                    query_value(path, "aperture").as_deref().unwrap_or("medium"),
                    20,
                );
                let args = json!({
                    "source": source,
                    "query": query,
                    "aperture": aperture
                });
                let trace_id = crate::deterministic_receipt_hash(&json!({
                    "tool": "batch_query",
                    "query": args.get("query").cloned().unwrap_or(Value::Null),
                    "route": "api_batch_query_get"
                }));
                let task_id = format!(
                    "tool-batch-query-{}",
                    trace_id.chars().take(12).collect::<String>()
                );
                let pipeline = tooling_pipeline_execute(
                    &trace_id,
                    &task_id,
                    "batch_query",
                    &args,
                    |normalized_args| {
                        Ok(crate::batch_query_primitive::api_batch_query(
                            root,
                            normalized_args,
                        ))
                    },
                );
                let mut payload = pipeline
                    .get("raw_payload")
                    .cloned()
                    .unwrap_or_else(|| json!({"status":"blocked","error":"tool_pipeline_failed"}));
                if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    attach_tool_pipeline(&mut payload, &pipeline);
                }
                finalize_global_status_tool_payload(
                    root,
                    "batch_query",
                    &args,
                    &mut payload,
                    nexus_connection,
                );
                payload
            }
            "/api/telemetry/alerts" => proactive_telemetry_alerts_payload(root, snapshot),
            "/api/continuity" | "/api/continuity/pending" => {
                continuity_pending_payload(root, snapshot)
            }
            "/api/config" => config_payload(root, snapshot),
            "/api/config/schema" => config_schema_payload(),
            "/api/auth/check" => auth_check_payload(),
            "/api/providers" => providers_payload(root, snapshot),
            "/api/models" => crate::dashboard_model_catalog::catalog_payload(root, snapshot),
            "/api/models/recommended" => crate::dashboard_model_catalog::route_decision_payload(
                root,
                snapshot,
                &json!({"task_type":"general","budget_mode":"balanced"}),
            ),
            "/api/route/auto" => crate::dashboard_model_catalog::route_decision_payload(
                root,
                snapshot,
                &json!({"task_type":"general","budget_mode":"balanced"}),
            ),
            "/api/route/decision" => {
                crate::dashboard_model_catalog::route_decision_payload(root, snapshot, &json!({}))
            }
            "/api/channels" => dashboard_compat_api_channels::channels_payload(root),
            "/api/audit/recent" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "entries": entries, "tip_hash": tip_hash})
            }
            "/api/audit/decisions" => {
                let limit = query_value(path, "limit")
                    .and_then(|raw| raw.parse::<usize>().ok())
                    .unwrap_or(20)
                    .clamp(1, 200);
                let rows = read_jsonl_loose(&tool_decision_audit_path(root), limit);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"rows": rows}));
                json!({"ok": true, "type": "tool_decision_audit_rows", "rows": rows, "tip_hash": tip_hash})
            }
            "/api/audit/verify" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "valid": true, "entries": entries.len(), "tip_hash": tip_hash})
            }
            "/api/version" => {
                let version_info = dashboard_runtime_version_info(root);
                json!({
                    "ok": true,
                    "version": version_info.get("version").and_then(Value::as_str).unwrap_or("0.0.0"),
                    "tag": version_info.get("tag").and_then(Value::as_str).unwrap_or("v0.0.0"),
                    "source": version_info.get("source").and_then(Value::as_str).unwrap_or("fallback_default"),
                    "rust_authority": "rust_core_lanes",
                    "platform": std::env::consts::OS,
                    "arch": std::env::consts::ARCH
                })
            }
            "/api/security" => json!({
                "ok": true,
                "mode": "strict",
                "fail_closed": true,
                "receipts_required": true,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime
            }),
            "/api/capabilities/status" => {
                let policy = tool_governance_policy(root);
                let broker = protheus_tooling_core_v1::ToolBroker::default();
                let catalog = broker.capability_catalog();
                let grouped_catalog = broker.grouped_capability_catalog();
                json!({
                    "ok": true,
                    "type": "tool_capability_tiers",
                    "policy": policy,
                    "catalog_contract": "domain_grouped_tool_catalog_v1",
                    "catalog_default_workflow": "complex_prompt_chain_v1",
                    "catalog_domains": grouped_catalog,
                    "tools": catalog.iter().map(|row| {
                        let tier = match row.status {
                            protheus_tooling_core_v1::ToolCapabilityStatus::Available => {
                                if row.read_only { "green" } else { "yellow" }
                            }
                            protheus_tooling_core_v1::ToolCapabilityStatus::Degraded => "yellow",
                            protheus_tooling_core_v1::ToolCapabilityStatus::Blocked => "red",
                            protheus_tooling_core_v1::ToolCapabilityStatus::Unavailable => "gray",
                        };
                        json!({
                            "tool": row.tool_name,
                            "tier": tier,
                            "domain": row.domain,
                            "backend": row.backend,
                            "read_only": row.read_only,
                            "discoverable": row.discoverable
                        })
                    }).collect::<Vec<_>>()
                })
            }
            "/api/tools" => json!({
                "ok": true,
                "tools": [
                    {"name": "protheus-ops", "category": "runtime"},
                    {"name": "infringd", "category": "runtime"},
                    {"name": "web_conduit", "category": "runtime"},
                    {"name": "git", "category": "cli"},
                    {"name": "rg", "category": "cli"}
                ],
                "runtime_sync": runtime
            }),
            "/api/commands" => json!({
                "ok": true,
                "commands": [
                    {"cmd": "/status", "command": "/status", "desc": "Show runtime status and cockpit summary", "description": "Show runtime status and cockpit summary"},
                    {"cmd": "/queue", "command": "/queue", "desc": "Show current queue pressure", "description": "Show current queue pressure"},
                    {"cmd": "/context", "command": "/context", "desc": "Show context and attention state", "description": "Show context and attention state"},
                    {"cmd": "/model", "command": "/model", "desc": "Inspect or switch model (/model [name])", "description": "Inspect or switch model (/model [name])"},
                    {"cmd": "/file <path>", "command": "/file <path>", "desc": "Render full file output in chat from workspace path", "description": "Render full file output in chat from workspace path"},
                    {"cmd": "/folder <path>", "command": "/folder <path>", "desc": "Render folder tree + downloadable archive in chat", "description": "Render folder tree + downloadable archive in chat"},
                    {"cmd": "/alerts", "command": "/alerts", "desc": "Show proactive telemetry alerts", "description": "Show proactive telemetry alerts"},
                    {"cmd": "/continuity", "command": "/continuity", "desc": "Show pending actions across sessions/channels/tasks", "description": "Show pending actions across sessions/channels/tasks"},
                    {"cmd": "/browse <url>", "command": "/browse <url>", "desc": "Fetch and summarize a web URL via governed web conduit", "description": "Fetch and summarize a web URL via governed web conduit"},
                    {"cmd": "/search <query>", "command": "/search <query>", "desc": "Search the web with governed web conduit and summarize results", "description": "Search the web with governed web conduit and summarize results"},
                    {"cmd": "/batch <query>", "command": "/batch <query>", "desc": "Run governed batch query primitive (source=web, aperture=medium)", "description": "Run governed batch query primitive (source=web, aperture=medium)"},
                    {"cmd": "/capabilities", "command": "/capabilities", "desc": "Show active tool capabilities and read surfaces", "description": "Show active tool capabilities and read surfaces"},
                    {"cmd": "/cron", "command": "/cron list | /cron schedule <interval> <message> | /cron run <job_id> | /cron cancel <job_id>", "desc": "Manage agent-owned scheduled jobs", "description": "Manage agent-owned scheduled jobs"},
                    {"cmd": "/memory query <text>", "command": "/memory query <text>", "desc": "Semantic memory lookup over persisted KV entries", "description": "Semantic memory lookup over persisted KV entries"},
                    {"cmd": "/undo", "command": "/undo", "desc": "Undo the last conversational turn with receipted rollback", "description": "Undo the last conversational turn with receipted rollback"},
                    {"cmd": "/aliases", "command": "/aliases", "desc": "List active slash command aliases", "description": "List active slash command aliases"},
                    {"cmd": "/alias", "command": "/alias <shortcut> <target>", "desc": "Create a custom slash alias", "description": "Create a custom slash alias"}
                ]
            }),
            "/api/budget" => json!({
                "ok": true,
                "hourly_spend": 0,
                "daily_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "monthly_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "hourly_limit": 0,
                "daily_limit": 0,
                "monthly_limit": 0
            }),
            "/api/sessions" => {
                json!({"ok": true, "sessions": session_summary_rows(root, snapshot)})
            }
            "/api/profiles" => json!({"ok": true, "profiles": extract_profiles(root)}),
            "/api/update/check" => crate::dashboard_release_update::check_update(root),
            "/api/templates" => json!({
                "ok": true,
                "templates": dashboard_agent_template_catalog()
            }),
            _ => return None,
        };
        let status = if path_only.starts_with("/api/web/media/") {
            match payload.get("error").and_then(Value::as_str).unwrap_or("") {
                "invalid-path" => 400,
                "outside-workspace" => 400,
                "expired" => 410,
                "too-large" => 413,
                "not-found" => 404,
                _ => {
                    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                        200
                    } else {
                        400
                    }
                }
            }
        } else {
            200
        };
        return Some(CompatApiResponse { status, payload });
    }

    None
}
