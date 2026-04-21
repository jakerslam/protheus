
fn classify_command_list(input: &[String], limit: usize) -> Value {
    let mut supported =
        BTreeMap::<String, (usize, &'static str, &'static str, f64, SupportStatus, usize)>::new();
    let mut unsupported = HashMap::<String, (usize, String)>::new();
    let mut ignored = 0usize;
    let mut total = 0usize;
    let mut seen = HashSet::<String>::new();

    for raw in input {
        for segment in split_command_chain(raw) {
            total += 1;
            match classify_command(&segment) {
                Classification::Supported {
                    command_key,
                    canonical,
                    category,
                    savings_pct,
                    status,
                } => {
                    let est_tokens = ((category_avg_tokens(category, &command_key) as f64)
                        * (savings_pct / 100.0))
                        .round() as usize;
                    let entry = supported
                        .entry(format!(
                            "{category}|{canonical}|{command_key}|{}",
                            status.as_str()
                        ))
                        .or_insert((0, canonical, category, savings_pct, status, 0));
                    entry.0 += 1;
                    entry.5 += est_tokens;
                }
                Classification::Unsupported { base_command } => {
                    let row = unsupported
                        .entry(base_command.clone())
                        .or_insert((0, clean_text(&segment, 220)));
                    row.0 += 1;
                }
                Classification::Ignored => {
                    ignored += 1;
                }
            }
        }
    }

    fn sort_and_limit(rows: &mut Vec<Value>, limit: usize) {
        rows.sort_by(|a, b| {
            let ac = a.get("count").and_then(Value::as_u64).unwrap_or(0);
            let bc = b.get("count").and_then(Value::as_u64).unwrap_or(0);
            bc.cmp(&ac)
        });
        rows.truncate(limit.max(1));
    }

    fn sum_row_key(rows: &[Value], key: &str) -> usize {
        rows.iter()
            .map(|row| row.get(key).and_then(Value::as_u64).unwrap_or(0) as usize)
            .sum::<usize>()
    }

    let mut supported_rows = supported
        .into_iter()
        .map(|(key, row)| {
            let command_key = key.split('|').nth(2).unwrap_or("").to_string();
            json!({
                "command": command_key,
                "count": row.0,
                "canonical": row.1,
                "category": row.2,
                "estimated_savings_tokens": row.5,
                "estimated_savings_pct": row.3,
                "status": row.4.as_str(),
            })
        })
        .collect::<Vec<_>>();
    sort_and_limit(&mut supported_rows, limit);

    let mut unsupported_rows = unsupported
        .into_iter()
        .map(|(base_command, row)| {
            json!({
                "base_command": base_command,
                "count": row.0,
                "example": row.1,
            })
        })
        .collect::<Vec<_>>();
    sort_and_limit(&mut unsupported_rows, limit);

    let supported_count = sum_row_key(&supported_rows, "count");
    let unsupported_count = sum_row_key(&unsupported_rows, "count");
    let total_estimated_savings_tokens = sum_row_key(&supported_rows, "estimated_savings_tokens");

    // Track unique commands from incoming payload for quick operator visibility.
    for row in input {
        let normalized = clean_text(row, 180);
        if !normalized.is_empty() {
            seen.insert(normalized);
        }
    }

    json!({
        "ok": true,
        "type": "session_command_discovery_report",
        "total_commands": total,
        "supported_count": supported_count,
        "unsupported_count": unsupported_count,
        "ignored_count": ignored,
        "total_estimated_savings_tokens": total_estimated_savings_tokens,
        "unique_input_commands": seen.len(),
        "supported": supported_rows,
        "unsupported": unsupported_rows,
    })
}

pub(crate) fn split_command_chain_for_kernel(raw: &str) -> Vec<String> {
    split_command_chain(raw)
}

pub(crate) fn classify_command_detail_for_kernel(raw: &str) -> Value {
    match classify_command(raw) {
        Classification::Supported {
            command_key,
            canonical,
            category,
            savings_pct,
            status,
        } => json!({
            "supported": true,
            "ignored": false,
            "command_key": command_key,
            "canonical": canonical,
            "category": category,
            "estimated_savings_pct": savings_pct,
            "status": status.as_str(),
        }),
        Classification::Unsupported { base_command } => json!({
            "supported": false,
            "ignored": false,
            "base_command": base_command,
        }),
        Classification::Ignored => json!({
            "supported": false,
            "ignored": true,
        }),
    }
}

pub(crate) fn classify_command_list_for_kernel(input: &[String], limit: usize) -> Value {
    classify_command_list(input, limit)
}
include!("020-run-and-tests.rs");
