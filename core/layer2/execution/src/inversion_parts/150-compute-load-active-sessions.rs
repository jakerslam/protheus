
fn normalize_active_session_rows(rows: &[Value], max_rows: usize) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    let mut seen_ids = std::collections::BTreeSet::<String>::new();
    for row in rows {
        if out.len() >= max_rows {
            break;
        }
        let Some(mut obj) = row.as_object().cloned() else {
            continue;
        };
        let session_id = clean_text_runtime(
            &value_to_string(
                obj.get("session_id")
                    .or_else(|| obj.get("sessionId"))
                    .or_else(|| obj.get("id")),
            ),
            120,
        );
        if !session_id.is_empty() {
            let dedupe_key = session_id.to_ascii_lowercase();
            if !seen_ids.insert(dedupe_key) {
                continue;
            }
            obj.insert("session_id".to_string(), Value::String(session_id));
        }
        let status = clean_text_runtime(&value_to_string(obj.get("status")), 32).to_ascii_lowercase();
        if status.is_empty() {
            obj.insert("status".to_string(), Value::String("active".to_string()));
        } else {
            obj.insert("status".to_string(), Value::String(status));
        }
        out.push(Value::Object(obj));
    }
    out
}

pub fn compute_load_active_sessions(input: &LoadActiveSessionsInput) -> LoadActiveSessionsOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let payload = compute_read_json(&ReadJsonInput {
        file_path: input.file_path.clone(),
        fallback: Some(Value::Null),
    })
    .value;
    let sessions_raw = payload
        .as_object()
        .and_then(|m| m.get("sessions"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let sessions = normalize_active_session_rows(&sessions_raw, 5000);
    let updated_at = {
        let value = value_to_string(payload.as_object().and_then(|m| m.get("updated_at")));
        if value.is_empty() || chrono::DateTime::parse_from_rfc3339(value.as_str()).is_err() {
            now_iso
        } else {
            clean_text_runtime(&value, 64)
        }
    };
    LoadActiveSessionsOutput {
        store: json!({
            "schema_id": "inversion_active_sessions",
            "schema_version": "1.0",
            "updated_at": updated_at,
            "sessions": sessions
        }),
    }
}

pub fn compute_save_active_sessions(input: &SaveActiveSessionsInput) -> SaveActiveSessionsOutput {
    let now_iso = input.now_iso.clone().unwrap_or_else(now_iso_runtime);
    let sessions_raw = input
        .store
        .as_ref()
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("sessions"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let sessions = normalize_active_session_rows(&sessions_raw, 5000);
    let out = json!({
        "schema_id": "inversion_active_sessions",
        "schema_version": "1.0",
        "updated_at": now_iso,
        "sessions": sessions
    });
    let _ = compute_write_json_atomic(&WriteJsonAtomicInput {
        file_path: input.file_path.clone(),
        value: Some(out.clone()),
    });
    SaveActiveSessionsOutput { store: out }
}
pub fn compute_emit_event(input: &EmitEventInput) -> EmitEventOutput {
    if !input.emit_events.unwrap_or(false) {
        return EmitEventOutput {
            emitted: false,
            file_path: None,
        };
    }
    let events_dir = input.events_dir.as_deref().unwrap_or("").trim();
    let date_str = clean_text_runtime(input.date_str.as_deref().unwrap_or(""), 32);
    if events_dir.is_empty() || date_str.is_empty() {
        return EmitEventOutput {
            emitted: false,
            file_path: None,
        };
    }
    let fp = Path::new(events_dir).join(format!("{date_str}.jsonl"));
    let event = {
        let token = normalize_token_runtime(input.event_type.as_deref().unwrap_or(""), 64);
        if token.is_empty() {
            "unknown".to_string()
        } else {
            token
        }
    };
    let row = json!({
        "ts": input.now_iso.clone().unwrap_or_else(now_iso_runtime),
        "type": "inversion_event",
        "event": event,
        "payload": input.payload.clone().unwrap_or_else(|| json!({}))
    });
    let _ = compute_append_jsonl(&AppendJsonlInput {
        file_path: Some(fp.to_string_lossy().to_string()),
        row: Some(row),
    });
    EmitEventOutput {
        emitted: true,
        file_path: Some(fp.to_string_lossy().to_string()),
    }
}

pub fn compute_append_persona_lens_gate_receipt(
    input: &AppendPersonaLensGateReceiptInput,
) -> AppendPersonaLensGateReceiptOutput {
    let payload = input.payload.as_ref().and_then(|v| v.as_object());
    if !to_bool_like(payload.and_then(|m| m.get("enabled")), false) {
        return AppendPersonaLensGateReceiptOutput { rel_path: None };
    }
    let mut target_path = clean_text_runtime(input.cfg_receipts_path.as_deref().unwrap_or(""), 420);
    if target_path.is_empty() {
        let state_dir = clean_text_runtime(input.state_dir.as_deref().unwrap_or(""), 420);
        target_path = Path::new(&state_dir)
            .join("lens_gate_receipts.jsonl")
            .to_string_lossy()
            .to_string();
    }
    let decision = input.decision.as_ref().and_then(|v| v.as_object());
    let feed_push = payload
        .and_then(|m| m.get("feed_push"))
        .and_then(|v| v.as_object());
    let persona_id = {
        let value = clean_text_runtime(
            &value_to_string(payload.and_then(|m| m.get("persona_id"))),
            120,
        );
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let mode = {
        let value = clean_text_runtime(&value_to_string(payload.and_then(|m| m.get("mode"))), 24);
        if value.is_empty() {
            "auto".to_string()
        } else {
            value
        }
    };
    let effective_mode = {
        let value = clean_text_runtime(
            &value_to_string(payload.and_then(|m| m.get("effective_mode"))),
            24,
        );
        if value.is_empty() {
            "shadow".to_string()
        } else {
            value
        }
    };
    let status = {
        let value = clean_text_runtime(&value_to_string(payload.and_then(|m| m.get("status"))), 32);
        if value.is_empty() {
            "unknown".to_string()
        } else {
            value
        }
    };
    let reasons = payload
        .and_then(|m| m.get("reasons"))
        .and_then(|v| v.as_array())
        .map(|rows| rows.iter().take(8).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let feed_push_value = if let Some(feed) = feed_push {
        let reason = {
            let value = clean_text_runtime(&value_to_string(feed.get("reason")), 120);
            if value.is_empty() {
                Value::Null
            } else {
                Value::String(value)
            }
        };
        let feed_path = {
            let value = clean_text_runtime(&value_to_string(feed.get("feed_path")), 220);
            if value.is_empty() {
                Value::Null
            } else {
                Value::String(value)
            }
        };
        let receipts_path = {
            let value = clean_text_runtime(&value_to_string(feed.get("receipts_path")), 220);
            if value.is_empty() {
                Value::Null
            } else {
                Value::String(value)
            }
        };
        let entry_hash = {
            let value = clean_text_runtime(&value_to_string(feed.get("entry_hash")), 120);
            if value.is_empty() {
                Value::Null
            } else {
                Value::String(value)
            }
        };
        json!({
            "pushed": to_bool_like(feed.get("pushed"), false),
            "reason": reason,
            "feed_path": feed_path,
            "receipts_path": receipts_path,
            "entry_hash": entry_hash
        })
    } else {
        Value::Null
    };
    let objective = {
        let value = clean_text_runtime(
            &value_to_string(
                decision
                    .and_then(|m| m.get("input"))
                    .and_then(|v| v.as_object())
                    .and_then(|m| m.get("objective")),
            ),
            260,
        );
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let target = {
        let value = clean_text_runtime(
            &value_to_string(
                decision
                    .and_then(|m| m.get("input"))
                    .and_then(|v| v.as_object())
                    .and_then(|m| m.get("target")),
            ),
            40,
        );
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let impact = {
        let value = clean_text_runtime(
            &value_to_string(
                decision
                    .and_then(|m| m.get("input"))
                    .and_then(|v| v.as_object())
                    .and_then(|m| m.get("impact")),
            ),
            40,
        );
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let row = json!({
        "ts": input.now_iso.clone().unwrap_or_else(now_iso_runtime),
        "type": "inversion_persona_lens_gate",
        "persona_id": persona_id,
        "mode": mode,
        "effective_mode": effective_mode,
        "status": status,
        "fail_closed": to_bool_like(payload.and_then(|m| m.get("fail_closed")), false),
        "drift_rate": parse_number_like(payload.and_then(|m| m.get("drift_rate"))).unwrap_or(0.0),
        "drift_threshold": parse_number_like(payload.and_then(|m| m.get("drift_threshold"))).unwrap_or(0.02),
        "parity_confidence": parse_number_like(payload.and_then(|m| m.get("parity_confidence"))).unwrap_or(0.0),
        "parity_confident": to_bool_like(payload.and_then(|m| m.get("parity_confident")), false),
        "reasons": reasons,
        "feed_push": feed_push_value,
        "objective": objective,
        "target": target,
        "impact": impact,
        "allowed": to_bool_like(decision.and_then(|m| m.get("allowed")), false)
    });
    let _ = compute_append_jsonl(&AppendJsonlInput {
        file_path: Some(target_path.clone()),
        row: Some(row),
    });
    let rel_path = {
        let root = clean_text_runtime(input.root.as_deref().unwrap_or(""), 420);
        if !root.is_empty() {
            let root_path = Path::new(&root);
            let target = Path::new(&target_path);
            if let Ok(rel) = target.strip_prefix(root_path) {
                rel.to_string_lossy().to_string()
            } else {
                target_path.clone()
            }
        } else {
            target_path.clone()
        }
    };
    AppendPersonaLensGateReceiptOutput {
        rel_path: Some(rel_path),
    }
}

pub fn compute_append_conclave_correspondence(
    input: &AppendConclaveCorrespondenceInput,
) -> AppendConclaveCorrespondenceOutput {
    let correspondence_path = input.correspondence_path.as_deref().unwrap_or("").trim();
    if correspondence_path.is_empty() {
        return AppendConclaveCorrespondenceOutput { ok: true };
    }
    let _ = compute_ensure_correspondence_file(&EnsureCorrespondenceFileInput {
        file_path: Some(correspondence_path.to_string()),
        header: Some("# Shadow Conclave Correspondence\n\n".to_string()),
    });
    let row = input.row.as_ref().and_then(|v| v.as_object());
    let high_risk_flags = row
        .and_then(|m| m.get("high_risk_flags"))
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .map(|r| clean_text_runtime(&value_to_string(Some(r)), 120))
                .filter(|r| !r.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let review_payload = row
        .and_then(|m| m.get("review_payload"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let entry = [
        format!(
            "## {} - Re: Inversion Shadow Conclave Review ({})",
            clean_text_runtime(&value_to_string(row.and_then(|m| m.get("ts"))), 64),
            {
                let value = clean_text_runtime(
                    &value_to_string(row.and_then(|m| m.get("session_or_step"))),
                    120,
                );
                if value.is_empty() {
                    "unknown".to_string()
                } else {
                    value
                }
            }
        ),
        format!(
            "- Decision: {}",
            if to_bool_like(row.and_then(|m| m.get("pass")), false) {
                "approved"
            } else {
                "escalated_to_monarch"
            }
        ),
        format!("- Winner: {}", {
            let value =
                clean_text_runtime(&value_to_string(row.and_then(|m| m.get("winner"))), 120);
            if value.is_empty() {
                "none".to_string()
            } else {
                value
            }
        }),
        format!("- Arbitration rule: {}", {
            let value = clean_text_runtime(
                &value_to_string(row.and_then(|m| m.get("arbitration_rule"))),
                160,
            );
            if value.is_empty() {
                "unknown".to_string()
            } else {
                value
            }
        }),
        format!(
            "- High-risk flags: {}",
            if high_risk_flags.is_empty() {
                "none".to_string()
            } else {
                high_risk_flags.join(", ")
            }
        ),
        format!("- Query: {}", {
            let value =
                clean_text_runtime(&value_to_string(row.and_then(|m| m.get("query"))), 1800);
            if value.is_empty() {
                "n/a".to_string()
            } else {
                value
            }
        }),
        format!("- Proposal summary: {}", {
            let value = clean_text_runtime(
                &value_to_string(row.and_then(|m| m.get("proposal_summary"))),
                1400,
            );
            if value.is_empty() {
                "n/a".to_string()
            } else {
                value
            }
        }),
        format!("- Receipt: {}", {
            let value = clean_text_runtime(
                &value_to_string(row.and_then(|m| m.get("receipt_path"))),
                260,
            );
            if value.is_empty() {
                "n/a".to_string()
            } else {
                value
            }
        }),
        String::new(),
        "```json".to_string(),
        serde_json::to_string_pretty(&review_payload).unwrap_or_else(|_| "{}".to_string()),
        "```".to_string(),
        String::new(),
    ]
    .join("\n");
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(Path::new(correspondence_path))
    {
        let _ = std::io::Write::write_all(&mut file, format!("{entry}\n").as_bytes());
    }
    AppendConclaveCorrespondenceOutput { ok: true }
}

pub fn compute_persist_decision(input: &PersistDecisionInput) -> PersistDecisionOutput {
    let payload = input.payload.clone().unwrap_or_else(|| json!({}));
    let _ = compute_write_json_atomic(&WriteJsonAtomicInput {
        file_path: input.latest_path.clone(),
        value: Some(payload.clone()),
    });
    let _ = compute_append_jsonl(&AppendJsonlInput {
        file_path: input.history_path.clone(),
        row: Some(payload),
    });
    PersistDecisionOutput { ok: true }
}

pub fn compute_persist_interface_envelope(
    input: &PersistInterfaceEnvelopeInput,
) -> PersistInterfaceEnvelopeOutput {
    let envelope = input.envelope.clone().unwrap_or_else(|| json!({}));
    let _ = compute_write_json_atomic(&WriteJsonAtomicInput {
        file_path: input.latest_path.clone(),
        value: Some(envelope.clone()),
    });
    let _ = compute_append_jsonl(&AppendJsonlInput {
        file_path: input.history_path.clone(),
        row: Some(envelope),
    });
    PersistInterfaceEnvelopeOutput { ok: true }
}
