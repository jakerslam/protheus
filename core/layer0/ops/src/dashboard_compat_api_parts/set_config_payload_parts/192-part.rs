include!("190_route_blocks/agent_scope_full.rs");

fn handle_agent_scope_routes(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    body: &[u8],
    headers: &[(&str, &str)],
    snapshot: &Value,
    requester_agent: &str,
) -> Option<CompatApiResponse> {
    handle_agent_scope_full(
        root,
        method,
        path,
        path_only,
        body,
        headers,
        snapshot,
        requester_agent,
    )
}
