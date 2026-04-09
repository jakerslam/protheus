include!("190_route_blocks/primary_a.rs");
include!("190_route_blocks/primary_b.rs");
include!("190_route_blocks/primary_c.rs");

fn handle_primary_dashboard_routes(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    body: &[u8],
    headers: &[(&str, &str)],
    snapshot: &Value,
    requester_agent: &str,
) -> Option<CompatApiResponse> {
    if let Some(response) = handle_primary_dashboard_routes_a(
        root,
        method,
        path,
        path_only,
        body,
        headers,
        snapshot,
        requester_agent,
    ) {
        return Some(response);
    }
    if let Some(response) = handle_primary_dashboard_routes_b(
        root,
        method,
        path,
        path_only,
        body,
        snapshot,
        requester_agent,
    ) {
        return Some(response);
    }
    if let Some(response) = handle_primary_dashboard_routes_c(
        root,
        method,
        path_only,
        body,
        snapshot,
        requester_agent,
    ) {
        return Some(response);
    }
    None
}
