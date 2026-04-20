
fn dashboard_message_stack_guard_from_payload(payload: &Value) -> (Value, Vec<String>) {
    let metadata_hover_scope = payload_string(payload, "metadata_hover_scope", "message_only");
    let hover_pushdown_layout_enabled =
        payload_bool(payload, "hover_pushdown_layout_enabled", true);
    let stack_interrupts_on_notifications =
        payload_bool(payload, "stack_interrupts_on_notifications", true);
    let messages = payload_array(payload, "messages");
    let mut previous_key = String::new();
    let mut source_runs = 0u64;
    let mut notification_rows = 0u64;
    for row in messages.iter() {
        let source = payload_string(
            row,
            "source",
            payload_string(row, "role", "unknown").as_str(),
        );
        let kind = payload_string(row, "kind", "message").to_ascii_lowercase();
        let is_notification = kind.contains("notification")
            || kind.contains("notice")
            || kind.contains("name_changed")
            || kind.contains("model_changed")
            || kind.contains("system_event");
        if is_notification {
            notification_rows = notification_rows.saturating_add(1);
        }
        let key = if is_notification {
            format!("notification::{source}")
        } else {
            format!("source::{source}")
        };
        if key != previous_key {
            source_runs = source_runs.saturating_add(1);
            previous_key = key;
        }
    }
    let expected_min_source_runs = payload_u64(
        payload,
        "expected_min_source_runs",
        if messages.is_empty() {
            0
        } else if notification_rows > 0 {
            2
        } else {
            1
        },
    );
    let mut violations = Vec::<String>::new();
    if metadata_hover_scope != "message_only" {
        violations.push(format!(
            "specific_dashboard_metadata_hover_scope_mismatch:{metadata_hover_scope}"
        ));
    }
    if !hover_pushdown_layout_enabled {
        violations.push("specific_dashboard_metadata_pushdown_disabled".to_string());
    }
    if notification_rows > 0 && !stack_interrupts_on_notifications {
        violations.push("specific_dashboard_notifications_must_interrupt_stack".to_string());
    }
    if source_runs < expected_min_source_runs {
        violations.push(format!(
            "specific_dashboard_source_run_count_too_low:{source_runs}<{}",
            expected_min_source_runs
        ));
    }

    (
        json!({
            "authority": "rust_runtime_systems",
            "policy": "V6-DASHBOARD-009.1",
            "metadata_hover_scope": metadata_hover_scope,
            "hover_pushdown_layout_enabled": hover_pushdown_layout_enabled,
            "stack_interrupts_on_notifications": stack_interrupts_on_notifications,
            "source_run_count": source_runs,
            "expected_min_source_runs": expected_min_source_runs,
            "notification_rows": notification_rows,
            "messages_seen": messages.len()
        }),
        violations,
    )
}

fn dashboard_boot_retry_guard_from_payload(payload: &Value) -> (Value, Vec<String>) {
    let boot_retry_enabled = payload_bool(payload, "boot_retry_enabled", true);
    let boot_retry_max_attempts = payload_u64(payload, "boot_retry_max_attempts", 5).clamp(1, 20);
    let boot_retry_backoff_ms =
        payload_u64(payload, "boot_retry_backoff_ms", 1000).clamp(1, 60_000);
    let startup_failed = payload_bool(payload, "startup_failed", false);
    let server_status_emitted = payload_bool(payload, "server_status_emitted", !startup_failed);
    let server_status_path = payload_string(
        payload,
        "server_status_path",
        "local/state/ops/daemon_control/server_status.json",
    );
    let status_error_code = payload_string(payload, "status_error_code", "");
    let mut violations = Vec::<String>::new();
    if !boot_retry_enabled {
        violations.push("specific_dashboard_boot_retry_disabled".to_string());
    }
    if boot_retry_max_attempts < 2 {
        violations.push(format!(
            "specific_dashboard_boot_retry_attempts_too_low:{boot_retry_max_attempts}"
        ));
    }
    if boot_retry_backoff_ms < 1000 {
        violations.push(format!(
            "specific_dashboard_boot_retry_backoff_too_low:{boot_retry_backoff_ms}"
        ));
    }
    if startup_failed && !server_status_emitted {
        violations.push("specific_dashboard_server_status_missing_on_failure".to_string());
    }
    if startup_failed && status_error_code.trim().is_empty() {
        violations.push("specific_dashboard_failure_missing_error_code".to_string());
    }
    if startup_failed && server_status_path.trim().is_empty() {
        violations.push("specific_dashboard_failure_missing_status_path".to_string());
    }

    (
        json!({
            "authority": "rust_runtime_systems",
            "policy": "V6-DASHBOARD-009.2",
            "boot_retry_enabled": boot_retry_enabled,
            "boot_retry_max_attempts": boot_retry_max_attempts,
            "boot_retry_backoff_ms": boot_retry_backoff_ms,
            "startup_failed": startup_failed,
            "server_status_emitted": server_status_emitted,
            "server_status_path": server_status_path,
            "status_error_code": status_error_code
        }),
        violations,
    )
}
