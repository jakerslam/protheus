fn search_parse_nonnegative_i64(value: Option<&Value>) -> i64 {
    let Some(value) = value else {
        return 0;
    };
    if let Some(raw) = value.as_i64() {
        return raw.max(0);
    }
    if let Some(raw) = value.as_u64() {
        return raw.min(i64::MAX as u64) as i64;
    }
    if let Some(raw) = value.as_f64() {
        if raw.is_finite() {
            return raw.floor().max(0.0).min(i64::MAX as f64) as i64;
        }
    }
    if let Some(raw) = value.as_str() {
        let trimmed = clean_text(raw, 32);
        if !trimmed.is_empty() {
            if let Ok(parsed) = trimmed.parse::<i64>() {
                return parsed.max(0);
            }
        }
    }
    0
}
