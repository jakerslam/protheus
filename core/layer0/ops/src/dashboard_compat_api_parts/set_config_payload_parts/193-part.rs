include!("190_route_blocks/late_a.rs");
include!("190_route_blocks/late_b.rs");
include!("190_route_blocks/late_c.rs");

fn handle_global_status_and_method_routes(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    body: &[u8],
    snapshot: &Value,
    request_host: &str,
) -> Option<CompatApiResponse> {
    let usage = usage_from_state(root, snapshot);
    let runtime = runtime_sync_summary(snapshot);
    let alerts_count = parse_non_negative_i64(snapshot.pointer("/health/alerts/count"), 0);
    let status =
        if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) && alerts_count == 0 {
            "healthy"
        } else if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            "degraded"
        } else {
            "critical"
        };

    if let Some(response) = handle_global_receipts_lineage_get(root, method, path, path_only) {
        return Some(response);
    }
    if let Some(response) = handle_global_status_get_routes(
        root,
        method,
        path,
        path_only,
        snapshot,
        request_host,
        &usage,
        &runtime,
        status,
    ) {
        return Some(response);
    }
    if let Some(response) =
        handle_global_post_delete_routes(root, method, path, path_only, body, snapshot)
    {
        return Some(response);
    }
    None
}
