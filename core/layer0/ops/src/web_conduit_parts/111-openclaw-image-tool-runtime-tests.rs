#[cfg(test)]
mod openclaw_image_tool_runtime_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    const ONE_PIXEL_PNG_B64: &str =
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/woAAn8B9FD5fHAAAAAASUVORK5CYII=";

    fn write_png(path: &Path) {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(ONE_PIXEL_PNG_B64)
            .expect("decode png");
        fs::write(path, bytes).expect("write png");
    }

    fn spawn_sequence_server(
        captures: Arc<Mutex<Vec<String>>>,
        responses: Vec<(u16, String)>,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            for (status, body) in responses {
                if let Ok((mut socket, _)) = listener.accept() {
                    let mut buffer = [0u8; 65536];
                    let read = socket.read(&mut buffer).unwrap_or(0);
                    captures
                        .lock()
                        .expect("lock")
                        .push(String::from_utf8_lossy(&buffer[..read]).to_string());
                    let status_line = if status == 200 {
                        "HTTP/1.1 200 OK"
                    } else {
                        "HTTP/1.1 500 Internal Server Error"
                    };
                    let reply = format!(
                        "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.as_bytes().len(),
                        body
                    );
                    let _ = socket.write_all(reply.as_bytes());
                    let _ = socket.flush();
                }
            }
        });
        format!("http://{}", addr)
    }

    #[test]
    fn openclaw_image_tool_executes_openai_compatible_request_with_inline_images() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let image = tmp.path().join("sample.png");
        write_png(&image);
        let captures = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_sequence_server(
            Arc::clone(&captures),
            vec![
                (200, r#"{"data":[{"id":"gpt-4o"}]}"#.to_string()),
                (
                    200,
                    r#"{"choices":[{"message":{"content":"openai image ok"}}]}"#.to_string(),
                ),
            ],
        );
        crate::dashboard_provider_runtime::save_provider_key(tmp.path(), "openai", "sk-test-openai");
        let set_url = crate::dashboard_provider_runtime::set_provider_url(tmp.path(), "openai", &server);
        assert_eq!(set_url.get("ok").and_then(Value::as_bool), Some(true));

        let out = api_image_tool(
            tmp.path(),
            &json!({
                "path": image.to_string_lossy(),
                "local_roots": tmp.path().to_string_lossy(),
                "provider": "openai",
                "prompt": "Describe the image."
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("provider").and_then(Value::as_str), Some("openai"));
        assert_eq!(
            out.get("analysis").and_then(Value::as_str),
            Some("openai image ok")
        );
        let raw = captures.lock().expect("lock").join("\n---\n");
        assert!(raw.contains("POST /chat/completions"));
        assert!(raw.contains("\"type\":\"image_url\""));
        assert!(raw.contains("data:image/png;base64,"));
    }

    #[test]
    fn openclaw_image_tool_executes_google_generate_content_with_inline_data() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let image = tmp.path().join("sample.png");
        write_png(&image);
        let captures = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_sequence_server(
            Arc::clone(&captures),
            vec![
                (
                    200,
                    r#"{"models":[{"name":"models/gemini-2.5-flash"}]}"#.to_string(),
                ),
                (
                    200,
                    r#"{"candidates":[{"content":{"parts":[{"text":"google image ok"}]}}]}"#
                        .to_string(),
                ),
            ],
        );
        crate::dashboard_provider_runtime::save_provider_key(tmp.path(), "google", "google-test-key");
        let set_url = crate::dashboard_provider_runtime::set_provider_url(tmp.path(), "google", &server);
        assert_eq!(set_url.get("ok").and_then(Value::as_bool), Some(true));

        let out = api_image_tool(
            tmp.path(),
            &json!({
                "path": image.to_string_lossy(),
                "local_roots": tmp.path().to_string_lossy(),
                "provider": "google",
                "model": "gemini-2.5-flash",
                "prompt": "Describe the image."
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("provider").and_then(Value::as_str), Some("google"));
        assert_eq!(
            out.get("analysis").and_then(Value::as_str),
            Some("google image ok")
        );
        let raw = captures.lock().expect("lock").join("\n---\n");
        assert!(raw.contains("POST /v1beta/models/gemini-2.5-flash:generateContent?key=google-test-key"));
        assert!(raw.contains("\"inline_data\""));
        assert!(raw.contains("\"mime_type\":\"image/png\""));
    }

    #[test]
    fn openclaw_image_tool_falls_back_to_next_ready_provider_after_failure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let image = tmp.path().join("sample.png");
        write_png(&image);

        let openai_captures = Arc::new(Mutex::new(Vec::<String>::new()));
        let openai_server = spawn_sequence_server(
            Arc::clone(&openai_captures),
            vec![
                (200, r#"{"data":[{"id":"gpt-4o"}]}"#.to_string()),
                (
                    500,
                    r#"{"error":{"message":"openai image backend unavailable"}}"#.to_string(),
                ),
            ],
        );
        let google_captures = Arc::new(Mutex::new(Vec::<String>::new()));
        let google_server = spawn_sequence_server(
            Arc::clone(&google_captures),
            vec![
                (
                    200,
                    r#"{"models":[{"name":"models/gemini-2.5-flash"}]}"#.to_string(),
                ),
                (
                    200,
                    r#"{"candidates":[{"content":{"parts":[{"text":"google fallback ok"}]}}]}"#
                        .to_string(),
                ),
            ],
        );

        crate::dashboard_provider_runtime::save_provider_key(tmp.path(), "openai", "sk-test-openai");
        crate::dashboard_provider_runtime::save_provider_key(tmp.path(), "google", "google-test-key");
        assert_eq!(
            crate::dashboard_provider_runtime::set_provider_url(tmp.path(), "openai", &openai_server)
                .get("ok")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            crate::dashboard_provider_runtime::set_provider_url(tmp.path(), "google", &google_server)
                .get("ok")
                .and_then(Value::as_bool),
            Some(true)
        );

        let out = api_image_tool(
            tmp.path(),
            &json!({
                "path": image.to_string_lossy(),
                "local_roots": tmp.path().to_string_lossy(),
                "prompt": "Describe the image."
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("provider").and_then(Value::as_str), Some("google"));
        assert_eq!(
            out.get("analysis").and_then(Value::as_str),
            Some("google fallback ok")
        );
        assert_eq!(
            out.pointer("/attempts/0/provider").and_then(Value::as_str),
            Some("openai")
        );
        assert_eq!(
            out.pointer("/attempts/0/ok").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/attempts/1/provider").and_then(Value::as_str),
            Some("google")
        );
        assert_eq!(
            out.pointer("/attempts/1/ok").and_then(Value::as_bool),
            Some(true)
        );
        assert!(openai_captures
            .lock()
            .expect("lock")
            .join("\n")
            .contains("POST /chat/completions"));
        assert!(google_captures
            .lock()
            .expect("lock")
            .join("\n")
            .contains(":generateContent?key=google-test-key"));
    }
}
