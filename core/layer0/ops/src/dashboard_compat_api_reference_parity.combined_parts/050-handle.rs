
pub fn handle(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    if method == "GET" && path_only == "/v1/models" {
        return Some(CompatApiResponse {
            status: 200,
            payload: models_v1_payload(root, snapshot),
        });
    }

    if method == "POST" && path_only == "/api/auth/login" {
        return Some(login_payload(root, body, headers));
    }
    if method == "POST" && path_only == "/api/auth/logout" {
        return Some(logout_payload(root));
    }

    if method == "GET"
        && (path_only == "/api/integrations" || path_only == "/api/integrations/catalog")
    {
        return Some(CompatApiResponse {
            status: 200,
            payload: integrations_payload(root),
        });
    }
    if method == "GET" {
        if let Some(id) = path_only.strip_prefix("/api/integrations/") {
            if !id.contains('/') {
                return Some(integration_detail_payload(root, id));
            }
        }
    }
    if let Some(channel_path) = rewrite_integration_to_channel(path_only) {
        return super::dashboard_compat_api_channels::handle(root, method, &channel_path, body);
    }

    if method == "POST" && path_only == "/api/pairing/start" {
        return Some(pairing_start_payload(root));
    }
    if method == "GET" && path_only == "/api/pairing/status" {
        return Some(pairing_status_payload(root, path));
    }
    if method == "POST" && path_only == "/api/pairing/confirm" {
        return Some(pairing_transition_payload(root, body, "confirmed"));
    }
    if method == "POST" && path_only == "/api/pairing/cancel" {
        return Some(pairing_transition_payload(root, body, "cancelled"));
    }

    if method == "GET" && path_only == "/api/uploads" {
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "uploads": uploads_list(root)}),
        });
    }
    if method == "POST" && path_only == "/api/uploads" {
        return Some(upload_create_payload(root, body));
    }
    if method == "GET" {
        if let Some(upload_id) = path_only.strip_prefix("/api/uploads/") {
            return Some(upload_detail_payload(root, upload_id));
        }
    }
    if method == "DELETE" {
        if let Some(upload_id) = path_only.strip_prefix("/api/uploads/") {
            return Some(upload_delete_payload(root, upload_id));
        }
    }

    if method == "GET" && path_only == "/api/logs/stream" {
        return Some(stream_payload(root, "logs"));
    }
    if method == "GET" && path_only == "/api/comms/events/stream" {
        return Some(stream_payload(root, "comms_events"));
    }

    if let Some(response) =
        handle_marketplace_aliases(root, method, path, path_only, snapshot, body)
    {
        return Some(response);
    }

    None
}
