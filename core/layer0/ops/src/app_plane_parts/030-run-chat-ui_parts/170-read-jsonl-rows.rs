fn chat_ui_read_jsonl_rows(path: &Path, max_rows: usize) -> Vec<Value> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    if raw.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::<Value>::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            rows.push(value);
        }
    }
    if rows.len() > max_rows {
        rows.split_off(rows.len().saturating_sub(max_rows))
    } else {
        rows
    }
}
