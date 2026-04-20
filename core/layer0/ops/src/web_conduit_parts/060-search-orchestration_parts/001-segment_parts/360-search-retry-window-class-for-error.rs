fn search_retry_window_class_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    let wait_seconds = search_retry_next_action_after_seconds_for_error(error, retry_after_seconds);
    if wait_seconds <= 0 {
        "immediate"
    } else if wait_seconds <= 60 {
        "short"
    } else if wait_seconds <= 900 {
        "medium"
    } else {
        "long"
    }
}
