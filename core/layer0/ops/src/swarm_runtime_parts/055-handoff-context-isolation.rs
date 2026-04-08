const HANDOFF_CONTEXT_MAX_KEYS: usize = 16;
const HANDOFF_CONTEXT_MAX_BYTES: usize = 2_048;
const HANDOFF_CONTEXT_SUMMARY_KEYS: usize = 6;

fn summarize_handoff_value(value: &Value) -> String {
    match value {
        Value::String(text) => clean_text(text, 64),
        Value::Number(number) => clean_text(&number.to_string(), 32),
        Value::Bool(flag) => {
            if *flag {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Array(rows) => format!("array({})", rows.len()),
        Value::Object(map) => format!("object({})", map.len()),
        Value::Null => "null".to_string(),
    }
}

fn isolate_handoff_context(context: Value, reason: &str, importance: f64) -> (Value, Value) {
    let normalized = Value::Object(normalize_context_map(context));
    let requested_bytes = json_size_bytes(&normalized);
    let compacted = requested_bytes > HANDOFF_CONTEXT_MAX_BYTES;
    let compacted_value = if compacted {
        compact_context_value(&normalized)
    } else {
        normalized
    };
    let mut rows = normalize_context_map(compacted_value);
    let mut dropped_keys = Vec::<String>::new();
    if rows.len() > HANDOFF_CONTEXT_MAX_KEYS {
        let mut limited = Map::<String, Value>::new();
        for (idx, (key, value)) in rows.into_iter().enumerate() {
            if idx < HANDOFF_CONTEXT_MAX_KEYS {
                limited.insert(key, value);
            } else {
                dropped_keys.push(key);
            }
        }
        rows = limited;
    }
    let summary = rows
        .iter()
        .take(HANDOFF_CONTEXT_SUMMARY_KEYS)
        .map(|(key, value)| format!("{key}={}", summarize_handoff_value(value)))
        .collect::<Vec<_>>()
        .join("; ");
    let isolated = Value::Object(rows.into_iter().collect::<Map<String, Value>>());
    let effective_bytes = json_size_bytes(&isolated);
    let receipt = json!({
        "reason": clean_text(reason, 180),
        "importance": importance.clamp(0.0, 1.0),
        "requested_bytes": requested_bytes,
        "effective_bytes": effective_bytes,
        "compacted": compacted,
        "max_keys": HANDOFF_CONTEXT_MAX_KEYS,
        "effective_key_count": isolated.as_object().map(|map| map.len()).unwrap_or(0),
        "dropped_keys": dropped_keys,
        "context_hash": deterministic_receipt_hash(&isolated),
        "summary": clean_text(&summary, 260),
    });
    (isolated, receipt)
}

#[cfg(test)]
mod handoff_context_isolation_tests {
    use super::*;

    #[test]
    fn handoff_context_isolation_compacts_and_limits_keys() {
        let mut payload = Map::<String, Value>::new();
        for idx in 0..40usize {
            payload.insert(
                format!("key_{idx:02}"),
                json!(format!("value-{idx}-{}", "x".repeat(80))),
            );
        }
        let (isolated, receipt) =
            isolate_handoff_context(Value::Object(payload), "stress-test", 0.9);
        assert!(
            isolated
                .as_object()
                .map(|rows| rows.len() <= HANDOFF_CONTEXT_MAX_KEYS)
                .unwrap_or(false),
            "isolation must cap key count"
        );
        assert_eq!(
            receipt.get("compacted").and_then(Value::as_bool),
            Some(true)
        );
        let dropped = receipt
            .get("dropped_keys")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0);
        let effective_keys = receipt
            .get("effective_key_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert!(
            dropped > 0 || effective_keys <= HANDOFF_CONTEXT_MAX_KEYS as u64,
            "oversized context should either drop keys or remain bounded after compaction"
        );
        assert!(
            receipt
                .get("context_hash")
                .and_then(Value::as_str)
                .map(|row| !row.is_empty())
                .unwrap_or(false),
            "isolation receipt requires deterministic context hash"
        );
    }
}
