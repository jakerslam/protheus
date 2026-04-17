include!("dashboard_compat_api_sidebar_ops_parts/010-state-helpers.rs");
include!("dashboard_compat_api_sidebar_ops_parts/020-cron-and-trigger-store.rs");
include!("dashboard_compat_api_sidebar_ops_parts/030-sidebar-route-handler.rs");
include!("dashboard_compat_api_sidebar_ops_parts/900-placeholder.rs");

pub(crate) const SIDEBAR_OPS_CONTRACT_VERSION: &str = "sidebar_ops_v2";

pub(crate) fn web_tooling_status_payload(root: &Path) -> Value {
    let profile = load_web_tooling_profile(root);
    let auth = web_tooling_auth_presence(&profile);
    let diagnostics = summarize_web_tooling_history(root, 80);
    let total_calls = as_i64(diagnostics.get("total_calls"), 0).max(0);
    let failed_calls = as_i64(diagnostics.get("failed_calls"), 0).max(0);
    let status = if !as_bool(auth.get("any_present"), false) {
        "auth_missing"
    } else if total_calls <= 0 {
        "idle"
    } else if failed_calls >= (total_calls + 1) / 2 {
        "degraded"
    } else {
        "ok"
    };
    json!({
        "ok": true,
        "type": "web_tooling_status",
        "contract_version": SIDEBAR_OPS_CONTRACT_VERSION,
        "status": status,
        "preferred_provider": preferred_web_tooling_provider(&profile),
        "auth": auth,
        "profile": profile,
        "diagnostics": diagnostics
    })
}

pub(crate) fn web_tooling_errors_payload(root: &Path) -> Value {
    let diagnostics = summarize_web_tooling_history(root, 120);
    json!({
        "ok": true,
        "type": "web_tooling_errors",
        "contract_version": SIDEBAR_OPS_CONTRACT_VERSION,
        "errors": diagnostics.get("recent_errors").cloned().unwrap_or_else(|| json!([])),
        "error_codes": diagnostics.get("error_codes").cloned().unwrap_or_else(|| json!({})),
        "last_error_code": diagnostics.get("last_error_code").cloned().unwrap_or(Value::Null),
        "history_path": diagnostics.get("history_path").cloned().unwrap_or(Value::Null)
    })
}

pub(crate) fn web_tooling_preferences_upsert_payload(root: &Path, request: &Value) -> Value {
    let existing = load_web_tooling_profile(root);
    let merged = merge_web_tooling_profile(&existing, request);
    save_web_tooling_profile(root, &merged);
    json!({
        "ok": true,
        "type": "web_tooling_preferences_upsert",
        "contract_version": SIDEBAR_OPS_CONTRACT_VERSION,
        "profile": merged
    })
}

pub(crate) fn web_tooling_probe_payload(root: &Path, request: &Value) -> Value {
    let query = clean_text(
        request
            .get("query")
            .or_else(|| request.get("q"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        1400,
    );
    if query.is_empty() {
        return json!({
            "ok": false,
            "type": "web_tooling_probe",
            "error": "query_required"
        });
    }
    let profile = load_web_tooling_profile(root);
    let sanitized = sanitize_web_tooling_query(&query);
    if sanitized.is_empty() {
        return json!({
            "ok": false,
            "type": "web_tooling_probe",
            "error": "query_invalid_after_sanitize"
        });
    }
    let canonical = canonicalize_web_tooling_query(&sanitized, &profile);
    json!({
        "ok": true,
        "type": "web_tooling_probe",
        "contract_version": SIDEBAR_OPS_CONTRACT_VERSION,
        "provider_hint": preferred_web_tooling_provider(&profile),
        "query": {
            "input": query,
            "sanitized": sanitized,
            "canonical": canonical
        },
        "query_policy": profile.get("query_policy").cloned().unwrap_or_else(|| json!({}))
    })
}
