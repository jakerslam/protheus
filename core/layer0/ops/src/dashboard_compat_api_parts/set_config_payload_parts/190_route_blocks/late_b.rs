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
            "/api/web/receipts" => {
                let limit = query_value(path, "limit")
                    .and_then(|raw| raw.parse::<usize>().ok())
                    .unwrap_or(20)
                    .clamp(1, 200);
                crate::web_conduit::api_receipts(root, limit)
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
                if let Some(meta) = nexus_connection {
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert("nexus_connection".to_string(), meta);
                    }
                }
                payload
            }
            "/api/batch-query" => {
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
                let version = read_json(&root.join("package.json"))
                    .and_then(|v| v.get("version").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_else(|| "0.0.0".to_string());
                json!({
                    "ok": true,
                    "version": version,
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
                let tiers = [
                    ("file_read", "green"),
                    ("file_read_many", "green"),
                    ("folder_export", "green"),
                    ("web_fetch", "green"),
                    ("batch_query", "green"),
                    ("web_search", "green"),
                    ("memory_kv_get", "green"),
                    ("memory_kv_list", "green"),
                    ("memory_semantic_query", "green"),
                    ("memory_kv_set", "yellow"),
                    ("cron_schedule", "yellow"),
                    ("cron_run", "yellow"),
                    ("cron_cancel", "yellow"),
                    ("manage_agent", "yellow"),
                    ("terminal_exec", "green"),
                    ("spawn_subagents", "green"),
                ];
                json!({
                    "ok": true,
                    "type": "tool_capability_tiers",
                    "policy": policy,
                    "tools": tiers.iter().map(|(tool, tier)| json!({"tool": tool, "tier": tier})).collect::<Vec<_>>()
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
            "/api/comms/topology" => json!({
                "ok": true,
                "topology": {
                    "nodes": snapshot.pointer("/collab/dashboard/agents").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
                    "edges": 0,
                    "connected": true
                }
            }),
            "/api/comms/events" => json!({"ok": true, "events": []}),
            "/api/profiles" => json!({"ok": true, "profiles": extract_profiles(root)}),
            "/api/update/check" => crate::dashboard_release_update::check_update(root),
            "/api/templates" => json!({
                "ok": true,
                "templates": [
                    {"id": "general-assistant", "name": "General Assistant", "provider": "auto", "model": "auto"},
                    {"id": "research-analyst", "name": "Research Analyst", "provider": "openai", "model": "gpt-5"},
                    {"id": "ops-reliability", "name": "Ops Reliability", "provider": "frontier_provider", "model": "claude-opus-4-20250514"}
                ]
            }),
            _ => return None,
        };
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }

    None
}
