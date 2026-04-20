
fn read_jsonl_rows(path: &Path, limit: usize) -> Vec<(usize, Value)> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let rows = raw
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            serde_json::from_str::<Value>(line.trim())
                .ok()
                .map(|v| (idx, v))
        })
        .collect::<Vec<_>>();
    if rows.len() <= limit {
        return rows;
    }
    rows[rows.len().saturating_sub(limit)..].to_vec()
}

fn collect_field_strings(value: &Value, field: &str, out: &mut Vec<String>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if out.len() >= cap {
                    break;
                }
                if k == field {
                    let parsed = as_str(Some(v));
                    if !parsed.is_empty() {
                        out.push(parsed);
                    }
                }
                collect_field_strings(v, field, out, cap);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                if out.len() >= cap {
                    break;
                }
                collect_field_strings(row, field, out, cap);
            }
        }
        _ => {}
    }
}

fn row_matches_task_or_trace(row: &Value, task_id: &str, trace_id: Option<&str>) -> bool {
    let mut task_ids = Vec::<String>::new();
    collect_field_strings(row, "task_id", &mut task_ids, 32);
    let task_match = task_ids.iter().any(|v| v == task_id);
    if task_match {
        return true;
    }
    let trace_id = trace_id.unwrap_or("");
    if trace_id.is_empty() {
        return false;
    }
    let mut trace_ids = Vec::<String>::new();
    collect_field_strings(row, "trace_id", &mut trace_ids, 32);
    trace_ids.iter().any(|v| v == trace_id)
}

fn collect_tool_pipeline_objects(value: &Value, out: &mut Vec<Value>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    match value {
        Value::Object(map) => {
            if map.contains_key("normalized_result")
                && (map.contains_key("evidence_cards")
                    || map.contains_key("claim_bundle")
                    || map.contains_key("worker_output"))
            {
                out.push(value.clone());
            }
            for child in map.values() {
                if out.len() >= cap {
                    break;
                }
                collect_tool_pipeline_objects(child, out, cap);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                if out.len() >= cap {
                    break;
                }
                collect_tool_pipeline_objects(row, out, cap);
            }
        }
        _ => {}
    }
}

fn lower_compact_type(row: &Value) -> String {
    let typ = as_str(row.get("type")).to_ascii_lowercase();
    let event = as_str(row.get("event_type")).to_ascii_lowercase();
    let payload_type = row
        .get("payload")
        .and_then(Value::as_object)
        .and_then(|payload| payload.get("type"))
        .map(|value| as_str(Some(value)))
        .unwrap_or_default()
        .to_ascii_lowercase();
    format!("{typ}|{event}|{payload_type}")
}
