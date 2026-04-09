fn compat_api_response_with_nexus(
    route_label: &str,
    mut response: CompatApiResponse,
) -> CompatApiResponse {
    match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(route_label) {
        Ok(Some(meta)) => {
            if let Some(obj) = response.payload.as_object_mut() {
                obj.insert("nexus_connection".to_string(), meta);
            }
            response
        }
        Ok(None) => response,
        Err(err) => CompatApiResponse {
            status: 403,
            payload: json!({
                "ok": false,
                "error": "nexus_route_denied",
                "route_label": clean_text(route_label, 180),
                "reason": clean_text(&err, 240),
                "fail_closed": true
            }),
        },
    }
}

pub fn handle(
    root: &Path,
    method: &str,
    path: &str,
    body: &[u8],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    handle_with_headers(root, method, path, body, &[], snapshot)
}

#[cfg(test)]
mod tests {
    include!("../config_payload_tests_parts/010-init-git-repo.rs");
    include!("../config_payload_tests_parts/020-agent-create-without-name-returns-non-generic-id.rs");
    include!("../config_payload_tests_parts/030-memory-kv-http-routes-round-trip-and-feed-context.rs");
    include!("../config_payload_tests_parts/040-terminated-agent-endpoints-round-trip.rs");
    include!("../config_payload_tests_parts/050-compact-session-keyframes.rs");
    include!("../config_payload_tests_parts/060-context-telemetry-and-auto-compact.rs");
    include!("../config_payload_tests_parts/070-cron-command-routing.rs");
    include!("../config_payload_tests_parts/080-conversation-search-includes-archived.rs");
    include!("../config_payload_tests_parts/090-latent-tool-discovery-and-rollback.rs");
    include!("../config_payload_tests_parts/100-governance-and-semantic-memory.rs");
    include!("../config_payload_tests_parts/110-agent-capability-gauntlet.rs");
    include!("../config_payload_tests_parts/120-receipts-lineage-route.rs");
}
