const MEDIA_FFMPEG_MAX_BUFFER_BYTES: usize = 10 * 1024 * 1024;
const MEDIA_FFPROBE_TIMEOUT_MS: u64 = 10_000;
const MEDIA_FFMPEG_TIMEOUT_MS: u64 = 45_000;
const MEDIA_FFMPEG_MAX_AUDIO_DURATION_SECS: u64 = 20 * 60;
const DEFAULT_PDF_EXTRACT_MAX_PAGES: usize = 5;
const DEFAULT_PDF_MIN_TEXT_CHARS: usize = 80;
const DEFAULT_PDF_MAX_PIXELS: u64 = 1_500_000;

fn resolve_trusted_media_exec_bin(name: &str) -> Option<PathBuf> {
    for prefix in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"] {
        let candidate = Path::new(prefix).join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn media_exec_install_hint(name: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("{name} not found in trusted system directories; install ffmpeg via brew install ffmpeg")
    } else {
        format!("{name} not found in trusted system directories; install ffmpeg via your system package manager")
    }
}

fn with_openclaw_media_temp_file<T, F>(
    stem: &str,
    suffix: &str,
    bytes: &[u8],
    callback: F,
) -> Result<T, String>
where
    F: FnOnce(&Path) -> Result<T, String>,
{
    let root = preferred_openclaw_tmp_dir();
    ensure_openclaw_tmp_root(&root)?;
    let path = root.join(format!(
        "{stem}-{}-{}{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or(0),
        suffix
    ));
    fs::write(&path, bytes).map_err(|err| format!("write_temp_media_failed:{err}"))?;
    let result = callback(&path);
    let _ = fs::remove_file(&path);
    result
}

fn run_ffprobe_capture(args: &[&str], timeout_ms: u64, max_buffer_bytes: usize) -> Result<String, String> {
    let Some(bin) = resolve_trusted_media_exec_bin("ffprobe") else {
        return Err(media_exec_install_hint("ffprobe"));
    };
    let output = Command::new(bin)
        .args(args)
        .output()
        .map_err(|err| format!("ffprobe_spawn_failed:{err}"))?;
    if !output.status.success() {
        let snippet = clean_text(&String::from_utf8_lossy(&output.stderr), 240);
        return Err(format!("ffprobe_failed:{snippet}"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.len() > max_buffer_bytes {
        return Err("ffprobe_max_buffer".to_string());
    }
    let _ = timeout_ms;
    Ok(stdout)
}

fn parse_ffprobe_csv_fields(stdout: &str, max_fields: usize) -> Vec<String> {
    stdout
        .trim()
        .split(|ch| matches!(ch, ',' | '\r' | '\n'))
        .filter_map(|field| {
            let cleaned = clean_text(field, 120).to_ascii_lowercase();
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            }
        })
        .take(max_fields)
        .collect()
}

fn parse_ffprobe_codec_and_sample_rate(stdout: &str) -> (Option<String>, Option<u64>) {
    let fields = parse_ffprobe_csv_fields(stdout, 2);
    let codec = fields.first().cloned().filter(|row| !row.is_empty());
    let sample_rate_hz = fields
        .get(1)
        .and_then(|row| row.parse::<u64>().ok())
        .filter(|row| *row > 0);
    (codec, sample_rate_hz)
}

fn parse_ffprobe_duration_secs(stdout: &str) -> Option<f64> {
    clean_text(stdout, 80)
        .split(|ch| matches!(ch, '\r' | '\n' | ','))
        .find_map(|row| row.trim().parse::<f64>().ok())
        .filter(|row| row.is_finite() && *row >= 0.0)
}

fn web_media_audio_probe_contract() -> Value {
    json!({
        "backend": "ffprobe",
        "trusted_bin_dirs": ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"],
        "timeout_ms": MEDIA_FFPROBE_TIMEOUT_MS,
        "max_buffer_bytes": MEDIA_FFMPEG_MAX_BUFFER_BYTES,
        "max_audio_duration_secs": MEDIA_FFMPEG_MAX_AUDIO_DURATION_SECS,
        "supported_kinds": ["audio", "video"],
        "returns": ["codec", "sample_rate_hz", "duration_secs", "duration_within_limit"]
    })
}

fn web_media_pdf_extract_contract() -> Value {
    json!({
        "max_pages_default": DEFAULT_PDF_EXTRACT_MAX_PAGES,
        "min_text_chars_default": DEFAULT_PDF_MIN_TEXT_CHARS,
        "max_pixels_default": DEFAULT_PDF_MAX_PIXELS,
        "page_numbers_fields": ["page_numbers", "pageNumbers"],
        "extract_images_supported": false,
        "image_extraction_backend": "unavailable",
        "returns": ["text", "text_chars", "page_numbers", "page_count", "images"]
    })
}

fn append_openclaw_pdf_tool_entries(tool_catalog: &mut Value, policy: &Value) {
    if let Some(rows) = tool_catalog.as_array_mut() {
        rows.push(json!({
            "tool": "web_media_audio_probe",
            "label": "Web Media Audio Probe",
            "family": "media",
            "enabled": true,
            "request_contract": web_media_audio_probe_contract()
        }));
        rows.push(json!({
            "tool": "web_media_pdf_extract",
            "label": "Web Media PDF Extract",
            "family": "media",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "request_contract": web_media_pdf_extract_contract()
        }));
    }
    append_pdf_native_provider_tool_entry(tool_catalog, policy);
    append_web_media_pdf_tool_entry(tool_catalog, policy);
}

fn parse_pdf_page_numbers(request: &Value, max_pages: usize) -> Vec<u32> {
    let mut rows = Vec::new();
    if let Some(values) = request.get("page_numbers").or_else(|| request.get("pageNumbers")) {
        if let Some(array) = values.as_array() {
            for value in array {
                if let Some(page) = value.as_u64().filter(|row| *row > 0) {
                    let page = page as u32;
                    if !rows.contains(&page) && rows.len() < max_pages {
                        rows.push(page);
                    }
                }
            }
        } else if let Some(raw) = values.as_str() {
            for token in raw.split(',') {
                if let Ok(page) = token.trim().parse::<u32>() {
                    if page > 0 && !rows.contains(&page) && rows.len() < max_pages {
                        rows.push(page);
                    }
                }
            }
        }
    }
    rows
}

fn load_media_binary_for_request(root: &Path, request: &Value) -> Result<LoadedMedia, Value> {
    let raw = media_request_source(request);
    if raw.is_empty() {
        return Err(json!({"ok": false, "error": "media_source_required"}));
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        fetch_remote_media_binary(root, request)
    } else if raw.starts_with("data:") {
        load_inline_media_binary(request)
    } else {
        load_local_media_binary(root, request)
    }
}

fn api_audio_probe(root: &Path, request: &Value) -> Value {
    let requested_source = media_request_source(request);
    let loaded = match load_media_binary_for_request(root, request) {
        Ok(row) => row,
        Err(mut err) => {
            err["type"] = json!("web_conduit_audio_probe");
            err["audio_probe_contract"] = web_media_audio_probe_contract();
            return err;
        }
    };
    if !matches!(loaded.kind.as_str(), "audio" | "video") {
        return json!({
            "ok": false,
            "type": "web_conduit_audio_probe",
            "error": "unsupported_media_kind",
            "requested_source": requested_source,
            "detected_kind": loaded.kind,
            "audio_probe_contract": web_media_audio_probe_contract()
        });
    }
    let result = with_openclaw_media_temp_file("openclaw-audio-probe", ".bin", &loaded.buffer, |path| {
        let path_string = path.to_string_lossy().to_string();
        let probe_args = [
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_name,sample_rate",
            "-of",
            "csv=p=0",
            path_string.as_str(),
        ];
        let duration_args = [
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            path_string.as_str(),
        ];
        let codec_stdout =
            run_ffprobe_capture(&probe_args, MEDIA_FFPROBE_TIMEOUT_MS, MEDIA_FFMPEG_MAX_BUFFER_BYTES)?;
        let duration_stdout =
            run_ffprobe_capture(&duration_args, MEDIA_FFPROBE_TIMEOUT_MS, MEDIA_FFMPEG_MAX_BUFFER_BYTES)?;
        let (codec, sample_rate_hz) = parse_ffprobe_codec_and_sample_rate(&codec_stdout);
        let duration_secs = parse_ffprobe_duration_secs(&duration_stdout);
        Ok(json!({
            "codec": codec,
            "sample_rate_hz": sample_rate_hz,
            "duration_secs": duration_secs,
            "duration_limit_secs": MEDIA_FFMPEG_MAX_AUDIO_DURATION_SECS,
            "duration_within_limit": duration_secs.map(|row| row <= MEDIA_FFMPEG_MAX_AUDIO_DURATION_SECS as f64),
            "backend": "ffprobe"
        }))
    });
    match result {
        Ok(probe) => json!({
            "ok": true,
            "type": "web_conduit_audio_probe",
            "requested_source": requested_source,
            "resolved_source": loaded.resolved_source,
            "content_type": loaded.content_type,
            "kind": loaded.kind,
            "file_name": loaded.file_name,
            "probe": probe,
            "audio_probe_contract": web_media_audio_probe_contract()
        }),
        Err(err) => json!({
            "ok": false,
            "type": "web_conduit_audio_probe",
            "error": if err.contains("not found in trusted system directories") { "ffprobe_not_available" } else { "audio_probe_failed" },
            "reason": clean_text(&err, 240),
            "requested_source": requested_source,
            "resolved_source": loaded.resolved_source,
            "audio_probe_contract": web_media_audio_probe_contract()
        }),
    }
}

fn api_pdf_extract(root: &Path, request: &Value) -> Value {
    let requested_source = media_request_source(request);
    let summary_only = request.get("summary_only").and_then(Value::as_bool).unwrap_or(false);
    let extract_images = request.get("extract_images").and_then(Value::as_bool).unwrap_or(false);
    let min_text_chars = parse_fetch_u64(request.get("min_text_chars"), DEFAULT_PDF_MIN_TEXT_CHARS as u64, 0, 20_000) as usize;
    let max_pages = parse_fetch_u64(request.get("max_pages"), DEFAULT_PDF_EXTRACT_MAX_PAGES as u64, 1, 32) as usize;
    let loaded = match load_media_binary_for_request(root, request) {
        Ok(row) => row,
        Err(mut err) => {
            err["type"] = json!("web_conduit_pdf_extract");
            err["pdf_extract_contract"] = web_media_pdf_extract_contract();
            return err;
        }
    };
    if normalize_media_content_type(&loaded.content_type) != "application/pdf" {
        return json!({
            "ok": false,
            "type": "web_conduit_pdf_extract",
            "error": "unsupported_content_type",
            "requested_source": requested_source,
            "resolved_source": loaded.resolved_source,
            "content_type": loaded.content_type,
            "pdf_extract_contract": web_media_pdf_extract_contract()
        });
    }
    let doc = match lopdf::Document::load_mem(&loaded.buffer) {
        Ok(row) => row,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "web_conduit_pdf_extract",
                "error": "pdf_parse_failed",
                "reason": clean_text(&err.to_string(), 240),
                "requested_source": requested_source,
                "resolved_source": loaded.resolved_source,
                "pdf_extract_contract": web_media_pdf_extract_contract()
            });
        }
    };
    let pages = doc.get_pages();
    let page_count = pages.len();
    let page_numbers = {
        let mut selected = parse_pdf_page_numbers(request, max_pages);
        if selected.is_empty() {
            selected.extend(pages.keys().take(max_pages).copied());
        }
        selected
    };
    let text_chunks = doc.extract_text_chunks(&page_numbers);
    let mut extracted_text = String::new();
    let mut extraction_errors = Vec::new();
    for chunk in text_chunks {
        match chunk {
            Ok(text) if !text.trim().is_empty() => {
                extracted_text.push_str(&text);
                if !text.ends_with('\n') {
                    extracted_text.push('\n');
                }
            }
            Ok(_) => {}
            Err(err) => extraction_errors.push(clean_text(&err.to_string(), 200)),
        }
    }
    let normalized_text = normalize_block_text(&extracted_text);
    let text_chars = normalized_text.chars().count();
    let response_text = if summary_only {
        normalized_text.chars().take(600).collect::<String>()
    } else {
        normalized_text.clone()
    };
    json!({
        "ok": true,
        "type": "web_conduit_pdf_extract",
        "requested_source": requested_source,
        "resolved_source": loaded.resolved_source,
        "source_kind": loaded.source_kind,
        "file_name": loaded.file_name,
        "content_type": loaded.content_type,
        "page_count": page_count,
        "page_numbers": page_numbers,
        "text": response_text,
        "text_chars": text_chars,
        "text_sufficient": text_chars >= min_text_chars,
        "extract_images": extract_images,
        "image_extraction_backend": "unavailable",
        "images": [],
        "extraction_errors": extraction_errors,
        "summary": format!("Extracted {} characters from {} PDF page(s).", text_chars, page_numbers.len()),
        "pdf_extract_contract": web_media_pdf_extract_contract()
    })
}
