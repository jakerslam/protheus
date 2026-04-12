#[cfg(test)]
mod openclaw_pdf_ffmpeg_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn build_simple_pdf_bytes(text: &str) -> Vec<u8> {
        let escaped = text.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
        let stream = format!("BT\n/F1 24 Tf\n72 72 Td\n({escaped}) Tj\nET\n");
        let objects = vec![
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_string(),
            "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_string(),
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n".to_string(),
            format!(
                "4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
                stream.as_bytes().len(),
                stream
            ),
            "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n".to_string(),
        ];
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = vec![0usize];
        for object in objects {
            offsets.push(pdf.len());
            pdf.extend_from_slice(object.as_bytes());
        }
        let xref_start = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets.iter().skip(1) {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                offsets.len(),
                xref_start
            )
            .as_bytes(),
        );
        pdf
    }

    fn write_pdf(path: &Path, text: &str) {
        fs::write(path, build_simple_pdf_bytes(text)).expect("write pdf");
    }

    fn spawn_json_server(capture: Arc<Mutex<String>>, response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            if let Ok((mut socket, _)) = listener.accept() {
                let mut buffer = [0u8; 32768];
                let read = socket.read(&mut buffer).unwrap_or(0);
                *capture.lock().expect("lock") = String::from_utf8_lossy(&buffer[..read]).to_string();
                let body = response.as_bytes();
                let reply = [
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: ".as_slice(),
                    body.len().to_string().as_bytes(),
                    b"\r\nConnection: close\r\n\r\n".as_slice(),
                    body,
                ]
                .concat();
                let _ = socket.write_all(&reply);
                let _ = socket.flush();
            }
        });
        format!("http://{}", addr)
    }

    #[test]
    fn openclaw_pdf_ffmpeg_contract_reports_new_tool_surfaces() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let status = api_status(tmp.path());
        assert_eq!(
            status
                .pointer("/media_request_contract/audio_probe_contract/backend")
                .and_then(Value::as_str),
            Some("ffprobe")
        );
        assert_eq!(
            status
                .pointer("/media_request_contract/pdf_extract_contract/max_pages_default")
                .and_then(Value::as_u64),
            Some(DEFAULT_PDF_EXTRACT_MAX_PAGES as u64)
        );
        assert!(status
            .pointer("/tool_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                matches!(
                    row.get("tool").and_then(Value::as_str),
                    Some("web_media_audio_probe")
                        | Some("web_media_pdf_extract")
                        | Some("web_media_pdf_native_provider")
                )
            }))
            .unwrap_or(false));
    }

    #[test]
    fn openclaw_pdf_extract_returns_text_from_minimal_pdf() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let pdf = tmp.path().join("hello.pdf");
        write_pdf(&pdf, "Hello OpenClaw PDF");
        let out = api_pdf_extract(tmp.path(), &json!({"path": pdf.to_string_lossy(), "summary_only": false}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(out
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Hello OpenClaw PDF"));
        assert_eq!(
            out.get("content_type").and_then(Value::as_str),
            Some("application/pdf")
        );
    }

    #[test]
    fn openclaw_ffprobe_csv_parsing_normalizes_codec_and_sample_rate() {
        let fields = parse_ffprobe_csv_fields("Opus,\n48000\n", 2);
        assert_eq!(fields, vec!["opus".to_string(), "48000".to_string()]);
        let (codec, sample_rate_hz) = parse_ffprobe_codec_and_sample_rate("Opus,48000\n");
        assert_eq!(codec.as_deref(), Some("opus"));
        assert_eq!(sample_rate_hz, Some(48_000));
    }

    #[test]
    fn openclaw_pdf_native_provider_analyze_sends_anthropic_document_blocks() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let pdf = tmp.path().join("anthropic.pdf");
        write_pdf(&pdf, "Anthropic PDF");
        let capture = Arc::new(Mutex::new(String::new()));
        let server = spawn_json_server(
            Arc::clone(&capture),
            r#"{"content":[{"type":"text","text":"Analysis of PDF"}]}"#,
        );
        let out = api_pdf_native_analyze(
            tmp.path(),
            &json!({
                "path": pdf.to_string_lossy(),
                "provider": "anthropic",
                "model_id": "claude-opus-4-1",
                "prompt": "Summarize this document",
                "api_key": "test-key",
                "base_url": server
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("analysis").and_then(Value::as_str), Some("Analysis of PDF"));
        let raw = capture.lock().expect("lock").clone();
        assert!(raw.contains("POST /v1/messages"));
        assert!(raw.contains("\"type\":\"document\""));
        assert!(raw.contains("\"media_type\":\"application/pdf\""));
    }

    #[test]
    fn openclaw_pdf_native_provider_analyze_normalizes_google_v1beta_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let pdf = tmp.path().join("gemini.pdf");
        write_pdf(&pdf, "Gemini PDF");
        let capture = Arc::new(Mutex::new(String::new()));
        let server = spawn_json_server(
            Arc::clone(&capture),
            r#"{"candidates":[{"content":{"parts":[{"text":"Gemini PDF analysis"}]}}]}"#,
        );
        let out = api_pdf_native_analyze(
            tmp.path(),
            &json!({
                "path": pdf.to_string_lossy(),
                "provider": "google",
                "model_id": "gemini-2.5-pro",
                "prompt": "Summarize this document",
                "api_key": "test-key",
                "base_url": format!("{server}/v1beta")
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("analysis").and_then(Value::as_str),
            Some("Gemini PDF analysis")
        );
        let raw = capture.lock().expect("lock").clone();
        assert!(raw.contains("POST /v1beta/models/gemini-2.5-pro:generateContent?key=test-key"));
        assert!(raw.contains("\"mime_type\":\"application/pdf\""));
    }
}
