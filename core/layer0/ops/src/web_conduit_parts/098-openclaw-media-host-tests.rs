#[cfg(test)]
mod openclaw_media_host_tests {
    use super::*;
    use base64::Engine;

    const TINY_PNG_BASE64: &str =
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/woAAn8B9FD5fHAAAAAASUVORK5CYII=";

    fn tiny_png_bytes() -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(TINY_PNG_BASE64)
            .expect("png")
    }

    #[test]
    fn openclaw_media_host_roundtrip_creates_delivery_route_and_cleans_up() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("tiny.png");
        fs::write(&target, tiny_png_bytes()).expect("png");

        let hosted = api_media_host(
            tmp.path(),
            &json!({
                "path": target.display().to_string(),
                "local_roots": "any"
            }),
        );
        assert_eq!(hosted.get("ok").and_then(Value::as_bool), Some(true));
        let hosted_id = clean_text(hosted.get("id").and_then(Value::as_str).unwrap_or(""), 220);
        assert!(hosted
            .get("route_path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .starts_with("/api/web/media/"));

        let delivered = api_media_host_read(tmp.path(), &hosted_id);
        assert_eq!(delivered.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            delivered.get("content_type").and_then(Value::as_str),
            Some("image/png")
        );
        assert!(delivered
            .get("data_url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .starts_with("data:image/png;base64,"));

        let missing = api_media_host_read(tmp.path(), &hosted_id);
        assert_eq!(missing.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            missing.get("error").and_then(Value::as_str),
            Some("not-found")
        );
    }

    #[test]
    fn openclaw_media_host_expires_entries_fail_closed() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("tiny.png");
        fs::write(&target, tiny_png_bytes()).expect("png");

        let hosted = api_media_host(
            tmp.path(),
            &json!({
                "path": target.display().to_string(),
                "local_roots": "any"
            }),
        );
        let hosted_id = clean_text(hosted.get("id").and_then(Value::as_str).unwrap_or(""), 220);
        let manifest_path = hosted_media_manifest_path(tmp.path(), &hosted_id);
        let mut manifest = read_json_or(&manifest_path, Value::Null);
        manifest["expires_at"] = json!("2000-01-01T00:00:00Z");
        write_json_atomic(&manifest_path, &manifest).expect("manifest");

        let expired = api_media_host_read(tmp.path(), &hosted_id);
        assert_eq!(expired.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            expired.get("error").and_then(Value::as_str),
            Some("expired")
        );
    }

    #[test]
    fn openclaw_media_host_rejects_invalid_ids() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let invalid = api_media_host_read(tmp.path(), "../escape");
        assert_eq!(invalid.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            invalid.get("error").and_then(Value::as_str),
            Some("invalid-path")
        );
    }

    #[test]
    fn openclaw_media_host_read_rejects_outside_workspace_manifests() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        let target = tmp.path().join("tiny.png");
        let outside_file = outside.path().join("outside.png");
        fs::write(&target, tiny_png_bytes()).expect("png");
        fs::write(&outside_file, tiny_png_bytes()).expect("outside png");

        let hosted = api_media_host(
            tmp.path(),
            &json!({
                "path": target.display().to_string(),
                "local_roots": "any"
            }),
        );
        let hosted_id = clean_text(hosted.get("id").and_then(Value::as_str).unwrap_or(""), 220);
        let manifest_path = hosted_media_manifest_path(tmp.path(), &hosted_id);
        let mut manifest = read_json_or(&manifest_path, Value::Null);
        manifest["path"] = json!(outside_file.display().to_string());
        write_json_atomic(&manifest_path, &manifest).expect("manifest");

        let denied = api_media_host_read(tmp.path(), &hosted_id);
        assert_eq!(denied.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            denied.get("error").and_then(Value::as_str),
            Some("outside-workspace")
        );
    }
}
