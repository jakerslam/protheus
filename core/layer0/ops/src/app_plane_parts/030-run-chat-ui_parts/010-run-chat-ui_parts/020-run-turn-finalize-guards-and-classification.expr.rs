{
        let (assistant_initial, response_finalization_outcome) =
            finalize_chat_ui_assistant_response(&message, &assistant_raw, &tools);
        let tool_diagnostics = chat_ui_tool_diagnostics(&tools);
        let receipt_summary =
            chat_ui_semantic_receipt_summary(&tool_diagnostics, requires_live_web, &message);
        let (assistant_rewritten, rewrite_outcome) =
            rewrite_chat_ui_placeholder_with_tool_diagnostics(&assistant_initial, &tool_diagnostics);
        let mut assistant = assistant_rewritten;
        let mut hard_guard = json!({
            "applied": false
        });
        let inline_tool_schema = chat_ui_inline_tool_call_schema(&assistant);
        let inline_tool_call_detected = inline_tool_schema
            .get("detected")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let inline_tool_schema_valid = inline_tool_schema
            .get("schema_valid")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let inline_tool_schema_repaired = inline_tool_schema
            .get("schema_repaired")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let inline_tool_name = clean(
            inline_tool_schema
                .get("tool")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        if inline_tool_call_detected {
            let inline_error_code = if inline_tool_schema_valid {
                if inline_tool_schema_repaired {
                    "inline_tool_call_schema_repaired_suppressed"
                } else {
                    "inline_tool_call_suppressed"
                }
            } else {
                "inline_tool_call_schema_invalid"
            };
            let detail = if inline_tool_name.is_empty() {
                None
            } else {
                Some(format!("tool={inline_tool_name}"))
            };
            assistant = crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "parse_failed",
                inline_error_code,
                detail.as_deref(),
            );
            hard_guard = json!({
                "applied": true,
                "status": "parse_failed",
                "error_code": inline_error_code,
                "source": "inline_tool_call_guard",
                "schema_valid": inline_tool_schema_valid,
                "schema_repaired": inline_tool_schema_repaired,
                "tool": if inline_tool_name.is_empty() { Value::Null } else { json!(inline_tool_name) }
            });
            if forced_web_error_code.is_empty() {
                forced_web_error_code = inline_error_code.to_string();
            }
        }
        let mut routing_claim_guard_applied = false;
        if chat_ui_contains_unverified_routing_root_cause_claim(&assistant)
            && !chat_ui_has_structured_routing_claim_evidence(&tools)
        {
            assistant = crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "parse_failed",
                "web_tool_unverified_routing_claim",
                Some("missing_receipt_evidence"),
            );
            hard_guard = json!({
                "applied": true,
                "status": "parse_failed",
                "error_code": "web_tool_unverified_routing_claim",
                "source": "claim_evidence_guard"
            });
            routing_claim_guard_applied = true;
            if forced_web_error_code.is_empty() {
                forced_web_error_code = "web_tool_unverified_routing_claim".to_string();
            }
        }
        let web_search_calls = chat_ui_web_search_call_count(&tools) as i64;
        if assistant.trim().is_empty()
            || crate::tool_output_match_filter::matches_ack_placeholder(&assistant)
            || crate::tool_output_match_filter::contains_forbidden_runtime_context_markers(&assistant)
        {
            let (fallback_status, fallback_error_code) = chat_ui_fallback_status_error_for_diagnostics(
                &tool_diagnostics,
                requires_live_web,
                web_search_calls,
            );
            assistant = crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                fallback_status,
                fallback_error_code,
                None,
            );
            hard_guard = json!({
                "applied": true,
                "status": fallback_status,
                "error_code": fallback_error_code
            });
            if forced_web_error_code.is_empty() {
                forced_web_error_code = fallback_error_code.to_string();
            }
        }
        let blocked_calls = tool_diagnostics
            .get("blocked_calls")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let blocked_error_calls = tool_diagnostics
            .pointer("/error_codes/web_tool_policy_blocked")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let blocked_receipt_calls = chat_ui_receipt_status_count(&tool_diagnostics, "blocked");
        let not_found_error_calls = tool_diagnostics
            .pointer("/error_codes/web_tool_not_found")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let not_found_receipt_calls = chat_ui_receipt_status_count(&tool_diagnostics, "not_found");
        let low_signal_calls = tool_diagnostics
            .get("low_signal_calls")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let no_result_calls = tool_diagnostics
            .get("no_result_calls")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let low_signal_error_calls = tool_diagnostics
            .pointer("/error_codes/web_tool_low_signal")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let low_signal_receipt_calls = chat_ui_receipt_status_count(&tool_diagnostics, "low_signal");
        let surface_unavailable_calls = tool_diagnostics
            .get("surface_unavailable_calls")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let surface_degraded_calls = tool_diagnostics
            .get("surface_degraded_calls")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let blocked_signal = blocked_calls > 0 || blocked_error_calls > 0 || blocked_receipt_calls > 0;
        let workflow_gate_blocked =
            forced_web_error_code == "workflow_gate_blocked_web_tooling"
                || tool_diagnostics
                    .pointer("/error_codes/workflow_gate_blocked_web_tooling")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    > 0;
        let blocked_signal = blocked_signal || workflow_gate_blocked;
        let not_found_signal = not_found_error_calls > 0 || not_found_receipt_calls > 0;
        let forced_surface_error_hint = if matches!(
            forced_web_error_code.as_str(),
            "web_tool_surface_unavailable" | "web_tool_surface_degraded"
        ) {
            Some(forced_web_error_code.clone())
        } else {
            None
        };
        let finalization_inferred_surface_error = if response_finalization_outcome
            == "tool_surface_error_fail_closed"
        {
            detected_tool_surface_error
                .clone()
                .or_else(|| forced_surface_error_hint.clone())
                .or_else(|| Some("web_tool_surface_degraded".to_string()))
        } else {
            None
        };
        let tool_surface_error_code = detected_tool_surface_error
            .clone()
            .or_else(|| forced_surface_error_hint.clone())
            .or(finalization_inferred_surface_error)
            .or_else(|| {
                if surface_unavailable_calls > 0 {
                    Some("web_tool_surface_unavailable".to_string())
                } else if surface_degraded_calls > 0 {
                    Some("web_tool_surface_degraded".to_string())
                } else {
                    None
                }
            });
        let low_signal = low_signal_calls > 0
            || no_result_calls > 0
            || low_signal_error_calls > 0
            || low_signal_receipt_calls > 0;
        let mut web_classification = if let Some(surface_error) = tool_surface_error_code.as_deref() {
            chat_ui_tool_surface_classification(surface_error).to_string()
        } else if blocked_signal {
            "policy_blocked".to_string()
        } else if requires_live_web && web_search_calls == 0 {
            "tool_not_invoked".to_string()
        } else if not_found_signal {
            "tool_not_found".to_string()
        } else if low_signal {
            "low_signal".to_string()
        } else if requires_live_web {
            "healthy".to_string()
        } else {
            "not_required".to_string()
        };
        let expected_web_classification = chat_ui_expected_classification_from_diagnostics(
            &tool_diagnostics,
            requires_live_web,
            web_search_calls,
        );
        let classification_consistent = web_classification == expected_web_classification;
        let selected_classification_error_code =
            chat_ui_error_code_for_classification(&web_classification).to_string();
        let expected_classification_error_code =
            chat_ui_error_code_for_classification(expected_web_classification).to_string();
        let mut classification_active_error_code = if classification_consistent {
            selected_classification_error_code.clone()
        } else {
            expected_classification_error_code.clone()
        };
        let mut classification_guard = json!({
            "applied": !classification_consistent,
            "selected": web_classification.clone(),
            "expected": expected_web_classification,
            "consistent": classification_consistent,
            "mode": if classification_consistent { "none" } else { "override" },
            "selected_error_code": if selected_classification_error_code.is_empty() { Value::Null } else { json!(selected_classification_error_code) },
            "expected_error_code": if expected_classification_error_code.is_empty() { Value::Null } else { json!(expected_classification_error_code) },
            "active_error_code": Value::Null,
            "retry_recommended": false,
            "retry_strategy": "none",
            "retry_lane": "none",
            "not_invoked_fail_closed": false,
            "fail_closed": false,
            "fail_closed_class": null,
            "inline_tool_call_detected": inline_tool_call_detected,
            "inline_tool_call_schema_valid": if inline_tool_call_detected { json!(inline_tool_schema_valid) } else { Value::Null },
            "inline_tool_call_schema_repaired": if inline_tool_call_detected { json!(inline_tool_schema_repaired) } else { Value::Null },
            "routing_claim_guard_applied": routing_claim_guard_applied
        });
        if let Some(guard) = classification_guard.as_object_mut() {
            if !classification_active_error_code.is_empty() {
                guard.insert(
                    "active_error_code".to_string(),
                    json!(classification_active_error_code),
                );
            }
        }
        if !classification_consistent {
            web_classification = expected_web_classification.to_string();
        }
        let assistant_placeholder_like = assistant.trim().is_empty()
            || crate::tool_output_match_filter::matches_ack_placeholder(&assistant)
            || crate::tool_output_match_filter::contains_forbidden_runtime_context_markers(&assistant);
        let assistant_context_mismatch = !chat_ui_response_matches_previous_message(&message, &assistant)
            || chat_ui_contains_kernel_patch_thread_dump(&message, &assistant)
            || chat_ui_contains_role_preamble_prompt_dump(&message, &assistant)
            || chat_ui_contains_competitive_programming_dump(&message, &assistant);
        let classification_findings_available = chat_ui_tools_have_valid_findings(&tools);
        let classification_should_fail_close = requires_live_web
            && matches!(
                web_classification.as_str(),
                "workflow_gate_blocked"
                    | "tool_surface_unavailable"
                    | "tool_surface_degraded"
                    | "tool_not_invoked"
                    | "policy_blocked"
                    | "tool_not_found"
                    | "low_signal"
            )
            && (assistant_placeholder_like
                || !classification_findings_available
                || assistant_context_mismatch);
        if classification_should_fail_close {
            let fallback_status = chat_ui_fallback_status_for_classification(&web_classification);
            let fallback_error_code = chat_ui_error_code_for_classification(&web_classification);
            if !fallback_error_code.is_empty() {
                assistant = crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                    fallback_status,
                    fallback_error_code,
                    None,
                );
                hard_guard = json!({
                    "applied": true,
                    "status": fallback_status,
                    "error_code": fallback_error_code,
                    "classification": web_classification,
                    "source": "classification_guard"
                });
                if forced_web_error_code.is_empty() {
                    forced_web_error_code = fallback_error_code.to_string();
                }
                classification_active_error_code = fallback_error_code.to_string();
                if let Some(guard) = classification_guard.as_object_mut() {
                    guard.insert("applied".to_string(), json!(true));
                    guard.insert("mode".to_string(), json!("fail_close"));
                    guard.insert("fail_closed".to_string(), json!(true));
                    guard.insert("fail_closed_class".to_string(), json!(web_classification));
                    guard.insert(
                        "active_error_code".to_string(),
                        json!(classification_active_error_code),
                    );
                    if matches!(
                        web_classification.as_str(),
                        "tool_not_invoked" | "workflow_gate_blocked"
                    ) {
                        guard.insert("not_invoked_fail_closed".to_string(), json!(true));
                    }
                }
            }
        }
        if !requires_live_web
            && assistant_context_mismatch
            && !hard_guard
                .get("applied")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            assistant = "I could not produce a reliable response for your last message in this turn. Please retry and I will answer directly without running tools.".to_string();
            hard_guard = json!({
                "applied": true,
                "status": "failed",
                "error_code": "assistant_context_mismatch",
                "classification": "info_route_context_mismatch",
                "source": "coherence_guard"
            });
            if forced_web_error_code.is_empty() {
                forced_web_error_code = "assistant_context_mismatch".to_string();
            }
        }
        let mut final_outcome = if forced_web_outcome.is_empty() {
            response_finalization_outcome.clone()
        } else {
            forced_web_outcome.clone()
        };
        let hard_guard_applied = hard_guard
            .get("applied")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let hard_guard_source = hard_guard.get("source").and_then(Value::as_str).unwrap_or("");
        if hard_guard_source == "classification_guard" {
            let classification_error_code = chat_ui_error_code_for_classification(&web_classification);
            if !classification_error_code.is_empty() {
                forced_web_error_code = classification_error_code.to_string();
                classification_active_error_code = classification_error_code.to_string();
                if let Some(guard) = classification_guard.as_object_mut() {
                    guard.insert(
                        "active_error_code".to_string(),
                        json!(classification_active_error_code),
                    );
                }
            }
        }
        let retry_loop_risk = tool_diagnostics
            .get("loop_risk")
            .cloned()
            .unwrap_or_else(|| chat_ui_retry_loop_risk_from_diagnostics(&tool_diagnostics));
        let (base_guard_retry_recommended, base_guard_retry_strategy, base_guard_retry_lane) =
            chat_ui_retry_profile_for_guard(&classification_active_error_code, &web_classification);
        let (guard_retry_recommended, guard_retry_strategy, guard_retry_lane) =
            chat_ui_apply_loop_risk_to_retry(
                base_guard_retry_recommended,
                base_guard_retry_strategy,
                base_guard_retry_lane,
                &retry_loop_risk,
            );
        let retry_suppressed_by_loop_risk = base_guard_retry_recommended && !guard_retry_recommended;
        let mut guard_retry_plan = chat_ui_retry_plan_for_guard(
            guard_retry_recommended,
            guard_retry_strategy,
            guard_retry_lane,
        );
        if let Some(plan) = guard_retry_plan.as_object_mut() {
            plan.insert("loop_risk".to_string(), retry_loop_risk.clone());
            plan.insert(
                "suppressed_by_loop_risk".to_string(),
                json!(retry_suppressed_by_loop_risk),
            );
        }
        if let Some(guard) = classification_guard.as_object_mut() {
            guard.insert(
                "retry_recommended".to_string(),
                json!(guard_retry_recommended),
            );
            guard.insert("retry_strategy".to_string(), json!(guard_retry_strategy));
            guard.insert("retry_lane".to_string(), json!(guard_retry_lane));
            guard.insert("retry_plan".to_string(), guard_retry_plan.clone());
            guard.insert(
                "retry_suppressed_by_loop_risk".to_string(),
                json!(retry_suppressed_by_loop_risk),
            );
            guard.insert("retry_loop_risk".to_string(), retry_loop_risk.clone());
        }
        if hard_guard_applied {
            final_outcome = if hard_guard_source == "classification_guard" {
                let guard_class = clean(
                    hard_guard
                        .get("classification")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    80,
                )
                .replace('-', "_");
                if guard_class == "tool_not_invoked" {
                    "classification_guard_not_invoked_fail_closed".to_string()
                } else {
                    format!("classification_guard_{guard_class}_fail_closed")
                }
            } else {
                "hard_guard_fallback".to_string()
            };
        }
        if !classification_consistent && !hard_guard_applied {
            final_outcome = "classification_guard_overrode".to_string();
        }
        if final_outcome == "tool_surface_error_fail_closed" {
            final_outcome = chat_ui_tool_surface_forced_outcome(
                tool_surface_error_code
                    .as_deref()
                    .unwrap_or("web_tool_surface_degraded"),
            )
            .to_string();
        }
        if assistant.trim().is_empty()
            || crate::tool_output_match_filter::matches_ack_placeholder(&assistant)
            || crate::tool_output_match_filter::contains_forbidden_runtime_context_markers(&assistant)
        {
            assistant = "I could not produce a reliable final answer in this turn. Please retry once; if it still fails, I will return a structured fail-closed diagnosis.".to_string();
            if !hard_guard
                .get("applied")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                hard_guard = json!({
                    "applied": true,
                    "status": "failed",
                    "error_code": "assistant_output_not_reliable",
                    "source": "final_output_guard"
                });
            }
            if forced_web_error_code.is_empty() {
                forced_web_error_code = "assistant_output_not_reliable".to_string();
            }
            if final_outcome == "unchanged" || final_outcome == "finalized" {
                final_outcome = "final_output_guard_fail_closed".to_string();
            }
        }
        if forced_web_error_code.is_empty() {
            if let Some(surface_error) = tool_surface_error_code.as_deref() {
                forced_web_error_code = surface_error.to_string();
            }
        }
        if forced_web_error_code.is_empty() {
            let classification_error_code = chat_ui_error_code_for_classification(&web_classification);
            if !classification_error_code.is_empty() {
                forced_web_error_code = classification_error_code.to_string();
            }
        }
        let response_tool_surface_error_code = if matches!(
            forced_web_error_code.as_str(),
            "web_tool_surface_unavailable" | "web_tool_surface_degraded"
        ) {
            Some(forced_web_error_code.clone())
        } else {
            tool_surface_error_code.clone()
        };

    (
        assistant,
        hard_guard,
        forced_web_error_code,
        final_outcome,
        response_tool_surface_error_code,
        tool_diagnostics,
        receipt_summary,
        rewrite_outcome,
        web_classification,
        guard_retry_recommended,
        guard_retry_strategy,
        guard_retry_lane,
        guard_retry_plan,
        retry_suppressed_by_loop_risk,
        retry_loop_risk,
        classification_guard,
    )
}
