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
        assert!(providers
            .get("tool_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.get("tool").and_then(Value::as_str) == Some("web_media")))
            .unwrap_or(false));
    }
}
