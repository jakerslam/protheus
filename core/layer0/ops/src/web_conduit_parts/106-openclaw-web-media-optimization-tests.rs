#[cfg(test)]
mod openclaw_web_media_optimization_tests {
    use super::*;
    use base64::Engine;

    const TINY_ALPHA_PNG_BASE64: &str =
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/woAAn8B9FD5fHAAAAAASUVORK5CYII=";

    fn tiny_alpha_png() -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(TINY_ALPHA_PNG_BASE64)
            .expect("tiny png")
    }

    fn tiny_loaded_media() -> LoadedMedia {
        LoadedMedia {
            buffer: tiny_alpha_png(),
            content_type: "image/png".to_string(),
            kind: "image".to_string(),
            file_name: "tiny.png".to_string(),
            resolved_source: "/tmp/tiny.png".to_string(),
            source_kind: "local".to_string(),
            status_code: 200,
            provider: "direct_http".to_string(),
            provider_hint: "local".to_string(),
            citation_redirect_resolved: false,
            redirect_count: 0,
        }
    }

    #[test]
    fn openclaw_web_media_contract_reports_image_optimization_surface() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let status = api_status(tmp.path());
        assert_eq!(
            status
                .pointer("/media_request_contract/image_optimization_contract/default_optimize_images")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            status
                .pointer("/media_request_contract/image_optimization_contract/raw_alias_disables_optimization")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(status
            .pointer("/tool_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.get("tool").and_then(Value::as_str) == Some("web_media")
                    && row
                        .pointer("/request_contract/image_optimization_contract/default_optimize_images")
                        .and_then(Value::as_bool)
                        == Some(true)
            }))
            .unwrap_or(false));
    }

    #[test]
    fn openclaw_web_media_prefetch_cap_keeps_document_ceiling_when_optimizing() {
        assert_eq!(
            media_prefetch_max_bytes(&json!({"max_bytes": 512u64})),
            MAX_DOCUMENT_BYTES
        );
        assert_eq!(
            media_prefetch_max_bytes(&json!({"max_bytes": 512u64, "raw": true})),
            512
        );
    }

    #[test]
    fn openclaw_web_media_png_header_alpha_detection_matches_openclaw() {
        assert!(png_has_alpha_channel(&tiny_alpha_png()));
    }

    #[test]
    fn openclaw_web_media_raw_alias_disables_image_optimization() {
        let finalized =
            finalize_loaded_media_for_request(tiny_loaded_media(), &json!({"raw": true}))
                .expect("finalized");
        assert!(!finalized.optimized);
        assert!(!finalized.optimize_images);
        assert_eq!(
            finalized
                .optimization
                .get("reason")
                .and_then(Value::as_str),
            Some("raw_passthrough")
        );
        assert_eq!(finalized.loaded.content_type, "image/png");
    }

    #[test]
    fn openclaw_web_media_default_image_path_emits_png_optimization_metadata() {
        let finalized = finalize_loaded_media_for_request(tiny_loaded_media(), &json!({}))
            .expect("finalized");
        assert!(finalized.optimize_images);
        assert_eq!(finalized.loaded.content_type, "image/png");
        assert_eq!(
            finalized
                .optimization
                .get("enabled")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
}
