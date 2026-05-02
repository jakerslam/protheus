include!("190_route_blocks/shell_socket.rs");

pub fn handle_with_headers(
    root: &Path,
    method: &str,
    path: &str,
    body: &[u8],
    headers: &[(&str, &str)],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    let path_only = path.split('?').next().unwrap_or(path);
    let requester_agent = requester_agent_id(headers);
    let request_host = header_value(headers, "host").unwrap_or_default();

    if let Some(response) = handle_shell_socket_routes(
        root,
        method,
        path,
        path_only,
        body,
        headers,
        snapshot,
        &requester_agent,
        &request_host,
    ) {
        return Some(response);
    }

    if let Some(response) = handle_primary_dashboard_routes(
        root,
        method,
        path,
        path_only,
        body,
        headers,
        snapshot,
        &requester_agent,
    ) {
        return Some(response);
    }

    if let Some(response) = handle_agent_scope_routes(
        root,
        method,
        path,
        path_only,
        body,
        headers,
        snapshot,
        &requester_agent,
    ) {
        return Some(response);
    }

    if let Some(response) = handle_global_status_and_method_routes(
        root,
        method,
        path,
        path_only,
        body,
        snapshot,
        &request_host,
    ) {
        return Some(response);
    }

    None
}
