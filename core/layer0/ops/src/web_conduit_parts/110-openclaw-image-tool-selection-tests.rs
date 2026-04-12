#[cfg(test)]
mod openclaw_image_tool_selection_tests {
    use super::*;

    #[test]
    fn openclaw_image_tool_contract_reports_defaults_and_selection_only_gap() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = default_policy();
        let contract =
            crate::web_conduit_provider_runtime::web_image_tool_contract(tmp.path(), &policy);
        assert_eq!(
            contract.get("default_prompt").and_then(Value::as_str),
            Some("Describe the image.")
        );
        assert_eq!(contract.get("max_images").and_then(Value::as_u64), Some(20));
        assert_eq!(
            contract
                .pointer("/execution_contract/mode")
                .and_then(Value::as_str),
            Some("selection_only")
        );
        assert_eq!(
            contract
                .pointer("/execution_contract/gap")
                .and_then(Value::as_str),
            Some("multimodal_transport_not_enabled")
        );
    }

    #[test]
    fn openclaw_image_tool_runtime_prefers_ready_provider_and_default_model() {
        let tmp = tempfile::tempdir().expect("tempdir");
        crate::dashboard_provider_runtime::save_provider_key(
            tmp.path(),
            "openai",
            "sk-test-openai",
        );

        let out = api_image_tool_status(tmp.path(), &json!({"provider": "openai"}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/runtime/selected_provider")
                .and_then(Value::as_str),
            Some("openai")
        );
        assert_eq!(
            out.pointer("/runtime/selected_model")
                .and_then(Value::as_str),
            Some("gpt-4o")
        );
        assert_eq!(
            out.pointer("/runtime/selection_scope")
                .and_then(Value::as_str),
            Some("request_provider")
        );
        assert_eq!(
            out.pointer("/runtime/selection_ready")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn openclaw_image_tool_runtime_falls_back_from_invalid_configured_provider() {
        let tmp = tempfile::tempdir().expect("tempdir");
        crate::dashboard_provider_runtime::save_provider_key(
            tmp.path(),
            "openai",
            "sk-test-openai",
        );
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "image_tool": {
                        "provider": "xai"
                    }
                }
            }),
        )
        .expect("write policy");

        let out = api_image_tool_status(tmp.path(), &json!({}));
        assert_eq!(
            out.pointer("/runtime/selected_provider")
                .and_then(Value::as_str),
            Some("openai")
        );
        assert_eq!(
            out.pointer("/runtime/selection_fallback_reason")
                .and_then(Value::as_str),
            Some("invalid_configured_provider")
        );
        assert!(out
            .pointer("/runtime/diagnostics")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.get("code").and_then(Value::as_str)
                    == Some("WEB_IMAGE_TOOL_PROVIDER_INVALID_FALLBACK_USED")
            }))
            .unwrap_or(false));
    }

    #[test]
    fn openclaw_image_tool_runtime_metadata_is_persisted_in_status_snapshot() {
        let tmp = tempfile::tempdir().expect("tempdir");
        crate::dashboard_provider_runtime::save_provider_key(
            tmp.path(),
            "openai",
            "sk-test-openai",
        );
        let status = api_status(tmp.path());
        assert_eq!(
            status
                .pointer("/runtime_web_tools_metadata/image_tool/selected_provider")
                .and_then(Value::as_str),
            Some("openai")
        );
        assert_eq!(
            status
                .pointer("/image_tool_contract/default_prompt")
                .and_then(Value::as_str),
            Some("Describe the image.")
        );
        assert!(status
            .pointer("/tool_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| { row.get("tool").and_then(Value::as_str) == Some("web_image_tool") }))
            .unwrap_or(false));
    }
}
