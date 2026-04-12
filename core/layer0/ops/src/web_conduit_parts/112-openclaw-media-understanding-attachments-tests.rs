#[cfg(test)]
mod openclaw_media_attachments_tests {
    use super::*;

    #[test]
    fn openclaw_media_attachments_normalize_path_allows_localhost_file_urls_and_blocks_remote_hosts()
    {
        assert_eq!(
            normalize_attachment_path("file://localhost/Users/alice/Pictures/photo.png"),
            Some("/Users/alice/Pictures/photo.png".to_string())
        );
        assert_eq!(
            normalize_attachment_path("file://attacker/share/photo.png"),
            None
        );
    }

    #[test]
    fn openclaw_media_attachments_guards_malformed_entries() {
        let out = api_media_attachments(&json!({
            "attachments": [
                null,
                { "index": 1, "path": 123 },
                { "index": 2, "url": true },
                { "index": 3, "mime": { "bad": true } }
            ],
            "capability": "audio",
            "prefer": "path"
        }));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("attachment_count").and_then(Value::as_u64), Some(0));
        assert_eq!(out.get("selected_count").and_then(Value::as_u64), Some(0));
    }

    #[test]
    fn openclaw_media_attachments_normalize_context_arrays_and_select_by_preference() {
        let out = api_media_attachments(&json!({
            "MediaPaths": [
                "file://localhost/Users/alice/Pictures/photo.png",
                "/Users/alice/Movies/clip.mp4"
            ],
            "MediaUrls": [
                "https://example.com/photo.png",
                "https://example.com/clip.mp4"
            ],
            "MediaTypes": ["image/png", "video/mp4"],
            "capability": "image",
            "prefer": "path",
            "mode": "all",
            "max_attachments": 2
        }));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("attachment_count").and_then(Value::as_u64), Some(2));
        assert_eq!(out.get("selected_count").and_then(Value::as_u64), Some(1));
        assert_eq!(
            out.pointer("/selected_attachments/0/path")
                .and_then(Value::as_str),
            Some("/Users/alice/Pictures/photo.png")
        );
        assert_eq!(
            out.pointer("/selected_attachments/0/kind")
                .and_then(Value::as_str),
            Some("image")
        );
    }

    #[test]
    fn openclaw_media_attachments_skip_already_transcribed_audio() {
        let out = api_media_attachments(&json!({
            "attachments": [
                { "index": 0, "path": "/tmp/clip1.mp3", "mime": "audio/mpeg", "alreadyTranscribed": true },
                { "index": 1, "path": "/tmp/clip2.mp3", "mime": "audio/mpeg" }
            ],
            "capability": "audio",
            "mode": "all",
            "max_attachments": 2
        }));
        assert_eq!(out.get("selected_count").and_then(Value::as_u64), Some(1));
        assert_eq!(
            out.pointer("/selected_attachments/0/index")
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn openclaw_media_attachments_contract_is_exposed_in_status_tool_catalog() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_status(tmp.path());
        assert_eq!(
            out.pointer("/media_request_contract/attachments_contract/capabilities")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str() == Some("image"))),
            Some(true)
        );
        assert!(out
            .pointer("/tool_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.get("tool").and_then(Value::as_str) == Some("web_media_attachments")
            }))
            .unwrap_or(false));
    }
}
