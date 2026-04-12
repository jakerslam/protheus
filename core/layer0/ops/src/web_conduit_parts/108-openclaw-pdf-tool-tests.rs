#[cfg(test)]
mod openclaw_pdf_tool_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn build_simple_pdf_bytes(text: &str) -> Vec<u8> {
        let escaped = text
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)");
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
            "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"
                .to_string(),
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
                *capture.lock().expect("lock") =
                    String::from_utf8_lossy(&buffer[..read]).to_string();
                let body = response.as_bytes();
                let reply = [
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: "
                        .as_slice(),
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
    fn openclaw_pdf_tool_contract_reports_surface_and_defaults() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let status = api_status(tmp.path());
        assert_eq!(
            status
                .pointer("/media_request_contract/pdf_tool_contract/max_pdfs")
                .and_then(Value::as_u64),
            Some(PDF_TOOL_MAX_PDFS as u64)
        );
        assert!(status
            .pointer("/tool_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.get("tool").and_then(Value::as_str) == Some("web_media_pdf_tool")
            }))
            .unwrap_or(false));
    }

    #[test]
    fn openclaw_pdf_tool_inputs_dedupe_pdf_and_pdfs_entries() {
        let inputs = resolve_pdf_tool_inputs(&json!({
            "pdf": " /tmp/a.pdf ",
            "pdfs": ["/tmp/a.pdf", "  ", "/tmp/b.pdf"]
        }))
        .expect("inputs");
        assert_eq!(inputs, vec!["/tmp/a.pdf", "/tmp/b.pdf"]);
    }

    #[test]
    fn openclaw_pdf_tool_page_range_parses_and_clamps() {
        assert_eq!(
            parse_pdf_tool_page_range("1,3,5-9", 6).expect("page range"),
            vec![1, 3, 5, 6]
        );
        assert!(parse_pdf_tool_page_range("5-3", 6).is_err());
        assert!(parse_pdf_tool_page_range("0", 6).is_err());
    }

    #[test]
    fn openclaw_pdf_tool_model_plan_prefers_native_provider_with_auth() {
        std::env::set_var("ANTHROPIC_API_KEY", "anthropic-test");
        std::env::set_var("OPENAI_API_KEY", "openai-test");
        let plan = resolve_pdf_tool_model_plan(&json!({}));
        assert_eq!(
            plan.get("primary").and_then(Value::as_str),
            Some(PDF_TOOL_DEFAULT_ANTHROPIC_MODEL)
        );
        assert_eq!(plan.get("native_supported").and_then(Value::as_bool), Some(true));
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    fn openclaw_pdf_tool_native_provider_rejects_page_ranges() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let pdf = tmp.path().join("native.pdf");
        write_pdf(&pdf, "Native PDF");
        let out = api_pdf_tool(
            tmp.path(),
            &json!({
                "path": pdf.to_string_lossy(),
                "prompt": "Summarize",
                "model": PDF_TOOL_DEFAULT_ANTHROPIC_MODEL,
                "api_key": "test-key",
                "pages": "1-2"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("pages_not_supported_with_native_provider")
        );
    }

    #[test]
    fn openclaw_pdf_tool_extraction_fallback_returns_combined_text() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let pdf = tmp.path().join("fallback.pdf");
        write_pdf(&pdf, "Fallback PDF");
        let out = api_pdf_tool(
            tmp.path(),
            &json!({
                "path": pdf.to_string_lossy(),
                "prompt": "Summarize",
                "model": PDF_TOOL_DEFAULT_OPENAI_MODEL,
                "api_key": "test-key"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("native").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("analysis_mode").and_then(Value::as_str),
            Some("extraction_only_fallback")
        );
        assert!(out
            .get("analysis")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Fallback PDF"));
    }

    #[test]
    fn openclaw_pdf_tool_native_provider_wraps_analysis() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let pdf = tmp.path().join("native-ok.pdf");
        write_pdf(&pdf, "Native OK");
        let capture = Arc::new(Mutex::new(String::new()));
        let server = spawn_json_server(
            Arc::clone(&capture),
            r#"{"content":[{"type":"text","text":"Native tool summary"}]}"#,
        );
        let out = api_pdf_tool(
            tmp.path(),
            &json!({
                "path": pdf.to_string_lossy(),
                "prompt": "Summarize",
                "model": PDF_TOOL_DEFAULT_ANTHROPIC_MODEL,
                "api_key": "test-key",
                "base_url": server
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("native").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("analysis").and_then(Value::as_str),
            Some("Native tool summary")
        );
        let raw = capture.lock().expect("lock").clone();
        assert!(raw.contains("POST /v1/messages"));
    }
}
