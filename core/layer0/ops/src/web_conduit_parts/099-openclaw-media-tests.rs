#[cfg(test)]
mod openclaw_media_tests {
    use super::*;
    use base64::Engine;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    const TINY_PNG_BASE64: &str =
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/woAAn8B9FD5fHAAAAAASUVORK5CYII=";

    fn tiny_png_bytes() -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(TINY_PNG_BASE64)
            .expect("png")
    }

    fn write_test_policy(root: &Path) {
        write_json_atomic(
            &policy_path(root),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "allow_domains": ["127.0.0.1"],
                    "deny_domains": [],
                    "sensitive_domains": [],
                    "require_human_for_sensitive": false,
                    "fetch_provider_order": ["direct_http"],
                    "search_provider_order": ["duckduckgo"]
                }
            }),
        )
        .expect("policy");
    }

    fn run_http_server(response: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            if let Ok((mut socket, _)) = listener.accept() {
                let mut buffer = [0u8; 1024];
                let _ = socket.read(&mut buffer);
                let _ = socket.write_all(&response);
            }
        });
        format!("http://{}", addr)
    }

    #[test]
    fn openclaw_media_rejects_remote_host_file_urls() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_media(tmp.path(), &json!({"url": "file://attacker/share/evil.png"}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("invalid-file-url")
        );
    }

    #[test]
    fn openclaw_media_rejects_windows_network_paths() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_media(tmp.path(), &json!({"path": "\\\\attacker\\share\\evil.png"}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("network-path-not-allowed")
        );
    }

    #[test]
    fn openclaw_media_resolves_relative_workspace_paths() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let workspace = tmp.path().join("workspace");
        fs::create_dir_all(&workspace).expect("workspace");
        let target = workspace.join("chart.png");
        fs::write(&target, tiny_png_bytes()).expect("png");
        let out = api_media(
            tmp.path(),
            &json!({
                "path": "chart.png",
                "workspace_dir": workspace.display().to_string()
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("kind").and_then(Value::as_str), Some("image"));
        assert_eq!(out.get("source_kind").and_then(Value::as_str), Some("local"));
    }

    #[test]
    fn openclaw_media_preserves_media_access_workspace_dir_when_top_level_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let workspace = tmp.path().join("media-workspace");
        fs::create_dir_all(&workspace).expect("workspace");
        let target = workspace.join("chart.png");
        fs::write(&target, tiny_png_bytes()).expect("png");
        let out = api_media(tmp.path(), &json!({"path": "chart.png", "media_access": {"workspace_dir": workspace.display().to_string()}}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("file_name").and_then(Value::as_str), Some("chart.png"));
    }

    #[test]
    fn openclaw_media_prefers_explicit_workspace_dir_over_media_access_workspace_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let nested_workspace = tmp.path().join("nested-workspace");
        fs::create_dir_all(&nested_workspace).expect("nested");
        fs::write(nested_workspace.join("chart.png"), tiny_png_bytes()).expect("nested png");
        let explicit_workspace = tmp.path().join("explicit-workspace");
        fs::create_dir_all(&explicit_workspace).expect("explicit");
        fs::write(explicit_workspace.join("chart.png"), tiny_png_bytes()).expect("explicit png");
        let out = api_media(tmp.path(), &json!({"path": "chart.png", "workspace_dir": explicit_workspace.display().to_string(), "media_access": {"workspace_dir": nested_workspace.display().to_string()}}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("resolved_source").and_then(Value::as_str), Some(explicit_workspace.join("chart.png").to_string_lossy().as_ref()));
    }

    #[test]
    fn openclaw_media_loads_localhost_file_urls() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("chart.png");
        fs::write(&target, tiny_png_bytes()).expect("png");
        let out = api_media(
            tmp.path(),
            &json!({
                "url": format!("file://localhost{}", target.display()),
                "local_roots": "any"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("file_name").and_then(Value::as_str), Some("chart.png"));
        assert_eq!(out.get("source_kind").and_then(Value::as_str), Some("local"));
    }

    #[test]
    fn openclaw_media_uses_media_access_local_roots_when_top_level_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("scoped/chart.png");
        fs::create_dir_all(target.parent().expect("parent")).expect("dirs");
        fs::write(&target, tiny_png_bytes()).expect("png");
        let out = api_media(tmp.path(), &json!({"path": target.display().to_string(), "media_access": {"local_roots": [tmp.path().join("scoped").display().to_string()]}}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("file_name").and_then(Value::as_str), Some("chart.png"));
    }

    #[test]
    fn openclaw_media_rejects_filesystem_root_local_roots() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("chart.png");
        fs::write(&target, tiny_png_bytes()).expect("png");
        let out = api_media(
            tmp.path(),
            &json!({
                "path": target.display().to_string(),
                "local_roots": "/"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("invalid-root")
        );
    }

    #[test]
    fn openclaw_media_accepts_wildcard_local_roots_for_attachment_paths() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp
            .path()
            .join("Users/alice/Library/Messages/Attachments/12/34/IMG_0001.png");
        fs::create_dir_all(target.parent().expect("parent")).expect("dirs");
        fs::write(&target, tiny_png_bytes()).expect("png");
        let out = api_media(
            tmp.path(),
            &json!({
                "path": target.display().to_string(),
                "local_roots": format!("{}/Users/*/Library/Messages/Attachments", tmp.path().display())
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("kind").and_then(Value::as_str), Some("image"));
        assert_eq!(out.get("file_name").and_then(Value::as_str), Some("IMG_0001.png"));
    }

    #[test]
    fn openclaw_media_rejects_invalid_double_star_local_root_patterns() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("chart.png");
        fs::write(&target, tiny_png_bytes()).expect("png");
        let out = api_media(
            tmp.path(),
            &json!({
                "path": target.display().to_string(),
                "local_roots": format!("{}/Users/**/Library/Messages/Attachments", tmp.path().display())
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("error").and_then(Value::as_str), Some("invalid-root"));
    }

    #[test]
    fn openclaw_media_rejects_disguised_host_read_text_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let disguised = tmp.path().join("secret.pdf");
        fs::write(&disguised, "secret").expect("secret");
        let out = api_media(
            tmp.path(),
            &json!({
                "path": disguised.display().to_string(),
                "local_roots": "any",
                "host_read_capability": true
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("path-not-allowed")
        );
    }

    #[test]
    fn openclaw_media_disables_unbounded_host_reads_when_sender_policy_denies_read() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        let target = outside.path().join("sender-denied/chart.png");
        fs::create_dir_all(target.parent().expect("parent")).expect("dirs");
        fs::write(&target, tiny_png_bytes()).expect("png");
        let out = api_media(tmp.path(), &json!({"path": target.display().to_string(), "local_roots": "any", "host_read_capability": true, "sender_tool_policy": {"deny": ["read"]}}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("error").and_then(Value::as_str), Some("path-not-allowed"));
    }

    #[test]
    fn openclaw_media_keeps_unbounded_host_reads_when_no_policy_denies_read() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        let target = outside.path().join("sender-allowed/chart.png");
        fs::create_dir_all(target.parent().expect("parent")).expect("dirs");
        fs::write(&target, tiny_png_bytes()).expect("png");
        let out = api_media(tmp.path(), &json!({"path": target.display().to_string(), "local_roots": "any", "host_read_capability": true}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("kind").and_then(Value::as_str), Some("image"));
    }

    #[test]
    fn openclaw_media_remote_fetch_uses_content_disposition_filename() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_test_policy(tmp.path());
        let png = tiny_png_bytes();
        let response = [
            b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=\"tiny.png\"\r\nContent-Length: ".as_slice(),
            png.len().to_string().as_bytes(),
            b"\r\n\r\n".as_slice(),
            png.as_slice(),
        ]
        .concat();
        let url = run_http_server(response);
        let out = api_media(tmp.path(), &json!({"url": url, "summary_only": true}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("file_name").and_then(Value::as_str), Some("tiny.png"));
        assert_eq!(out.get("kind").and_then(Value::as_str), Some("image"));
        assert_eq!(out.get("source_kind").and_then(Value::as_str), Some("remote"));
    }

    #[test]
    fn openclaw_media_remote_http_error_has_bounded_body_snippet() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_test_policy(tmp.path());
        let body = format!("{}BAD", " ".repeat(9000));
        let response = format!(
            "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .into_bytes();
        let url = run_http_server(response);
        let out = api_media(tmp.path(), &json!({"url": url}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("error").and_then(Value::as_str), Some("http_error"));
        let snippet = out.get("body_snippet").and_then(Value::as_str).unwrap_or("");
        assert!(!snippet.contains("BAD"));
    }

    #[test]
    fn openclaw_media_remote_fetch_prefers_extension_for_generic_content_type() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_test_policy(tmp.path());
        let body = b"body { color: red; }\n".to_vec();
        let response = [
            b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: ".as_slice(),
            body.len().to_string().as_bytes(),
            b"\r\n\r\n".as_slice(),
            body.as_slice(),
        ]
        .concat();
        let base = run_http_server(response);
        let out = api_media(tmp.path(), &json!({"url": format!("{base}/styles.css")}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("content_type").and_then(Value::as_str),
            Some("text/css")
        );
        assert_eq!(out.get("kind").and_then(Value::as_str), Some("document"));
        assert_eq!(out.get("file_name").and_then(Value::as_str), Some("styles.css"));
    }

    #[test]
    fn openclaw_media_remote_fetch_rejects_declared_oversize_content_length() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_test_policy(tmp.path());
        let body = vec![b'a'; 2048];
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            if let Ok((mut socket, _)) = listener.accept() {
                let mut buffer = [0u8; 1024];
                let _ = socket.read(&mut buffer);
                let response = [
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: "
                        .as_slice(),
                    body.len().to_string().as_bytes(),
                    b"\r\nConnection: close\r\n\r\n".as_slice(),
                    body.as_slice(),
                ]
                .concat();
                let _ = socket.write_all(&response);
                let _ = socket.flush();
            }
        });
        let url = format!("http://{addr}/oversize.bin");
        let out = api_media(tmp.path(), &json!({"url": url, "max_bytes": 512}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("error").and_then(Value::as_str), Some("max_bytes"));
        assert_eq!(out.get("declared_size").and_then(Value::as_u64), Some(2048));
    }

    #[test]
    fn openclaw_media_remote_fetch_classifies_stalled_transfer() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_test_policy(tmp.path());
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            if let Ok((mut socket, _)) = listener.accept() {
                let mut buffer = [0u8; 1024];
                let _ = socket.read(&mut buffer);
                let _ = socket.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 10\r\n\r\n1",
                );
                let _ = socket.flush();
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        });
        let out = api_media(
            tmp.path(),
            &json!({
                "url": format!("http://{addr}/stalled.bin"),
                "timeout_ms": 3000,
                "idle_timeout_ms": 1000
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("fetch_stalled")
        );
    }

    #[test]
    fn openclaw_media_mime_helpers_cover_extended_audio_and_archive_types() {
        assert_eq!(
            media_guess_content_type(Some("voice.m4a"), b"hello", Some("application/octet-stream")),
            "audio/x-m4a"
        );
        assert_eq!(
            media_extension_for_content_type("audio/flac"),
            Some("flac")
        );
        assert_eq!(
            media_extension_for_content_type("application/gzip"),
            Some("gz")
        );
        assert_eq!(
            media_extension_for_content_type("text/css"),
            Some("css")
        );
    }

    #[test]
    fn openclaw_media_loads_managed_canvas_document_paths() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let canvas_file = tmp
            .path()
            .join("client/runtime/local/state/canvas/documents/cv_demo/collection.media/tiny.png");
        fs::create_dir_all(canvas_file.parent().expect("parent")).expect("canvas dir");
        fs::write(&canvas_file, tiny_png_bytes()).expect("png");
        let out = api_media(
            tmp.path(),
            &json!({
                "path": "/canvas/documents/cv_demo/collection.media/tiny.png?cache=1"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("source_kind").and_then(Value::as_str), Some("local"));
        assert_eq!(out.get("file_name").and_then(Value::as_str), Some("tiny.png"));
    }

    #[test]
    fn openclaw_media_rejects_canvas_document_traversal_paths() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_media(
            tmp.path(),
            &json!({
                "path": "/canvas/documents/../collection.media/index.html"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("invalid-path")
        );
    }

    #[test]
    fn openclaw_media_status_and_providers_report_contract_and_tool_catalog() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let status = api_status(tmp.path());
        let providers = api_providers(tmp.path());
        assert_eq!(
            status
                .pointer("/media_request_contract/managed_canvas_media_prefix")
                .and_then(Value::as_str),
            Some("/canvas/documents/")
        );
        let suffixes = status
            .pointer("/media_request_contract/default_local_root_suffixes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(suffixes.iter().any(|row| row.as_str() == Some("client/runtime/local/state/canvas")));
        assert_eq!(
            status
                .pointer("/media_request_contract/supports_wildcard_local_roots")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            status
                .pointer("/media_request_contract/local_root_pattern_contract/wildcard_segment")
                .and_then(Value::as_str),
            Some("*")
        );
        assert_eq!(status.pointer("/media_request_contract/workspace_dir_resolution_contract/precedence").and_then(Value::as_str), Some("top_level_over_media_access"));
        let policy_fields = status
            .pointer("/media_request_contract/host_read_policy_contract/deny_policy_fields")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(policy_fields.iter().any(|row| row.as_str() == Some("sender_tool_policy")));
        let supported_channels = status
            .pointer("/media_request_contract/channel_attachment_root_contract/supported_channels")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(supported_channels.iter().any(|row| row.as_str() == Some("imessage")));
        let imessage_roots = status
            .pointer("/media_request_contract/channel_attachment_root_contract/channels/imessage/default_attachment_roots")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(imessage_roots.iter().any(|row| row.as_str() == Some("/Users/*/Library/Messages/Attachments")));
        let error_codes = status
            .pointer("/media_request_contract/fail_closed_error_codes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(error_codes.iter().any(|row| row.as_str() == Some("invalid-root")));
        assert!(providers
            .get("tool_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.get("tool").and_then(Value::as_str) == Some("web_media")))
            .unwrap_or(false));
    }
}
