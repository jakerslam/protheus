fn command_list_from_payload(payload: &Map<String, Value>) -> Vec<String> {
    let mut out = Vec::<String>::new();
    if let Some(commands) = payload.get("commands").and_then(Value::as_array) {
        for row in commands {
            let command = clean_text(row.as_str().unwrap_or(""), 2000);
            if !command.is_empty() {
                out.push(command);
            }
        }
    }
    out
}

fn build_adoption_for_commands(session_id: &str, commands: &[String], output_tokens: usize) -> Value {
    let mut total = 0usize;
    let mut prefixed = 0usize;
    let mut supported = 0usize;
    let mut unsupported = 0usize;
    let mut ignored = 0usize;

    for raw in commands {
        for segment in split_command_chain_for_kernel(raw) {
            let trimmed = clean_text(&segment, 600);
            if trimmed.is_empty() {
                continue;
            }
            total += 1;
            if trimmed.starts_with("rtk ") {
                prefixed += 1;
                supported += 1;
                continue;
            }
            let detail = classify_command_detail_for_kernel(&trimmed);
            if detail
                .get("ignored")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                ignored += 1;
                continue;
            }
            if detail
                .get("supported")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                supported += 1;
            } else {
                unsupported += 1;
            }
        }
    }

    let adoption_pct = if total == 0 {
        0.0
    } else {
        (supported as f64 / total as f64) * 100.0
    };

    json!({
      "session_id": clean_text(session_id, 120),
      "total_commands": total,
      "supported_commands": supported,
      "prefixed_commands": prefixed,
      "unsupported_commands": unsupported,
      "ignored_commands": ignored,
      "adoption_pct": adoption_pct,
      "output_tokens": output_tokens
    })
}

fn build_adoption_report(payload: &Map<String, Value>, limit: usize) -> Value {
    let mut rows = Vec::<Value>::new();
    if let Some(sessions) = payload.get("sessions").and_then(Value::as_array) {
        for row in sessions {
            let Some(obj) = row.as_object() else {
                continue;
            };
            let session_id = clean_text(
                obj.get("session_id")
                    .and_then(Value::as_str)
                    .or_else(|| obj.get("id").and_then(Value::as_str))
                    .unwrap_or("session"),
                120,
            );
            if let Some(jsonl) = obj.get("jsonl").and_then(Value::as_str) {
                let extracted = extract_commands_from_jsonl(&session_id, jsonl);
                let commands = extracted
                    .iter()
                    .map(|entry| entry.command.clone())
                    .collect::<Vec<_>>();
                let output_tokens = extracted
                    .iter()
                    .filter_map(|entry| entry.output_len)
                    .sum::<usize>()
                    / 4;
                rows.push(build_adoption_for_commands(
                    &session_id,
                    &commands,
                    output_tokens,
                ));
            } else {
                let commands = command_list_from_payload(obj);
                let output_tokens = obj.get("output_tokens").and_then(Value::as_u64).unwrap_or(0)
                    as usize;
                rows.push(build_adoption_for_commands(
                    &session_id,
                    &commands,
                    output_tokens,
                ));
            }
        }
    } else {
        let session_id = clean_text(
            payload
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or("session"),
            120,
        );
        if let Some(jsonl) = payload.get("jsonl").and_then(Value::as_str) {
            let extracted = extract_commands_from_jsonl(&session_id, jsonl);
            let commands = extracted
                .iter()
                .map(|entry| entry.command.clone())
                .collect::<Vec<_>>();
            let output_tokens = extracted
                .iter()
                .filter_map(|entry| entry.output_len)
                .sum::<usize>()
                / 4;
            rows.push(build_adoption_for_commands(
                &session_id,
                &commands,
                output_tokens,
            ));
        } else {
            let commands = command_list_from_payload(payload);
            let output_tokens = payload
                .get("output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            rows.push(build_adoption_for_commands(
                &session_id,
                &commands,
                output_tokens,
            ));
        }
    }

    rows.sort_by(|a, b| {
        let ac = a.get("total_commands").and_then(Value::as_u64).unwrap_or(0);
        let bc = b.get("total_commands").and_then(Value::as_u64).unwrap_or(0);
        bc.cmp(&ac)
    });
    rows.truncate(limit.max(1));

    let totals = rows
        .iter()
        .fold((0usize, 0usize, 0usize, 0usize), |acc, row| {
            (
                acc.0 + row.get("total_commands").and_then(Value::as_u64).unwrap_or(0) as usize,
                acc.1
                    + row
                        .get("supported_commands")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize,
                acc.2
                    + row
                        .get("unsupported_commands")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize,
                acc.3 + row.get("output_tokens").and_then(Value::as_u64).unwrap_or(0) as usize,
            )
        });

    let adoption_pct = if totals.0 == 0 {
        0.0
    } else {
        (totals.1 as f64 / totals.0 as f64) * 100.0
    };

    json!({
      "ok": true,
      "type": "session_command_adoption_report",
      "sessions_scanned": rows.len(),
      "total_commands": totals.0,
      "supported_commands": totals.1,
      "unsupported_commands": totals.2,
      "total_output_tokens": totals.3,
      "adoption_pct": adoption_pct,
      "sessions": rows
    })
}

fn recommendation_suggestions_from_report(payload: &Map<String, Value>, limit: usize) -> Vec<String> {
    let report = build_adoption_report(payload, limit.max(1));
    let total = report
        .get("total_commands")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if total == 0 {
        return Vec::new();
    }
    let unsupported = report
        .get("unsupported_commands")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let adoption_pct = report.get("adoption_pct").and_then(Value::as_f64).unwrap_or(0.0);
    let output_tokens = report
        .get("total_output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let commands = command_list_from_payload(payload);
    let classify = classify_command_list_for_kernel(&commands, 8);
    let unsupported_base = classify
        .get("unsupported")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("base_command"))
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 80))
        .unwrap_or_default();
    let supported_canonical = classify
        .get("supported")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("canonical"))
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 80))
        .unwrap_or_default();

    let mut out = Vec::<String>::new();
    if unsupported > 0 {
        if !unsupported_base.is_empty() {
            out.push(format!(
                "Implement a supported Rust route for `{}`.",
                unsupported_base
            ));
        } else {
            out.push("Convert unsupported commands into supported Rust routes.".to_string());
        }
    }
    if adoption_pct < 80.0 {
        out.push("Improve command-to-route mapping for higher supported tool hit rate.".to_string());
    }
    if output_tokens > 1200 {
        out.push("Generate a concise digest of terminal output and next actions.".to_string());
    }
    if !supported_canonical.is_empty() {
        out.push(format!("Run `{}` as the next safe step.", supported_canonical));
    }
    if out.is_empty() {
        out.push("Run one focused command and summarize results.".to_string());
    }

    let mut dedup = Vec::<String>::new();
    for row in out {
        let cleaned = normalize_follow_up_suggestion(&row);
        if cleaned.is_empty() {
            continue;
        }
        if dedup
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&cleaned))
        {
            continue;
        }
        dedup.push(cleaned);
        if dedup.len() >= limit.max(1) {
            break;
        }
    }
    dedup
}

pub(crate) fn adoption_report_for_kernel(payload: &Value, limit: usize) -> Value {
    build_adoption_report(payload_obj(payload), limit)
}

pub(crate) fn follow_up_suggestions_for_kernel(payload: &Value, limit: usize) -> Vec<String> {
    recommendation_suggestions_from_report(payload_obj(payload), limit)
}
