fn chat_ui_workflow_stage_progress(stages: &[Value], active_stage: &str) -> Value {
    let total = stages.len().max(1) as i64;
    let active_index = stages
        .iter()
        .position(|row| row.get("stage").and_then(Value::as_str) == Some(active_stage))
        .map(|idx| idx as i64 + 1)
        .unwrap_or(total);
    let progress_pct = ((active_index as f64 / total as f64) * 100.0).round() as i64;
    json!({
        "current_stage": active_stage,
        "stage_index": active_index,
        "stage_total": total,
        "progress_pct": progress_pct.clamp(0, 100)
    })
}

fn chat_ui_tool_execution_stream(rows: &[Value]) -> Vec<Value> {
    rows.iter()
        .enumerate()
        .map(|(idx, row)| {
            let name = clean(row.get("name").and_then(Value::as_str).unwrap_or("tool"), 80);
            let status = clean(row.get("status").and_then(Value::as_str).unwrap_or("unknown"), 80)
                .to_ascii_lowercase();
            let blocked = matches!(status.as_str(), "blocked" | "policy_denied");
            let ok = row
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or_else(|| status == "ok");
            let error = clean(row.get("error").and_then(Value::as_str).unwrap_or(""), 240);
            json!({
                "seq": idx + 1,
                "ts": crate::now_iso(),
                "tool": if name.is_empty() { "tool" } else { &name },
                "status": status,
                "ok": ok,
                "blocked": blocked,
                "error_code": if error.is_empty() { Value::Null } else { json!(error) },
                "result_preview": clean(row.get("result").and_then(Value::as_str).unwrap_or(""), 220)
            })
        })
        .collect()
}

fn chat_ui_render_workflow_timeline(workflow_trace: &Value) -> String {
    let mut lines = vec![
        "# response_workflow timeline".to_string(),
        format!(
            "trace_id: {}",
            clean(
                workflow_trace
                    .get("trace_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                160
            )
        ),
        format!(
            "workflow: {}",
            clean(
                workflow_trace
                    .pointer("/selected_workflow/name")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                160
            )
        ),
    ];
    if let Some(states) = workflow_trace
        .pointer("/trace_streams/workflow_state")
        .and_then(Value::as_array)
    {
        lines.push(String::new());
        lines.push("## workflow_state".to_string());
        for row in states {
            let stage = clean(row.get("stage").and_then(Value::as_str).unwrap_or(""), 120);
            let status = clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 120);
            let note = clean(row.get("note").and_then(Value::as_str).unwrap_or(""), 320);
            lines.push(format!("- {stage} [{status}] {note}"));
        }
    }
    if let Some(status_rows) = workflow_trace
        .pointer("/trace_streams/ui_status")
        .and_then(Value::as_array)
    {
        lines.push(String::new());
        lines.push("## ui_status".to_string());
        for row in status_rows {
            let message = clean(row.get("message").and_then(Value::as_str).unwrap_or(""), 320);
            lines.push(format!("- {message}"));
        }
    }
    lines.join("\n")
}

fn chat_ui_append_response_workflow_export(root: &Path, workflow_trace: &Value) -> Value {
    let chat_state_dir = state_root(root).join("chat_ui");
    let _ = fs::create_dir_all(&chat_state_dir);
    let jsonl_path = chat_state_dir.join("workflow_trace_history.jsonl");
    let json_path = chat_state_dir.join("workflow_trace_latest.json");
    let timeline_path = chat_state_dir.join("workflow_trace_latest.timeline.txt");
    let _ = append_jsonl(&jsonl_path, workflow_trace);
    let _ = write_json(&json_path, workflow_trace);
    let timeline = chat_ui_render_workflow_timeline(workflow_trace);
    let _ = fs::write(&timeline_path, timeline.as_bytes());
    json!({
        "formats": ["json", "jsonl", "timeline"],
        "jsonl_path": jsonl_path.display().to_string(),
        "json_path": json_path.display().to_string(),
        "timeline_path": timeline_path.display().to_string(),
        "payload_sha256": sha256_hex_str(&workflow_trace.to_string()),
        "timeline_sha256": sha256_hex_str(&timeline),
    })
}

fn chat_ui_build_response_workflow_trace(
    root: &Path,
    session_id: &str,
    trace_id: &str,
    message: &str,
    assistant: &str,
    tool_gate: &Value,
    tools: &[Value],
    web_classification: &str,
    final_outcome: &str,
    guard_retry_recommended: bool,
    guard_retry_strategy: &str,
    guard_retry_lane: &str,
    hard_guard_applied: bool,
) -> Value {
    let needs_tool_access = !tools.is_empty()
        || tool_gate
            .get("needs_tool_access")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let reason_code = clean(
        tool_gate
            .get("reason_code")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        120,
    );
    let workflow_gate_mode = clean(
        tool_gate
            .get("gate_decision_mode")
            .and_then(Value::as_str)
            .unwrap_or("manual_need_tools_yes_no"),
        40,
    );
    let requires_live_web = tool_gate
        .get("requires_live_web")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let selected_tool_family = clean(
        tool_gate
            .get("selected_tool_family")
            .and_then(Value::as_str)
            .unwrap_or("unselected"),
        80,
    );
    let tool_family_menu = tool_gate
        .get("tool_family_menu")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let tool_menu = tool_gate
        .get("tool_menu")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let findings_available = chat_ui_tools_have_valid_findings(tools);
    let post_tool_decision = if needs_tool_access && findings_available {
        "awaiting_finish_or_another_tool_submission"
    } else if needs_tool_access {
        "awaiting_tool_submission"
    } else {
        "no_tool_path"
    };
    let post_tool_loop_target = if needs_tool_access && findings_available {
        "post_tool_menu"
    } else if needs_tool_access {
        "tool_menu"
    } else {
        "llm_final_output"
    };
    let mut workflow_state = vec![json!({
        "seq": 1,
        "stage": "need_tool_access_gate",
        "status": if needs_tool_access { "submitted_true" } else { "submitted_false_or_not_used" },
        "note": "presented options=Yes,No",
        "ts": crate::now_iso()
    })];
    if needs_tool_access {
        workflow_state.push(json!({
            "seq": 2,
            "stage": "tool_family_selection",
            "status": if selected_tool_family == "unselected" || selected_tool_family == "none" {
                "awaiting_llm_submission"
            } else {
                "submitted"
            },
            "note": format!("submitted_family={selected_tool_family}"),
            "ts": crate::now_iso()
        }));
        workflow_state.push(json!({
            "seq": 3,
            "stage": "tool_selection",
            "status": if tools.is_empty() { "awaiting_llm_submission" } else { "submitted_or_executed" },
            "note": "numbered menu only",
            "ts": crate::now_iso()
        }));
        workflow_state.push(json!({
            "seq": 4,
            "stage": "tool_execution",
            "status": if tools.is_empty() { "blocked" } else { "completed" },
            "note": format!("tool_calls_recorded={}", tools.len()),
            "ts": crate::now_iso()
        }));
        workflow_state.push(json!({
            "seq": 5,
            "stage": "post_tool_gate",
            "status": if findings_available { "awaiting_llm_submission" } else { "not_reached" },
            "note": "presented options=1,2",
            "ts": crate::now_iso()
        }));
    } else {
        workflow_state.push(json!({
            "seq": 2,
            "stage": "direct_response",
            "status": "submitted_false_or_not_used",
            "note": "no tool menu consumed",
            "ts": crate::now_iso()
        }));
    }
    workflow_state.push(json!({
        "seq": workflow_state.len() + 1,
        "stage": "result_packaging",
        "status": "completed",
        "note": format!("outcome={final_outcome}"),
        "ts": crate::now_iso()
    }));

    let mut ui_status = vec![json!({
        "seq": 1,
        "ts": crate::now_iso(),
            "message": "Workflow gate presented: Need tools? Yes/No",
        "stage": "need_tool_access_gate"
    })];
    if !needs_tool_access {
        ui_status.push(json!({
            "seq": 2,
            "ts": crate::now_iso(),
            "message": "No tool execution recorded for this turn.",
            "stage": "direct_response"
        }));
    } else if selected_tool_family == "web_tools" {
        ui_status.push(json!({
            "seq": 2,
            "ts": crate::now_iso(),
            "message": "Web tool menu selection submitted.",
            "stage": "tool_family_selection"
        }));
        let criteria = chat_ui_extract_web_query(message);
        ui_status.push(json!({
            "seq": 3,
            "ts": crate::now_iso(),
            "message": format!("Tool data submitted: \"{}\"", clean(criteria, 200)),
            "stage": "tool_execution"
        }));
    } else if selected_tool_family == "file_tools" {
        ui_status.push(json!({
            "seq": 2,
            "ts": crate::now_iso(),
            "message": "File/workspace tool menu selection submitted.",
            "stage": "tool_family_selection"
        }));
    } else {
        ui_status.push(json!({
            "seq": 2,
            "ts": crate::now_iso(),
            "message": "Tool menu selection submitted or pending.",
            "stage": "tool_family_selection"
        }));
    }
    ui_status.push(json!({
        "seq": ui_status.len() + 1,
        "ts": crate::now_iso(),
        "message": if needs_tool_access && findings_available {
            "Post-tool gate presented: 1) finish 2) another tool."
        } else if needs_tool_access {
            "Tool request field awaiting LLM submission."
        } else {
            "Direct response path selected or pending."
        },
        "stage": "post_tool_gate"
    }));

    let decision_summary = vec![
        json!({
            "seq": 1,
            "ts": crate::now_iso(),
            "decision": "need_tool_access",
            "value": needs_tool_access,
            "reason_code": reason_code,
            "selection_source": "llm_submission_or_observed_tool_execution"
        }),
        json!({
            "seq": 2,
            "ts": crate::now_iso(),
            "decision": "tool_family_selection",
            "value": selected_tool_family,
            "reason_code": "menu_only",
            "selection_source": "llm_menu_or_unselected"
        }),
        json!({
            "seq": 3,
            "ts": crate::now_iso(),
            "decision": "post_tool_gate",
            "value": post_tool_decision,
            "diagnostic_classification": clean(web_classification, 80),
            "final_outcome": clean(final_outcome, 120),
            "guard_retry_observed": guard_retry_recommended,
            "guard_retry_strategy": clean(guard_retry_strategy, 80),
            "guard_retry_lane": clean(guard_retry_lane, 80)
        }),
    ];

    let tool_execution = chat_ui_tool_execution_stream(tools);
    let active_stage = if needs_tool_access && !findings_available {
        "request_payload_entry"
    } else if needs_tool_access && findings_available {
        "post_tool_gate"
    } else {
        "llm_final_output"
    };
    let selected_workflow_name = if needs_tool_access {
        "complex_prompt_chain_v1"
    } else {
        "simple_conversation_v1"
    };
    let mut workflow_trace = json!({
        "contract": "response_workflow_control_plane_trace_v1",
        "trace_id": trace_id,
        "session_id": session_id,
        "selected_workflow": {
            "name": selected_workflow_name,
            "gate_contract": "tool_menu_interface_v1",
            "manual_tool_selection": true
        },
        "selected": {
            "name": selected_workflow_name
        },
        "gates": {
            "gate_mode": {
                "mode": workflow_gate_mode,
                "reason_code": reason_code.clone(),
                "requires_live_web": requires_live_web
            },
            "need_tool_access": {
                "question": "Need tools? Yes/No",
                "required": false,
                "reason_code": reason_code,
                "selected_tool_family": selected_tool_family,
                "selection_authority": "llm_submission_only",
                "tool_family_menu": tool_family_menu,
                "tool_menu": tool_menu,
                "loop_mode": "numbered_menu"
            },
            "post_tool_decision": {
                "decision": post_tool_decision,
                "loop_target": post_tool_loop_target,
                "selection_authority": "llm_submission_only",
                "guard_retry_observed": guard_retry_recommended,
                "guard_retry_strategy": clean(guard_retry_strategy, 80),
                "guard_retry_lane": clean(guard_retry_lane, 80)
            }
        },
        "process_position": chat_ui_workflow_stage_progress(&workflow_state, active_stage),
        "trace_streams": {
            "workflow_state": workflow_state,
            "ui_status": ui_status,
            "decision_summary": decision_summary,
            "tool_execution": tool_execution
        },
        "response": clean(assistant, 20_000),
        "final_llm_response": {
            "required": true,
            "status": if clean(assistant, 200).is_empty() { "failed" } else { "synthesized" },
            "source": if hard_guard_applied { "non_llm_candidate_withheld" } else { "llm_authored" }
        }
    });
    let export = chat_ui_append_response_workflow_export(root, &workflow_trace);
    if let Some(obj) = workflow_trace.as_object_mut() {
        obj.insert("export".to_string(), export);
    }
    workflow_trace
}
