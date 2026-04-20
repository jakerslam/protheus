fn fetch_retry_after_seconds_from_value(value: Option<&Value>) -> i64 {
    let raw = fetch_parse_nonnegative_i64(value);
    if raw <= 0 {
        return 0;
    }
    let now_epoch_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0);
    let normalized = if now_epoch_seconds > 0 && raw > now_epoch_seconds {
        raw.saturating_sub(now_epoch_seconds)
    } else {
        raw
    };
    normalized.clamp(0, FETCH_RETRY_AFTER_SECONDS_MAX)
}
