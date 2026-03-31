fn execute_edge_bridge_message(message: EdgeBridgeMessage) -> RustEvent {
    match message {
        EdgeBridgeMessage::EdgeStatus { probe } => {
            let detail = serde_json::json!({
                "ok": true,
                "type": "edge_status",
                "probe": probe,
                "backend": edge_backend_label(),
                "available": cfg!(feature = "edge"),
                "compile_time_feature_edge": cfg!(feature = "edge")
            });
            let mut out = detail;
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            RustEvent::SystemFeedback {
                status: "edge_status".to_string(),
                detail: out,
                violation_reason: None,
            }
        }
        EdgeBridgeMessage::EdgeInference { prompt, max_tokens } => {
            #[cfg(not(feature = "edge"))]
            {
                let detail = serde_json::json!({
                    "ok": false,
                    "type": "edge_inference",
                    "backend": edge_backend_label(),
                    "reason": "edge_feature_disabled",
                    "compile_time_feature_edge": false,
                });
                let mut out = detail;
                out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
                return RustEvent::SystemFeedback {
                    status: "edge_backend_unavailable".to_string(),
                    detail: out,
                    violation_reason: Some("edge_feature_disabled".to_string()),
                };
            }
            #[cfg(feature = "edge")]
            {
                let normalized = normalize_edge_prompt(&prompt);
                let token_cap = max_tokens.unwrap_or(64).clamp(1, 256) as usize;
                let output_text = summarize_for_edge_backend(&normalized, token_cap);
                let output_tokens = output_text.split_whitespace().count() as u32;
                let mut detail = serde_json::json!({
                    "ok": true,
                    "type": "edge_inference",
                    "backend": edge_backend_label(),
                    "input": {
                        "prompt_hash": deterministic_hash(&normalized),
                        "max_tokens": token_cap,
                    },
                    "output": {
                        "text": output_text,
                        "token_count": output_tokens,
                        "truncated": normalized.split_whitespace().count() > token_cap
                    }
                });
                detail["receipt_hash"] = Value::String(deterministic_receipt_hash(&detail));
                RustEvent::SystemFeedback {
                    status: "edge_inference".to_string(),
                    detail,
                    violation_reason: None,
                }
            }
        }
        EdgeBridgeMessage::SpineCommand { args, run_context } => {
            let detail = execute_spine_bridge_command(&args, run_context.as_deref());
            let status = if detail.get("exit_code").and_then(Value::as_i64).unwrap_or(1) == 0 {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("spine_bridge_ok")
                    .to_string()
            } else {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("spine_bridge_error")
                    .to_string()
            };
            let violation_reason = if detail.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                detail
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| Some("spine_bridge_failed".to_string()))
            };
            RustEvent::SystemFeedback {
                status,
                detail,
                violation_reason,
            }
        }
        EdgeBridgeMessage::AttentionCommand { args } => {
            let detail = execute_attention_bridge_command(&args);
            let status = if detail.get("exit_code").and_then(Value::as_i64).unwrap_or(1) == 0 {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("attention_bridge_ok")
                    .to_string()
            } else {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("attention_bridge_error")
                    .to_string()
            };
            let violation_reason = if detail.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                detail
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| Some("attention_bridge_failed".to_string()))
            };
            RustEvent::SystemFeedback {
                status,
                detail,
                violation_reason,
            }
        }
        EdgeBridgeMessage::PersonaAmbientCommand { args } => {
            let detail = execute_persona_ambient_bridge_command(&args);
            let status = if detail.get("exit_code").and_then(Value::as_i64).unwrap_or(1) == 0 {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("persona_ambient_bridge_ok")
                    .to_string()
            } else {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("persona_ambient_bridge_error")
                    .to_string()
            };
            let violation_reason = if detail.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                detail
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| Some("persona_ambient_bridge_failed".to_string()))
            };
            RustEvent::SystemFeedback {
                status,
                detail,
                violation_reason,
            }
        }
        EdgeBridgeMessage::DopamineAmbientCommand { args } => {
            let detail = execute_dopamine_ambient_bridge_command(&args);
            let status = if detail.get("exit_code").and_then(Value::as_i64).unwrap_or(1) == 0 {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("dopamine_ambient_bridge_ok")
                    .to_string()
            } else {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("dopamine_ambient_bridge_error")
                    .to_string()
            };
            let violation_reason = if detail.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                detail
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| Some("dopamine_ambient_bridge_failed".to_string()))
            };
            RustEvent::SystemFeedback {
                status,
                detail,
                violation_reason,
            }
        }
        EdgeBridgeMessage::MemoryAmbientCommand { args } => {
            let detail = execute_memory_ambient_bridge_command(&args);
            let status = if detail.get("exit_code").and_then(Value::as_i64).unwrap_or(1) == 0 {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("memory_ambient_bridge_ok")
                    .to_string()
            } else {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("memory_ambient_bridge_error")
                    .to_string()
            };
            let violation_reason = if detail.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                detail
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| Some("memory_ambient_bridge_failed".to_string()))
            };
            RustEvent::SystemFeedback {
                status,
                detail,
                violation_reason,
            }
        }
        EdgeBridgeMessage::OpsDomainCommand {
            domain,
            args,
            run_context,
        } => {
            let clean_domain = domain.trim();
            if clean_domain.is_empty() {
                let detail = serde_json::json!({
                    "ok": false,
                    "type": "ops_domain_bridge_error",
                    "reason": "missing_domain",
                    "domain": domain,
                    "args": args,
                    "routed_via": "conduit"
                });
                return RustEvent::SystemFeedback {
                    status: "ops_domain_bridge_error".to_string(),
                    detail,
                    violation_reason: Some("missing_domain".to_string()),
                };
            }
            let detail = execute_ops_bridge_command(clean_domain, &args, run_context.as_deref());
            let status = if detail.get("exit_code").and_then(Value::as_i64).unwrap_or(1) == 0 {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("ops_domain_bridge_ok")
                    .to_string()
            } else {
                detail
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("ops_domain_bridge_error")
                    .to_string()
            };
            let violation_reason = if detail.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                detail
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| Some("ops_domain_bridge_failed".to_string()))
            };
            RustEvent::SystemFeedback {
                status,
                detail,
                violation_reason,
            }
        }
    }
}
