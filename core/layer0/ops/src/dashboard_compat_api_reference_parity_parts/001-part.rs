    let mut state = load_parity_state(root);
    let uploads = as_array_mut(&mut state, "uploads");
    uploads.retain(|row| {
        clean_text(
            row.get("upload_id").and_then(Value::as_str).unwrap_or(""),
            120,
        ) != upload_id
    });
    uploads.push(json!({
        "upload_id": upload_id,
        "filename": filename,
        "size_bytes": bytes.len(),
        "hash": digest,
        "path": clean_text(file_path.to_string_lossy().as_ref(), 500),
        "created_at": crate::now_iso(),
        "created_at_ms": now_ms()
    }));
    save_parity_state(root, state.clone());
    let upload = state
        .get("uploads")
        .and_then(Value::as_array)
        .and_then(|rows| rows.last())
        .cloned()
        .unwrap_or_else(|| json!({}));
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "upload": upload}),
    }
}

fn upload_detail_payload(root: &Path, upload_id: &str) -> CompatApiResponse {
    let needle = clean_text(upload_id, 120);
    let Some(row) = uploads_list(root).into_iter().find(|row| {
        clean_text(
            row.get("upload_id").and_then(Value::as_str).unwrap_or(""),
            120,
        ) == needle
    }) else {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "upload_not_found", "upload_id": needle}),
        };
    };
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "upload": row}),
    }
}

fn upload_delete_payload(root: &Path, upload_id: &str) -> CompatApiResponse {
    let needle = clean_text(upload_id, 120);
    let mut state = load_parity_state(root);
    let rows = as_array_mut(&mut state, "uploads");
    let mut removed = None;
    rows.retain(|row| {
        let keep = clean_text(
            row.get("upload_id").and_then(Value::as_str).unwrap_or(""),
            120,
        ) != needle;
        if !keep {
            removed = row
                .get("path")
                .and_then(Value::as_str)
                .map(|v| v.to_string());
        }
        keep
    });
    if let Some(path) = removed {
        let _ = fs::remove_file(path);
        save_parity_state(root, state);
        return CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "deleted": true, "upload_id": needle}),
        };
    }
    CompatApiResponse {
        status: 404,
        payload: json!({"ok": false, "error": "upload_not_found", "upload_id": needle}),
    }
}

fn recent_log_events(root: &Path, limit: usize) -> Vec<Value> {
    let mut rows = fs::read_to_string(state_path(root, ACTION_HISTORY_REL))
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if rows.len() > limit {
        rows = rows.split_off(rows.len() - limit);
    }
    rows
}

fn stream_payload(root: &Path, stream: &str) -> CompatApiResponse {
    let events = recent_log_events(root, 60);
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "stream": clean_text(stream, 80),
            "events": events,
            "count": events.len(),
            "cursor": stable_hash(&format!("{}|{}", stream, now_ms()), 12)
        }),
    }
}

fn handle_marketplace_aliases(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    snapshot: &Value,
    body: &[u8],
) -> Option<CompatApiResponse> {
    if method == "GET"
        && (path_only == "/api/marketplace" || path_only == "/api/marketplace/browse")
    {
        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("limit=25");
        let forwarded = format!("/api/clawhub/browse?{query}");
        return super::dashboard_skills_marketplace::handle(root, "GET", &forwarded, snapshot, &[]);
    }
    if method == "GET" && path_only == "/api/marketplace/search" {
        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("q=");
        let forwarded = format!("/api/clawhub/search?{query}");
        return super::dashboard_skills_marketplace::handle(root, "GET", &forwarded, snapshot, &[]);
    }
    if method == "GET" {
        if let Some(slug) = path_only.strip_prefix("/api/marketplace/skill/") {
            let forwarded = format!("/api/clawhub/skill/{}", clean_text(slug, 160));
            return super::dashboard_skills_marketplace::handle(
                root,
                "GET",
                &forwarded,
                snapshot,
                &[],
            );
        }
    }
    if method == "POST" && path_only == "/api/skills/install" {
        return super::dashboard_skills_marketplace::handle(
            root,
            "POST",
            "/api/clawhub/install",
            snapshot,
            body,
        );
    }
    if method == "POST" && path_only == "/api/skills/reload" {
        if let Some(listed) =
            super::dashboard_skills_marketplace::handle(root, "GET", "/api/skills", snapshot, &[])
        {
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "reloaded": true, "skills": listed.payload.get("skills").cloned().unwrap_or_else(|| Value::Array(Vec::new()))}),
            });
        }
    }
    None
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_models_endpoint_returns_list_shape() {
        let root = tempfile::tempdir().expect("tempdir");
        let response = handle(
            root.path(),
            "GET",
            "/v1/models",
            "/v1/models",
            &[],
            &[],
            &json!({"ok": true}),
        )
        .expect("models response");
        assert_eq!(response.status, 200);
        assert_eq!(
            response
                .payload
                .get("object")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "list"
        );
        assert!(response
            .payload
            .get("data")
            .map(Value::is_array)
            .unwrap_or(false));
    }

    #[test]
    fn auth_login_logout_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let login = handle(
            root.path(),
            "POST",
            "/api/auth/login",
            "/api/auth/login",
            &[("host", "localhost:4200")],
            br#"{"email":"ops@example.com"}"#,
            &json!({}),
        )
        .expect("login");
        assert_eq!(login.status, 200);
        let token = clean_text(
            login
                .payload
                .get("token")
                .and_then(Value::as_str)
                .unwrap_or(""),
            200,
        );
        assert!(!token.is_empty());

        let logout = handle(
            root.path(),
            "POST",
            "/api/auth/logout",
            "/api/auth/logout",
            &[],
            &[],
            &json!({}),
        )
        .expect("logout");
        assert_eq!(logout.status, 200);
        assert!(logout
            .payload
            .get("logged_out")
            .and_then(Value::as_bool)
            .unwrap_or(false));
    }

    #[test]
    fn integrations_aliases_to_channels() {
        let root = tempfile::tempdir().expect("tempdir");
        let list = handle(
            root.path(),
            "GET",
            "/api/integrations",
            "/api/integrations",
            &[],
            &[],
            &json!({}),
        )
        .expect("integrations");
        assert_eq!(list.status, 200);
        let items = list
            .payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!items.is_empty());

        let configure = handle(
            root.path(),
            "POST",
            "/api/integrations/telegram/configure",
            "/api/integrations/telegram/configure",
            &[],
            br#"{"token":"abc","endpoint":"https://api.telegram.org"}"#,
            &json!({}),
        )
        .expect("configure");
        assert_eq!(configure.status, 200);
        assert!(configure
            .payload
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false));
    }

    #[test]
    fn pairing_start_status_confirm_flow() {
        let root = tempfile::tempdir().expect("tempdir");
        let started = handle(
            root.path(),
            "POST",
            "/api/pairing/start",
            "/api/pairing/start",
            &[],
            &[],
            &json!({}),
        )
        .expect("pair start");
        let pairing_id = clean_text(
            started
                .payload
                .get("pairing_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        assert!(!pairing_id.is_empty());

        let status = handle(
            root.path(),
            "GET",
            &format!("/api/pairing/status?pairing_id={pairing_id}"),
            "/api/pairing/status",
            &[],
            &[],
            &json!({}),
        )
        .expect("pair status");
        assert_eq!(status.status, 200);

        let confirmed = handle(
            root.path(),
            "POST",
            "/api/pairing/confirm",
            "/api/pairing/confirm",
            &[],
            format!(r#"{{"pairing_id":"{pairing_id}"}}"#).as_bytes(),
            &json!({}),
        )
        .expect("pair confirm");
        assert_eq!(
            confirmed
                .payload
                .get("status")
                .and_then(Value::as_str)
