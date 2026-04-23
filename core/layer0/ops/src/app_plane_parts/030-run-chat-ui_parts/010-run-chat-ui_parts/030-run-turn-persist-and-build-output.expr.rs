{
        let hard_guard_applied = hard_guard
            .get("applied")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let tool_gate = chat_ui_turn_tool_decision_tree(&message);
        let web_search_calls = chat_ui_web_search_call_count(&tools) as i64;
        let workflow_route = clean(
            tool_gate
                .get("workflow_route")
                .and_then(Value::as_str)
                .unwrap_or("info"),
            40,
        );
        let workflow_reason_code = clean(
            tool_gate
                .get("reason_code")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            120,
        );
        let workflow_should_call_tools = tool_gate
            .get("needs_tool_access")
            .and_then(Value::as_bool)
            .or_else(|| tool_gate.get("should_call_tools").and_then(Value::as_bool))
            .unwrap_or(false);
        let workflow_retry_limit = tool_gate
            .get("workflow_retry_limit")
            .and_then(Value::as_i64)
            .unwrap_or(1);
        let workflow_selection_authority = clean(
            tool_gate
                .get("tool_selection_authority")
                .and_then(Value::as_str)
                .unwrap_or("llm_selected"),
            80,
        );
        let workflow_auto_tools_allowed = tool_gate
            .get("automatic_tool_calls_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let workflow_process_summary = json!({
            "ts": crate::now_iso(),
            "route": workflow_route.clone(),
            "reason_code": workflow_reason_code.clone(),
            "selection_authority": workflow_selection_authority.clone(),
            "automatic_tool_calls_allowed": workflow_auto_tools_allowed,
            "should_call_tools": workflow_should_call_tools,
            "tool_calls_recorded": tools.len(),
            "web_search_calls": web_search_calls,
            "classification": web_classification.clone(),
            "retry_limit": workflow_retry_limit,
            "retry_recommended": guard_retry_recommended,
            "retry_strategy": guard_retry_strategy,
            "retry_lane": guard_retry_lane
        });
        let trace_id = format!(
            "trace_{}",
            &sha256_hex_str(&format!(
                "{}:{}:{}:{}:{}",
                session_id,
                selected_provider,
                selected_model,
                message,
                crate::now_iso()
            ))[..12]
        );
        let mut discovered_tools = Vec::<String>::new();
        for row in &tools {
            let tool_name = tool_name_for_diagnostics(row);
            if !tool_name.is_empty() && !discovered_tools.iter().any(|existing| existing == &tool_name)
            {
                discovered_tools.push(tool_name);
            }
        }
        if requires_live_web && !discovered_tools.iter().any(|tool| tool == "batch_query") {
            discovered_tools.push("batch_query".to_string());
        }
        let transaction_complete = (!requires_live_web || web_classification == "healthy")
            && !hard_guard_applied;
        let transaction_status = if transaction_complete {
            "complete"
        } else if matches!(
            web_classification.as_str(),
            "low_signal" | "tool_surface_degraded"
        ) {
            "degraded"
        } else {
            "failed"
        };
        let transaction_id = format!(
            "txn_{}",
            &sha256_hex_str(&format!(
                "{}:{}:{}:{}",
                session_id, trace_id, web_classification, final_outcome
            ))[..12]
        );
        let transaction_intent = if requires_live_web {
            chat_ui_extract_web_query(&message)
        } else {
            clean(&message, 200)
        };
        let response_workflow = chat_ui_build_response_workflow_trace(
            root,
            &session_id,
            &trace_id,
            &message,
            &assistant,
            &tool_gate,
            &tools,
            &web_classification,
            &final_outcome,
            guard_retry_recommended,
            guard_retry_strategy,
            guard_retry_lane,
            hard_guard_applied,
        );
        let workflow_trace_export = response_workflow
            .get("export")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let turn = json!({
            "turn_id": format!(
                "turn_{}",
                &sha256_hex_str(&format!("{}:{}:{}:{}", session_id, selected_provider, selected_model, crate::now_iso()))[..10]
            ),
            "trace_id": trace_id,
            "ts": crate::now_iso(),
            "provider": selected_provider,
            "model": selected_model,
            "user": message,
            "assistant": assistant.clone(),
            "response_workflow": response_workflow.clone(),
            "tool_summary": receipt_summary,
            "transaction": {
                "id": transaction_id,
                "intent": transaction_intent,
                "status": transaction_status,
                "complete": transaction_complete,
                "classification": web_classification.clone(),
                "workflow_route": workflow_route.clone(),
                "workflow_reason_code": workflow_reason_code.clone(),
                "retry": {
                    "recommended": guard_retry_recommended,
                    "strategy": guard_retry_strategy,
                    "lane": guard_retry_lane,
                    "plan": guard_retry_plan.clone()
                },
                "closed_at": crate::now_iso()
            },
            "workflow_process_summary": workflow_process_summary.clone()
        });
        let mut turns = session
            .get("turns")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        turns.push(turn.clone());
        session["turns"] = Value::Array(turns);
        let mut workflow_summaries = session
            .get("workflow_summaries")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        workflow_summaries.push(workflow_process_summary.clone());
        if workflow_summaries.len() > 10 {
            let keep_from = workflow_summaries.len() - 10;
            workflow_summaries = workflow_summaries.split_off(keep_from);
        }
        session["workflow_summaries"] = Value::Array(workflow_summaries.clone());
        session["last_workflow_process_summary"] = workflow_process_summary.clone();
        session["updated_at"] = Value::String(crate::now_iso());
        let _ = write_json(&path, &session);
        let _ = append_jsonl(
            &state_root(root).join("chat_ui").join("history.jsonl"),
            &json!({"action":"run","session_id":session_id,"turn":turn,"ts":crate::now_iso()}),
        );
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "app_plane_chat_ui",
            "lane": "core/layer0/ops",
            "action": "run",
            "session_id": session_id,
            "trace_id": trace_id,
            "response": assistant.clone(),
            "content": assistant.clone(),
            "response_workflow": response_workflow.clone(),
            "turn": turn,
            "workflow_process_summary": workflow_process_summary.clone(),
            "workflow_recent_summaries": workflow_summaries.clone(),
            "provider": response.get("provider").cloned().unwrap_or_else(|| json!(provider)),
            "model": response.get("model").cloned().unwrap_or_else(|| json!(model)),
            "runtime_model": response.get("runtime_model").cloned().unwrap_or_else(|| json!(selected_model)),
            "input_tokens": response.get("input_tokens").cloned().unwrap_or_else(|| json!(0)),
            "output_tokens": response.get("output_tokens").cloned().unwrap_or_else(|| json!(0)),
            "cost_usd": response.get("cost_usd").cloned().unwrap_or_else(|| json!(0.0)),
            "context_window": response.get("context_window").cloned().unwrap_or_else(|| json!(0)),
            "tools": Value::Array(tools.clone()),
            "response_finalization": {
                "applied": true,
                "outcome": final_outcome,
                "rewrite_outcome": rewrite_outcome,
                "tool_surface_error_code": response_tool_surface_error_code,
                "final_ack_only": crate::tool_output_match_filter::matches_ack_placeholder(&assistant),
                "findings_available": chat_ui_tools_have_valid_findings(&tools),
                "tool_receipt_summary": receipt_summary,
                "tool_transaction": {
                    "id": transaction_id,
                    "intent": transaction_intent,
                    "status": transaction_status,
                    "complete": transaction_complete,
                    "classification": web_classification.clone(),
                    "retry": {
                        "recommended": guard_retry_recommended,
                        "strategy": guard_retry_strategy,
                        "lane": guard_retry_lane,
                        "plan": guard_retry_plan,
                        "suppressed_by_loop_risk": retry_suppressed_by_loop_risk,
                        "loop_risk": retry_loop_risk
                    },
                    "closed_at": crate::now_iso()
                },
                "hard_guard": hard_guard,
                "classification_guard": classification_guard,
                "tool_diagnostics": tool_diagnostics,
                "tool_gate": tool_gate,
                "workflow_trace_export": workflow_trace_export,
                "workflow_process_summary": workflow_process_summary,
                "workflow_recent_summaries": workflow_summaries,
                "capability_discovery": {
                    "contract": "tool_execution_receipt_v1",
                    "execution_statuses": ["ok", "error", "blocked", "not_found", "low_signal", "unknown"],
                    "discovered_tools": discovered_tools,
                    "recommended_tool_family": clean(
                        tool_gate.get("recommended_tool_family").and_then(Value::as_str).unwrap_or("none"),
                        80
                    ),
                    "provider_catalog": providers,
                    "selected_provider": selected_provider,
                    "selected_model": selected_model
                },
                "web_invariant": {
                    "requires_live_web": requires_live_web,
                    "tool_attempted": web_search_calls > 0,
                    "web_search_calls": web_search_calls,
                    "classification": web_classification,
                    "tool_surface_error_code": response_tool_surface_error_code,
                    "diagnostic": "forced_live_web_invariant_from_app_plane_chat_ui"
                }
            },
            "web_tooling_fallback": forced_web_fallback,
            "artifact": {
                "path": path.display().to_string(),
                "sha256": sha256_hex_str(&session.to_string())
            },
            "claim_evidence": [
                {
                    "id": "V6-APP-007.1",
                    "claim": "chat_ui_runs_multi_provider_conversation_with_receipted_model_calls",
                    "evidence": {
                        "provider": settings.get("provider"),
                        "model": settings.get("model"),
                        "session_id": session_id
                    }
                }
            ]
        });
        if !forced_web_error_code.is_empty() {
            out["error"] = Value::String(forced_web_error_code);
        }
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        out
}
