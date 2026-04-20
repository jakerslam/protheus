
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
                .unwrap_or(""),
            "confirmed"
        );
    }

    #[test]
    fn uploads_create_list_detail_delete() {
        let root = tempfile::tempdir().expect("tempdir");
        let created = handle(
            root.path(),
            "POST",
            "/api/uploads",
            "/api/uploads",
            &[],
            br#"{"filename":"notes.txt","content":"hello"}"#,
            &json!({}),
        )
        .expect("upload create");
        assert_eq!(created.status, 200);
        let upload_id = clean_text(
            created
                .payload
                .pointer("/upload/upload_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        assert!(!upload_id.is_empty());

        let listed = handle(
            root.path(),
            "GET",
            "/api/uploads",
            "/api/uploads",
            &[],
            &[],
            &json!({}),
        )
        .expect("uploads list");
        let rows = listed
            .payload
            .get("uploads")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 1);

        let detail = handle(
            root.path(),
            "GET",
            &format!("/api/uploads/{upload_id}"),
            &format!("/api/uploads/{upload_id}"),
            &[],
            &[],
            &json!({}),
        )
        .expect("upload detail");
        assert_eq!(
            clean_text(
                detail
                    .payload
                    .pointer("/upload/upload_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120
            ),
            upload_id
        );

        let deleted = handle(
            root.path(),
            "DELETE",
            &format!("/api/uploads/{upload_id}"),
            &format!("/api/uploads/{upload_id}"),
            &[],
            &[],
            &json!({}),
        )
        .expect("upload delete");
        assert!(deleted
            .payload
            .get("deleted")
            .and_then(Value::as_bool)
            .unwrap_or(false));
    }

    #[test]
    fn marketplace_aliases_and_reload_work() {
        let root = tempfile::tempdir().expect("tempdir");
        let browse = handle(
            root.path(),
            "GET",
            "/api/marketplace?limit=5",
            "/api/marketplace",
            &[],
            &[],
            &json!({}),
        )
        .expect("marketplace browse");
        assert_eq!(browse.status, 200);
        assert!(browse
            .payload
            .get("items")
            .map(Value::is_array)
            .unwrap_or(false));

        let reload = handle(
            root.path(),
            "POST",
            "/api/skills/reload",
            "/api/skills/reload",
            &[],
            &[],
            &json!({}),
        )
        .expect("skills reload");
        assert_eq!(reload.status, 200);
        assert!(reload
            .payload
            .get("reloaded")
            .and_then(Value::as_bool)
            .unwrap_or(false));
    }
}
