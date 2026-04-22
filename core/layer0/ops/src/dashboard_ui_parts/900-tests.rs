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
    include!("900-tests_parts/180-dashboard-agent-task-history-favorites-feedback-and-ui-controller-con.rs");
    include!("900-tests_parts/190-dashboard-agent-task-aliases-and-worktree-extended-controls-contract.rs");
    include!("900-tests_parts/200-dashboard-hooks-governance-routes-contract.rs");
    include!("900-tests_parts/210-dashboard-hooks-test-scenario-routes-contract.rs");
    include!("900-tests_parts/220-dashboard-hooks-test-scenario-extended-routes-contract.rs");
    include!("900-tests_parts/230-dashboard-lock-permission-prompt-routes-contract.rs");
    include!("900-tests_parts/240-dashboard-system-prompt-compose-routes-contract.rs");
    include!("900-tests_parts/250-dashboard-system-prompt-registry-and-spec-routes-contract.rs");
    include!("900-tests_parts/260-dashboard-system-prompt-variant-routes-contract.rs");
    include!("900-tests_parts/270-dashboard-system-prompt-variant-profile-and-builder-routes-contract.rs");
    include!("900-tests_parts/280-dashboard-system-prompt-native-variant-routes-contract.rs");
    include!("900-tests_parts/290-dashboard-system-prompt-variant-validator-and-storage-routes-contract.rs");
    include!("900-tests_parts/300-dashboard-system-prompt-storage-and-task-routes-contract.rs");
    include!("900-tests_parts/310-dashboard-system-prompt-focus-chain-and-task-analysis-routes-contract.rs");
    include!("900-tests_parts/320-dashboard-system-prompt-task-utils-webview-workspace-routes-contract.rs");
    include!("900-tests_parts/330-dashboard-system-prompt-workspace-extension-host-routes-contract.rs");
    include!("900-tests_parts/340-dashboard-system-prompt-host-bridge-vscode-routes-contract.rs");
    include!("900-tests_parts/350-dashboard-system-prompt-vscode-hostbridge-grpc-and-diff-ops-routes-contract.rs");
    include!("900-tests_parts/360-dashboard-system-prompt-vscode-hostbridge-diff-env-tail-routes-contract.rs");
    include!("900-tests_parts/370-dashboard-system-prompt-vscode-hostbridge-env-window-tail-routes-contract.rs");
    include!("900-tests_parts/380-dashboard-system-prompt-vscode-hostbridge-window-workspace-tail-routes-contract.rs");
    include!("900-tests_parts/390-dashboard-system-prompt-vscode-hostbridge-workspace-review-terminal-tail-routes-contract.rs");
    include!("900-tests_parts/400-dashboard-system-prompt-vscode-terminal-checkpoint-tail-routes-contract.rs");
    include!("900-tests_parts/410-dashboard-system-prompt-integrations-checkpoint-claude-diagnostics-tail-routes-contract.rs");
    include!("900-tests_parts/420-dashboard-system-prompt-integrations-editor-misc-tail-routes-contract.rs");
    include!("900-tests_parts/430-dashboard-system-prompt-integrations-runtime-terminal-tail-routes-contract.rs");
    include!("900-tests_parts/440-dashboard-system-prompt-integrations-terminal-package-services-tail-routes-contract.rs");
    include!("900-tests_parts/450-dashboard-system-prompt-services-auth-browser-error-tail-routes-contract.rs");
    include!("900-tests_parts/460-dashboard-system-prompt-services-featureflags-mcp-tail-routes-contract.rs");
    include!("900-tests_parts/470-dashboard-system-prompt-services-mcp-ripgrep-telemetry-tail-routes-contract.rs");
    include!("900-tests_parts/480-dashboard-system-prompt-services-telemetry-providers-temp-tail-routes-contract.rs");
    include!("900-tests_parts/490-dashboard-system-prompt-services-test-tree-sitter-tail-routes-contract.rs");
    include!("900-tests_parts/500-dashboard-system-prompt-services-tree-sitter-queries-uri-tail-routes-contract.rs");
    include!("900-tests_parts/510-dashboard-system-prompt-shared-settings-messages-tail-routes-contract.rs");
    include!("900-tests_parts/520-dashboard-system-prompt-shared-core-api-tail-routes-contract.rs");
    include!("900-tests_parts/530-dashboard-system-prompt-shared-cline-combine-tail-routes-contract.rs");
    include!("900-tests_parts/540-dashboard-system-prompt-shared-constants-messages-tail-routes-contract.rs");
    include!("900-tests_parts/550-dashboard-system-prompt-shared-proto-conversions-tail-routes-contract.rs");
    include!("900-tests_parts/560-dashboard-system-prompt-shared-provider-remote-services-tail-routes-contract.rs");
    include!("900-tests_parts/570-dashboard-system-prompt-shared-services-worker-storage-tail-routes-contract.rs");
    include!("900-tests_parts/580-dashboard-system-prompt-shared-utils-standalone-tail-routes-contract.rs");
    include!("900-tests_parts/590-dashboard-system-prompt-utils-core-tail-routes-contract.rs");
    include!("900-tests_parts/600-dashboard-system-prompt-utils-runtime-tail-routes-contract.rs");
    include!("900-tests_parts/610-dashboard-system-prompt-webview-account-tail-routes-contract.rs");
    include!("900-tests_parts/620-dashboard-system-prompt-webview-auth-storage-tail-routes-contract.rs");
    include!("900-tests_parts/630-dashboard-system-prompt-controller-task-ui-tail-routes-contract.rs");
    include!("900-tests_parts/640-dashboard-system-prompt-controller-ui-event-tail-routes-contract.rs");
    include!("900-tests_parts/650-dashboard-system-prompt-controller-ui-web-worktree-tail-routes-contract.rs");
    include!("900-tests_parts/660-dashboard-system-prompt-controller-worktree-ops-tail-routes-contract.rs");
    include!("900-tests_parts/670-dashboard-system-prompt-hooks-tail-routes-contract.rs");
    include!("900-tests_parts/680-dashboard-system-prompt-hooks-extended-tail-routes-contract.rs");
    include!("900-tests_parts/690-dashboard-system-prompt-hooks-runtime-locks-tail-routes-contract.rs");
    include!("900-tests_parts/700-dashboard-system-prompt-locks-mentions-permissions-prompts-tail-routes-contract.rs");
    include!("900-tests_parts/710-dashboard-system-prompt-components-tail-routes-contract.rs");
    include!("900-tests_parts/720-dashboard-system-prompt-registry-templates-tail-routes-contract.rs");
    include!("900-tests_parts/730-dashboard-system-prompt-variants-tail-routes-contract.rs");
    include!("900-tests_parts/740-dashboard-system-prompt-variants-family-tail-routes-contract.rs");
    include!("900-tests_parts/750-dashboard-system-prompt-variants-native-tail-routes-contract.rs");
    include!("900-tests_parts/760-dashboard-system-prompt-variants-storage-tail-routes-contract.rs");
    include!("900-tests_parts/770-dashboard-system-prompt-storage-task-tail-routes-contract.rs");
    include!("900-tests_parts/780-dashboard-system-prompt-task-focus-chain-tail-routes-contract.rs");
    include!("900-tests_parts/790-dashboard-system-prompt-task-webview-workspace-tail-routes-contract.rs");
    include!("900-tests_parts/800-dashboard-system-prompt-workspace-extension-hosts-tail-routes-contract.rs");
    include!("900-tests_parts/810-dashboard-system-prompt-hosts-surface-tail-routes-contract.rs");
    include!("900-tests_parts/820-dashboard-system-prompt-hostbridge-diff-grpc-tail-routes-contract.rs");
    include!("900-tests_parts/830-dashboard-system-prompt-hostbridge-diff-env-tail-routes-contract.rs");
    include!("900-tests_parts/840-dashboard-system-prompt-hostbridge-env-window-tail-routes-contract.rs");
    include!("900-tests_parts/850-dashboard-system-prompt-hostbridge-window-workspace-tail-routes-contract.rs");
    include!("900-tests_parts/860-dashboard-system-prompt-hostbridge-workspace-review-terminal-tail-routes-contract.rs");
    include!("900-tests_parts/870-dashboard-system-prompt-terminal-checkpoint-tail-routes-contract.rs");
    include!("900-tests_parts/880-dashboard-system-prompt-checkpoint-claude-diagnostics-tail-routes-contract.rs");
    include!("900-tests_parts/890-dashboard-system-prompt-integrations-editor-misc-tail-routes-contract.rs");
    include!("900-tests_parts/900-dashboard-system-prompt-integrations-runtime-terminal-tail-routes-contract.rs");
    include!("900-tests_parts/910-dashboard-system-prompt-integrations-terminal-package-services-tail-routes-contract.rs");
    include!("900-tests_parts/920-dashboard-system-prompt-services-auth-browser-error-tail-routes-contract.rs");
    include!("900-tests_parts/930-dashboard-system-prompt-services-error-featureflags-glob-logging-mcp-tail-routes-contract.rs");
    include!("900-tests_parts/940-dashboard-system-prompt-services-mcp-ripgrep-telemetry-tail-routes-contract.rs");
}
