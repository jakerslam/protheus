    let limit = dashboard_payload_usize(payload, "limit", 20, 1, 200);
    let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
    let window_seconds = dashboard_summary_window_seconds(payload);
    let explicit_since_epoch = dashboard_payload_i64_with_bounds(
        payload,
        "since_epoch_s",
        0,
        0,
        i64::MAX,
    );
    let effective_since_epoch = if explicit_since_epoch > 0 {
        explicit_since_epoch
    } else if window_seconds > 0 {
        now_epoch.saturating_sub(window_seconds)
    } else {
        0
    };
    let window_filter_applied = effective_since_epoch > 0;
    let recent_entries = dashboard_troubleshooting_read_recent_entries(root);
    let classification_filter = dashboard_payload_first_string_filter(
        payload,
        &["classification_filter", "class_filter", "classifications"],
        12,
        80,
    );
    let error_filter =
        dashboard_payload_first_string_filter(payload, &["error_filter", "errors", "error_codes"], 12, 120);
    let filters_applied = !classification_filter.is_empty() || !error_filter.is_empty();
    let filtered_entries = recent_entries
        .iter()
        .filter(|row| {
            let error_code = clean_text(
                row.pointer("/workflow/error_code")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            )
            .to_ascii_lowercase();
            let class = clean_text(
                row.pointer("/workflow/classification")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                80,
            )
            .to_ascii_lowercase();
            let class_match = classification_filter.is_empty()
                || classification_filter
                    .iter()
                    .any(|value| dashboard_troubleshooting_filter_match(&class, value));
            let error_match = error_filter.is_empty()
                || error_filter
                    .iter()
                    .any(|value| dashboard_troubleshooting_filter_match(&error_code, value));
            let window_match = if window_filter_applied {
                let epoch = dashboard_troubleshooting_epoch_hint(row);
                epoch > 0 && epoch >= effective_since_epoch
            } else {
                true
            };
            class_match && error_match && window_match
        })
        .cloned()
        .collect::<Vec<_>>();
    let entry_count = filtered_entries.len();
    let filtered_out_count = recent_entries.len().saturating_sub(filtered_entries.len());
    let failure_count = filtered_entries
        .iter()
        .filter(|row| dashboard_troubleshooting_exchange_failed(row))
        .count();
    let stale_count = filtered_entries
        .iter()
        .filter(|row| row.get("stale").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let failure_rate = if entry_count == 0 {
        0.0
    } else {
        ((failure_count as f64) / (entry_count as f64) * 10_000.0).round() / 10_000.0
    };
    let stale_rate = if entry_count == 0 {
        0.0
    } else {
        ((stale_count as f64) / (entry_count as f64) * 10_000.0).round() / 10_000.0
    };
    let mut error_counts = HashMap::<String, i64>::new();
    let mut class_counts = HashMap::<String, i64>::new();
    for row in &filtered_entries {
        let error_code = clean_text(
            row.pointer("/workflow/error_code")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if !error_code.is_empty() {
            *error_counts.entry(error_code).or_insert(0) += 1;
        }
        let class = clean_text(
            row.pointer("/workflow/classification")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            80,
        )
        .to_ascii_lowercase();
        *class_counts.entry(class).or_insert(0) += 1;
    }
    let error_hist = dashboard_troubleshooting_sorted_histogram(error_counts, "error");
    let class_hist = dashboard_troubleshooting_sorted_histogram(class_counts, "classification");
    let top_error = error_hist
        .first()
        .and_then(|row| row.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let top_class = class_hist
        .first()
        .and_then(|row| row.get("classification"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let recommendations = dashboard_troubleshooting_eval_recommendations(top_error, top_class);
