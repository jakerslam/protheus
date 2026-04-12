#[cfg(test)]
mod openclaw_media_qr_tests {
    use super::*;

    #[test]
    fn openclaw_qr_image_generates_png_artifact_and_inline_data_url() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_qr_image(
            tmp.path(),
            &json!({
                "text": "https://example.com/qr",
                "summary_only": false
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("content_type").and_then(Value::as_str), Some("image/png"));
        let data_url = out.get("data_url").and_then(Value::as_str).unwrap_or("");
        assert!(data_url.starts_with("data:image/png;base64,"));
        let artifact_path = out
            .pointer("/artifact/path")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(artifact_path.ends_with(".png"));
        assert!(Path::new(artifact_path).exists());
    }

    #[test]
    fn openclaw_qr_image_offloaded_order_omits_inline_data_url() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_qr_image(
            tmp.path(),
            &json!({
                "text": "hello",
                "prompt_image_order": "offloaded"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("prompt_image_order").and_then(Value::as_str),
            Some("offloaded")
        );
        assert!(out.get("data_url").is_some_and(Value::is_null));
    }

    #[test]
    fn openclaw_qr_image_rejects_invalid_prompt_order() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_qr_image(
            tmp.path(),
            &json!({
                "text": "hello",
                "prompt_image_order": "diagonal"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("invalid_prompt_image_order")
        );
    }

    #[test]
    fn openclaw_parse_media_reports_audio_tag_contract_fields() {
        let out = api_parse_media(&json!({
            "text": "hello [[audio_as_voice]]\nMEDIA:\"clip.ogg\""
        }));
        assert_eq!(out.get("audio_as_voice").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("had_audio_tag").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("audio_delivery_mode").and_then(Value::as_str),
            Some("voice")
        );
    }
}
