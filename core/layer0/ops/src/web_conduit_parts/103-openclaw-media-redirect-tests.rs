#[cfg(test)]
mod openclaw_media_redirect_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn create_png_buffer_with_dimensions(width: u32, height: u32) -> Vec<u8> {
        let mut ihdr_data = vec![0u8; 13];
        ihdr_data[0..4].copy_from_slice(&width.to_be_bytes());
        ihdr_data[4..8].copy_from_slice(&height.to_be_bytes());
        ihdr_data[8] = 8;
        ihdr_data[9] = 6;
        [
            vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
            vec![0x00, 0x00, 0x00, 0x0d],
            b"IHDR".to_vec(),
            ihdr_data,
            vec![0x00, 0x00, 0x00, 0x00],
            vec![0x00, 0x00, 0x00, 0x00, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82],
        ]
        .concat()
    }

    fn create_jpeg_buffer_with_dimensions(width: u16, height: u16) -> Vec<u8> {
        [
            vec![0xff, 0xd8],
            vec![
                0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00,
                0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
            ],
            vec![
                0xff,
                0xc0,
                0x00,
                0x11,
                0x08,
                (height >> 8) as u8,
                (height & 0xff) as u8,
                (width >> 8) as u8,
                (width & 0xff) as u8,
                0x03,
                0x01,
                0x11,
                0x00,
                0x02,
                0x11,
                0x00,
                0x03,
                0x11,
                0x00,
            ],
            vec![0xff, 0xda, 0x00, 0x0c, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3f, 0x00],
            vec![0xff, 0xd9],
        ]
        .concat()
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

    fn spawn_single_response_server(response: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            if let Ok((mut socket, _)) = listener.accept() {
                let mut buffer = [0u8; 4096];
                let _ = socket.read(&mut buffer);
                let _ = socket.write_all(&response);
                let _ = socket.flush();
            }
        });
        format!("http://{}", addr)
    }

    #[test]
    fn openclaw_media_redirect_follows_remote_png_and_keeps_detected_extension() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_test_policy(tmp.path());
        let png = create_png_buffer_with_dimensions(2, 1);
        let final_listener = TcpListener::bind("127.0.0.1:0").expect("bind final");
        let final_addr = final_listener.local_addr().expect("final addr");
        thread::spawn(move || {
            if let Ok((mut socket, _)) = final_listener.accept() {
                let mut buffer = [0u8; 4096];
                let _ = socket.read(&mut buffer);
                let response = [
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: "
                        .as_slice(),
                    png.len().to_string().as_bytes(),
                    b"\r\nConnection: close\r\n\r\n".as_slice(),
                    png.as_slice(),
                ]
                .concat();
                let _ = socket.write_all(&response);
                let _ = socket.flush();
            }
        });
        let redirect_url = spawn_single_response_server(
            format!(
                "HTTP/1.1 302 Found\r\nLocation: http://{}/final.png\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                final_addr
            )
            .into_bytes(),
        );
        let out = api_media(tmp.path(), &json!({"url": redirect_url, "summary_only": true}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("redirect_count").and_then(Value::as_u64), Some(1));
        assert_eq!(out.get("file_name").and_then(Value::as_str), Some("final.png"));
        assert_eq!(out.get("content_type").and_then(Value::as_str), Some("image/png"));
    }

    #[test]
    fn openclaw_media_redirect_strips_sensitive_headers_across_origins() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_test_policy(tmp.path());
        let final_request = Arc::new(Mutex::new(String::new()));
        let final_request_reader = Arc::clone(&final_request);
        let final_listener = TcpListener::bind("127.0.0.1:0").expect("bind final");
        let final_addr = final_listener.local_addr().expect("final addr");
        thread::spawn(move || {
            if let Ok((mut socket, _)) = final_listener.accept() {
                let mut buffer = [0u8; 4096];
                let read = socket.read(&mut buffer).unwrap_or(0);
                *final_request_reader.lock().expect("lock") =
                    String::from_utf8_lossy(&buffer[..read]).to_string();
                let response =
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 10\r\nConnection: close\r\n\r\nredirected"
                        .to_vec();
                let _ = socket.write_all(&response);
                let _ = socket.flush();
            }
        });
        let redirect_url = spawn_single_response_server(
            format!(
                "HTTP/1.1 302 Found\r\nLocation: http://{}/final.txt\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                final_addr
            )
            .into_bytes(),
        );
        let out = api_media(
            tmp.path(),
            &json!({
                "url": redirect_url,
                "headers": {
                    "Authorization": "Bearer secret",
                    "Cookie": "session=abc",
                    "X-Api-Key": "custom-secret",
                    "Accept": "text/plain",
                    "User-Agent": "OpenClaw-Test/1.0"
                },
                "summary_only": true
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let captured = final_request.lock().expect("lock").to_ascii_lowercase();
        assert!(!captured.contains("authorization: bearer secret"));
        assert!(!captured.contains("cookie: session=abc"));
        assert!(!captured.contains("x-api-key: custom-secret"));
        assert!(captured.contains("accept: text/plain"));
        assert!(captured.contains("user-agent: openclaw-test/1.0"));
    }

    #[test]
    fn openclaw_media_redirect_keeps_headers_on_same_origin() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_test_policy(tmp.path());
        let second_request = Arc::new(Mutex::new(String::new()));
        let second_request_reader = Arc::clone(&second_request);
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            if let Ok((mut first, _)) = listener.accept() {
                let mut buffer = [0u8; 4096];
                let _ = first.read(&mut buffer);
                let response =
                    b"HTTP/1.1 302 Found\r\nLocation: /final.jpg\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = first.write_all(response);
                let _ = first.flush();
            }
            if let Ok((mut second, _)) = listener.accept() {
                let mut buffer = [0u8; 4096];
                let read = second.read(&mut buffer).unwrap_or(0);
                *second_request_reader.lock().expect("lock") =
                    String::from_utf8_lossy(&buffer[..read]).to_string();
                let jpeg = create_jpeg_buffer_with_dimensions(3, 2);
                let response = [
                    b"HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: "
                        .as_slice(),
                    jpeg.len().to_string().as_bytes(),
                    b"\r\nConnection: close\r\n\r\n".as_slice(),
                    jpeg.as_slice(),
                ]
                .concat();
                let _ = second.write_all(&response);
                let _ = second.flush();
            }
        });
        let out = api_media(
            tmp.path(),
            &json!({
                "url": format!("http://{addr}/start"),
                "headers": {"Authorization": "Bearer secret"},
                "summary_only": true
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("file_name").and_then(Value::as_str), Some("final.jpg"));
        let captured = second_request.lock().expect("lock").to_ascii_lowercase();
        assert!(captured.contains("authorization: bearer secret"));
    }

    #[test]
    fn openclaw_media_redirect_rejects_missing_location_header() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_test_policy(tmp.path());
        let out = api_media(
            tmp.path(),
            &json!({
                "url": spawn_single_response_server(
                    b"HTTP/1.1 302 Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
                )
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("error").and_then(Value::as_str), Some("invalid_redirect"));
        assert_eq!(out.get("reason").and_then(Value::as_str), Some("redirect_missing_location"));
    }

    #[test]
    fn openclaw_media_contract_reports_remote_redirect_and_header_policy() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let status = api_status(tmp.path());
        assert_eq!(
            status
                .pointer("/media_request_contract/remote_redirect_contract/max_redirects")
                .and_then(Value::as_u64),
            Some(5)
        );
        assert!(
            status
                .pointer("/media_request_contract/remote_request_headers_contract/cross_origin_retained_headers")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str() == Some("accept")))
                .unwrap_or(false)
        );
    }
}
