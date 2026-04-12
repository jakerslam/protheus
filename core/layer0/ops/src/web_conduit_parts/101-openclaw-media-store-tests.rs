#[cfg(test)]
mod openclaw_media_store_tests {
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
    fn openclaw_media_store_extracts_original_filename_from_saved_id() {
        let restored = media_store_extract_original_filename(
            "/tmp/outbound/my_report---0123456789abcdef01234567.pdf",
        );
        assert_eq!(restored, "my_report.pdf");
        assert_eq!(
            media_store_extract_original_filename("/tmp/outbound/plain-file.png"),
            "plain-file.png"
        );
    }

    #[test]
    fn openclaw_media_store_save_buffer_preserves_filename_prefix_and_mime() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let saved = save_media_buffer_to_store(
            tmp.path(),
            &tiny_png_bytes(),
            Some("application/octet-stream"),
            "inbound",
            5 * 1024 * 1024,
            Some("My Report.png"),
        )
        .expect("saved");
        let saved_id = saved.get("id").and_then(Value::as_str).unwrap_or("");
        assert!(saved_id.starts_with("My_Report---"));
        assert!(saved_id.ends_with(".png"));
        assert_eq!(
            saved.get("content_type").and_then(Value::as_str),
            Some("image/png")
        );
        let saved_path = saved.get("path").and_then(Value::as_str).unwrap_or("");
        assert!(Path::new(saved_path).exists());
        assert_eq!(
            media_store_extract_original_filename(saved_path),
            "My_Report.png"
        );
    }

    #[test]
    fn openclaw_media_store_resolve_and_delete_are_fail_closed() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let saved = save_media_buffer_to_store(
            tmp.path(),
            b"hello",
            Some("text/plain"),
            "inbound",
            5 * 1024 * 1024,
            Some("notes.txt"),
        )
        .expect("saved");
        let saved_id = saved.get("id").and_then(Value::as_str).unwrap_or("");
        let resolved = resolve_saved_media_path(tmp.path(), saved_id, "inbound").expect("resolve");
        assert!(resolved.exists());
        let bad = resolve_saved_media_path(tmp.path(), "../evil", "inbound").expect_err("bad");
        assert_eq!(bad.get("error").and_then(Value::as_str), Some("invalid-path"));
        let deleted = delete_saved_media(tmp.path(), saved_id, "inbound").expect("delete");
        assert_eq!(deleted.get("ok").and_then(Value::as_bool), Some(true));
        assert!(!resolved.exists());
    }

    #[test]
    fn openclaw_media_store_cleanup_prunes_expired_files_and_empty_dirs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let saved = save_media_buffer_to_store(
            tmp.path(),
            b"hello",
            Some("text/plain"),
            "nested/store",
            5 * 1024 * 1024,
            Some("notes.txt"),
        )
        .expect("saved");
        let saved_path = PathBuf::from(saved.get("path").and_then(Value::as_str).unwrap_or(""));
        std::thread::sleep(std::time::Duration::from_millis(10));
        clean_old_media_store(tmp.path(), 1, true, true).expect("cleanup");
        assert!(!saved_path.exists());
        let nested_dir = media_store_dir_path(tmp.path(), "nested/store");
        assert!(!nested_dir.exists() || fs::read_dir(&nested_dir).ok().and_then(|mut rows| rows.next()).is_none());
    }

    #[test]
    fn openclaw_outbound_attachment_maps_outside_workspace_to_invalid_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        let target = outside.path().join("leak.png");
        fs::write(&target, tiny_png_bytes()).expect("png");
        let out = api_outbound_attachment(
            tmp.path(),
            &json!({
                "path": target.display().to_string()
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("error").and_then(Value::as_str), Some("invalid-path"));
        assert_eq!(
            out.get("message").and_then(Value::as_str),
            Some("Media path is outside workspace root")
        );
    }

    #[test]
    fn openclaw_media_store_contract_is_visible_in_status() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let status = api_status(tmp.path());
        assert_eq!(
            status
                .pointer("/media_request_contract/media_store_contract/root")
                .and_then(Value::as_str),
            Some("client/runtime/local/state/web_conduit/stored_media")
        );
    }
}
