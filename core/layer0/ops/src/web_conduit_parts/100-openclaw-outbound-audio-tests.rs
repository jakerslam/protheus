#[cfg(test)]
mod openclaw_outbound_audio_tests {
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
    fn openclaw_outbound_media_request_uses_media_access_when_top_level_missing() {
        let request = build_outbound_media_request(&json!({
            "url": "https://example.com/image.png",
            "media_access": {
                "workspace_dir": "/tmp/media-workspace",
                "local_roots": ["/tmp/media-workspace/assets"],
                "host_read_capability": true
            }
        }));
        assert_eq!(
            request.get("workspace_dir").and_then(Value::as_str),
            Some("/tmp/media-workspace")
        );
        assert_eq!(
            request.pointer("/local_roots/0").and_then(Value::as_str),
            Some("/tmp/media-workspace/assets")
        );
        assert_eq!(
            request.get("host_read_capability").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn openclaw_outbound_media_request_prefers_top_level_over_media_access() {
        let request = build_outbound_media_request(&json!({
            "path": "chart.png",
            "workspace_dir": "/tmp/top-level",
            "local_roots": ["/tmp/top-level/assets"],
            "media_access": {
                "workspace_dir": "/tmp/nested",
                "local_roots": ["/tmp/nested/assets"],
                "host_read_capability": false
            },
            "host_read_capability": true
        }));
        assert_eq!(
            request.get("workspace_dir").and_then(Value::as_str),
            Some("/tmp/top-level")
        );
        assert_eq!(
            request.pointer("/local_roots/0").and_then(Value::as_str),
            Some("/tmp/top-level/assets")
        );
        assert_eq!(
            request.get("host_read_capability").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn openclaw_audio_voice_compatibility_respects_supported_mime_and_extension() {
        assert!(is_telegram_voice_compatible_audio(
            Some("audio/mp4; codecs=mp4a.40.2"),
            None
        ));
        assert!(is_telegram_voice_compatible_audio(None, Some("voice.ogg")));
        assert!(!is_telegram_voice_compatible_audio(
            Some("audio/wav"),
            Some("voice.wav")
        ));
    }

    #[test]
    fn openclaw_media_reports_voice_compatible_audio_for_m4a_extension() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let workspace = tmp.path().join("workspace");
        fs::create_dir_all(&workspace).expect("workspace");
        let target = workspace.join("voice.m4a");
        fs::write(&target, b"not really audio").expect("audio stub");
        let out = api_media(
            tmp.path(),
            &json!({
                "path": "voice.m4a",
                "workspace_dir": workspace.display().to_string()
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("content_type").and_then(Value::as_str),
            Some("audio/x-m4a")
        );
        assert_eq!(
            out.get("voice_compatible_audio").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn openclaw_outbound_attachment_saves_media_with_original_filename_prefix() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let workspace = tmp.path().join("workspace");
        fs::create_dir_all(&workspace).expect("workspace");
        let target = workspace.join("tiny image.png");
        fs::write(&target, tiny_png_bytes()).expect("png");
        let out = api_outbound_attachment(
            tmp.path(),
            &json!({
                "path": "tiny image.png",
                "workspace_dir": workspace.display().to_string()
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("web_conduit_outbound_attachment")
        );
        assert_eq!(
            out.get("file_name").and_then(Value::as_str),
            Some("tiny image.png")
        );
        let saved_id = out
            .pointer("/saved_media/id")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(saved_id.starts_with("tiny_image---"));
        assert!(saved_id.ends_with(".png"));
        let saved_path = out.get("path").and_then(Value::as_str).unwrap_or("");
        assert!(saved_path.contains("/stored_media/outbound/"));
        assert!(Path::new(saved_path).exists());
        assert_eq!(
            out.get("content_type").and_then(Value::as_str),
            Some("image/png")
        );
    }

    #[test]
    fn openclaw_outbound_attachment_contract_is_exposed_in_status_tool_catalog() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let status = api_status(tmp.path());
        assert_eq!(
            status
                .pointer("/media_request_contract/voice_audio_contract/voice_compatible_field")
                .and_then(Value::as_str),
            Some("voice_compatible_audio")
        );
        assert_eq!(
            status
                .pointer("/media_request_contract/outbound_attachment_contract/default_store_subdir")
                .and_then(Value::as_str),
            Some("outbound")
        );
        let has_tool = status
            .get("tool_catalog")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter().any(|row| {
                    row.get("tool").and_then(Value::as_str)
                        == Some("web_media_outbound_attachment")
                })
            })
            .unwrap_or(false);
        assert!(has_tool);
    }
}
