// Layer ownership: core/layer0/ops (dashboard_ui_parts tests)
#[cfg(test)]
fn dashboard_assert_bool_pointer(
    payload: &serde_json::Value,
    pointer: &str,
    expected: bool,
) {
    assert_eq!(
        payload.pointer(pointer).and_then(serde_json::Value::as_bool),
        Some(expected),
        "expected {pointer} to be {expected}"
    );
}

#[cfg(test)]
fn dashboard_assert_bool_keys_with_prefix(
    payload: &serde_json::Value,
    prefix: &str,
    keys: &[&str],
    expected: bool,
) {
    for key in keys {
        dashboard_assert_bool_pointer(payload, &format!("{prefix}/{key}"), expected);
    }
}

#[cfg(test)]
fn dashboard_write_troubleshooting_recent_entries(
    root: &std::path::Path,
    entries: Vec<serde_json::Value>,
) {
    let recent_path = root.join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
    if let Some(parent) = recent_path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir troubleshooting");
    }
    std::fs::write(
        &recent_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "type": "dashboard_troubleshooting_recent_workflows",
            "entries": entries
        }))
        .expect("json"),
    )
    .expect("write");
}

#[cfg(test)]
mod tests {
    include!("900-tests_parts/010-dashboard-assert-bool-pointer.rs");
    include!("900-tests_parts/020-memory-artifacts-cache-stabilizes-repeated-snapshot-reads.rs");
    include!("900-tests_parts/030-dashboard-troubleshooting-report-message-dedupes-identical-outbox-reques.rs");
    include!("900-tests_parts/040-dashboard-troubleshooting-outbox-state-lane-reports-depth-and-histogram.rs");
    include!("900-tests_parts/050-dashboard-troubleshooting-summary-lane-reports-recommendations-and-queue.rs");
    include!("900-tests_parts/060-dashboard-troubleshooting-pressure-contract-object-aliases-route-with-pa.rs");
    include!("900-tests_parts/070-dashboard-troubleshooting-deadletter-state-and-requeue-flow.rs");
    include!("900-tests_parts/080-dashboard-troubleshooting-synthetic-failure-sample-bundle-shape.rs");
    include!("900-tests_parts/090-dashboard-troubleshooting-summary-filtered-alias-matches-summary-lane.rs");
    include!("900-tests_parts/100-dashboard-troubleshooting-outbox-queue-alias-routes-to-state-lane.rs");
    include!("900-tests_parts/110-dashboard-troubleshooting-outbox-pressure-reason-alias-routes-to-state-l.rs");
    include!("900-tests_parts/120-dashboard-troubleshooting-outbox-pressure-next-action-after-seconds-alia.rs");
    include!("900-tests_parts/130-dashboard-troubleshooting-summary-pressure-decision-lane-token-alias-rou.rs");
    include!("900-tests_parts/140-dashboard-troubleshooting-summary-pressure-contract-runbook-alias-routes.rs");
    include!("900-tests_parts/150-dashboard-troubleshooting-summary-pressure-contract-next-action-after-se.rs");
    include!("900-tests_parts/160-dashboard-troubleshooting-summary-pressure-contract-decision-lane-token-.rs");
    include!("900-tests_parts/170-dashboard-troubleshooting-summary-window-filter-excludes-old-entries.rs");
}
