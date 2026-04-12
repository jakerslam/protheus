const WEB_MEDIA_OPTIMIZE_JPEG_SIDES: &[u64] = &[2048, 1536, 1280, 1024, 800];
const WEB_MEDIA_OPTIMIZE_JPEG_QUALITY_STEPS: &[u64] = &[80, 70, 60, 50, 40];
const WEB_MEDIA_OPTIMIZE_PNG_COMPRESSION_STEPS: &[u64] = &[6, 7, 8, 9];

#[derive(Debug)]
struct FinalizedMedia {
    loaded: LoadedMedia,
    effective_max_bytes: usize,
    original_bytes: usize,
    optimize_images: bool,
    optimized: bool,
    optimization: Value,
}

#[derive(Debug)]
struct MediaOptimizationAttempt {
    buffer: Vec<u8>,
    format: &'static str,
    resize_side: u64,
    quality: Option<u64>,
    compression_level: Option<u64>,
    converted_from_heic: bool,
    preserved_alpha: bool,
}

fn media_requested_max_bytes(request: &Value) -> Option<usize> {
    request
        .get("max_bytes")
        .and_then(Value::as_u64)
        .map(|row| row.clamp(256, MAX_DOCUMENT_BYTES as u64) as usize)
}

fn media_request_optimize_images(request: &Value) -> bool {
    let raw = request.get("raw").and_then(Value::as_bool).unwrap_or(false);
    if raw {
        return false;
    }
    request
        .get("optimize_images")
        .or_else(|| request.get("optimizeImages"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn media_prefetch_max_bytes(request: &Value) -> usize {
    match media_requested_max_bytes(request) {
        Some(row) if media_request_optimize_images(request) => row.max(MAX_DOCUMENT_BYTES),
        Some(row) => row,
        None => MAX_DOCUMENT_BYTES,
    }
}

fn media_effective_output_max_bytes(request: &Value, kind: &str) -> usize {
    media_requested_max_bytes(request).unwrap_or_else(|| max_bytes_for_media_kind(kind))
}

fn media_format_mb(bytes: usize, digits: usize) -> String {
    format!("{:.*}", digits, bytes as f64 / (1024f64 * 1024f64))
}

fn media_format_cap_limit(label: &str, cap: usize, size: usize) -> String {
    format!(
        "{label} exceeds {}MB limit (got {}MB)",
        media_format_mb(cap, 0),
        media_format_mb(size, 2)
    )
}

fn media_format_cap_reduce(label: &str, cap: usize, size: usize) -> String {
    format!(
        "{label} could not be reduced below {}MB (got {}MB)",
        media_format_mb(cap, 0),
        media_format_mb(size, 2)
    )
}

fn media_is_heic_source(content_type: &str, file_name: &str) -> bool {
    let normalized = normalize_media_content_type(content_type);
    normalized == "image/heic"
        || normalized == "image/heif"
        || file_name.to_ascii_lowercase().ends_with(".heic")
        || file_name.to_ascii_lowercase().ends_with(".heif")
}

fn media_is_png_source(content_type: &str, file_name: &str) -> bool {
    normalize_media_content_type(content_type) == "image/png"
        || file_name.to_ascii_lowercase().ends_with(".png")
}

fn media_is_gif_source(content_type: &str) -> bool {
    normalize_media_content_type(content_type) == "image/gif"
}

fn media_to_jpeg_file_name(file_name: &str) -> String {
    if file_name.trim().is_empty() {
        return "image.jpg".to_string();
    }
    let parsed = Path::new(file_name);
    let stem = parsed
        .file_stem()
        .and_then(|row| row.to_str())
        .filter(|row| !row.trim().is_empty())
        .unwrap_or("image");
    match parsed.parent().filter(|row| !row.as_os_str().is_empty()) {
        Some(parent) => parent.join(format!("{stem}.jpg")).display().to_string(),
        None => format!("{stem}.jpg"),
    }
}

fn png_color_type(bytes: &[u8]) -> Option<u8> {
    if bytes.len() < 26 {
        return None;
    }
    if &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    Some(bytes[25])
}

fn png_has_alpha_channel(bytes: &[u8]) -> bool {
    matches!(png_color_type(bytes), Some(4 | 6))
}

fn read_jpeg_exif_orientation(bytes: &[u8]) -> Option<u16> {
    if bytes.len() < 4 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return None;
    }
    let mut offset = 2usize;
    while offset + 4 < bytes.len() {
        if bytes[offset] != 0xff {
            offset += 1;
            continue;
        }
        let marker = bytes[offset + 1];
        if marker == 0xff {
            offset += 1;
            continue;
        }
        if marker == 0xe1 {
            let exif_start = offset + 4;
            if bytes.len() <= exif_start + 6 || &bytes[exif_start..exif_start + 4] != b"Exif" {
                return None;
            }
            let tiff_start = exif_start + 6;
            if bytes.len() < tiff_start + 8 {
                return None;
            }
            let byte_order = &bytes[tiff_start..tiff_start + 2];
            let little_endian = byte_order == b"II";
            let read_u16 = |pos: usize| -> Option<u16> {
                let slice = bytes.get(pos..pos + 2)?;
                Some(if little_endian {
                    u16::from_le_bytes(slice.try_into().ok()?)
                } else {
                    u16::from_be_bytes(slice.try_into().ok()?)
                })
            };
            let read_u32 = |pos: usize| -> Option<u32> {
                let slice = bytes.get(pos..pos + 4)?;
                Some(if little_endian {
                    u32::from_le_bytes(slice.try_into().ok()?)
                } else {
                    u32::from_be_bytes(slice.try_into().ok()?)
                })
            };
            let ifd0_start = tiff_start + read_u32(tiff_start + 4)? as usize;
            let num_entries = read_u16(ifd0_start)? as usize;
            for idx in 0..num_entries {
                let entry = ifd0_start + 2 + idx * 12;
                let tag = read_u16(entry)?;
                if tag == 0x0112 {
                    let value = read_u16(entry + 8)?;
                    if (1..=8).contains(&value) {
                        return Some(value);
                    }
                    return None;
                }
            }
            return None;
        }
        if (0xe0..=0xef).contains(&marker) {
            let segment_len = u16::from_be_bytes(bytes.get(offset + 2..offset + 4)?.try_into().ok()?) as usize;
            offset += 2 + segment_len;
            continue;
        }
        if marker == 0xc0 || marker == 0xda {
            break;
        }
        offset += 1;
    }
    None
}

fn sips_apply_orientation(bytes: &[u8], orientation: u16) -> Result<Vec<u8>, String> {
    let ops: Vec<String> = match orientation {
        2 => vec!["-f".to_string(), "horizontal".to_string()],
        3 => vec!["-r".to_string(), "180".to_string()],
        4 => vec!["-f".to_string(), "vertical".to_string()],
        5 => vec![
            "-r".to_string(),
            "270".to_string(),
            "-f".to_string(),
            "horizontal".to_string(),
        ],
        6 => vec!["-r".to_string(), "90".to_string()],
        7 => vec![
            "-r".to_string(),
            "90".to_string(),
            "-f".to_string(),
            "horizontal".to_string(),
        ],
        8 => vec!["-r".to_string(), "270".to_string()],
        _ => return Ok(bytes.to_vec()),
    };
    with_openclaw_image_temp_dir(|dir| {
        let input = dir.join("in.jpg");
        let output = dir.join("out.jpg");
        fs::write(&input, bytes).map_err(|err| format!("write_orientation_input_failed:{err}"))?;
        let mut args = ops;
        args.push(input.display().to_string());
        args.push("--out".to_string());
        args.push(output.display().to_string());
        run_sips_capture(&args)?;
        fs::read(&output).map_err(|err| format!("read_orientation_output_failed:{err}"))
    })
}

fn normalize_exif_orientation_for_sips(bytes: &[u8]) -> Vec<u8> {
    if !prefers_sips_backend() {
        return bytes.to_vec();
    }
    let Some(orientation) = read_jpeg_exif_orientation(bytes) else {
        return bytes.to_vec();
    };
    if orientation == 1 {
        return bytes.to_vec();
    }
    sips_apply_orientation(bytes, orientation).unwrap_or_else(|_| bytes.to_vec())
}

fn resize_image_to_png(
    bytes: &[u8],
    max_side: u64,
    _compression_level: u64,
    without_enlargement: bool,
) -> Result<Vec<u8>, String> {
    assert_image_pixel_limit(bytes)?;
    if !prefers_sips_backend() {
        return Err("image_png_backend_unavailable".to_string());
    }
    with_openclaw_image_temp_dir(|dir| {
        let input = dir.join("in.png");
        let output = dir.join("out.png");
        fs::write(&input, bytes).map_err(|err| format!("write_png_input_failed:{err}"))?;
        let effective_side = if without_enlargement {
            if let Some(meta) = get_image_metadata(bytes) {
                max_side.min(meta.width.max(meta.height)).max(1)
            } else {
                max_side.max(1)
            }
        } else {
            max_side.max(1)
        };
        run_sips_capture(&[
            "-Z".to_string(),
            effective_side.to_string(),
            "-s".to_string(),
            "format".to_string(),
            "png".to_string(),
            input.display().to_string(),
            "--out".to_string(),
            output.display().to_string(),
        ])?;
        fs::read(&output).map_err(|err| format!("read_png_output_failed:{err}"))
    })
}

fn convert_heic_to_jpeg(bytes: &[u8]) -> Result<Vec<u8>, String> {
    assert_image_pixel_limit(bytes)?;
    if !prefers_sips_backend() {
        return Err("heic_conversion_backend_unavailable".to_string());
    }
    with_openclaw_image_temp_dir(|dir| {
        let input = dir.join("in.heic");
        let output = dir.join("out.jpg");
        fs::write(&input, bytes).map_err(|err| format!("write_heic_input_failed:{err}"))?;
        run_sips_capture(&[
            "-s".to_string(),
            "format".to_string(),
            "jpeg".to_string(),
            input.display().to_string(),
            "--out".to_string(),
            output.display().to_string(),
        ])?;
        fs::read(&output).map_err(|err| format!("read_heic_output_failed:{err}"))
    })
}

fn optimize_image_to_png(bytes: &[u8], max_bytes: usize) -> Result<MediaOptimizationAttempt, String> {
    let mut smallest: Option<MediaOptimizationAttempt> = None;
    for resize_side in WEB_MEDIA_OPTIMIZE_JPEG_SIDES {
        for compression_level in WEB_MEDIA_OPTIMIZE_PNG_COMPRESSION_STEPS {
            let out = match resize_image_to_png(bytes, *resize_side, *compression_level, true) {
                Ok(row) => row,
                Err(_) => continue,
            };
            let attempt = MediaOptimizationAttempt {
                buffer: out,
                format: "png",
                resize_side: *resize_side,
                quality: None,
                compression_level: Some(*compression_level),
                converted_from_heic: false,
                preserved_alpha: true,
            };
            if smallest
                .as_ref()
                .map(|row| attempt.buffer.len() < row.buffer.len())
                .unwrap_or(true)
            {
                smallest = Some(MediaOptimizationAttempt {
                    buffer: attempt.buffer.clone(),
                    ..attempt
                });
            }
            if attempt.buffer.len() <= max_bytes {
                return Ok(attempt);
            }
        }
    }
    smallest.ok_or_else(|| "Failed to optimize PNG image".to_string())
}

fn optimize_image_to_jpeg(
    bytes: &[u8],
    max_bytes: usize,
    content_type: &str,
    file_name: &str,
) -> Result<MediaOptimizationAttempt, String> {
    let (source, converted_from_heic) = if media_is_heic_source(content_type, file_name) {
        (convert_heic_to_jpeg(bytes)?, true)
    } else {
        (bytes.to_vec(), false)
    };
    let mut smallest: Option<MediaOptimizationAttempt> = None;
    for resize_side in WEB_MEDIA_OPTIMIZE_JPEG_SIDES {
        for quality in WEB_MEDIA_OPTIMIZE_JPEG_QUALITY_STEPS {
            let out = match resize_image_to_jpeg(&source, *resize_side, *quality, true) {
                Ok(row) => row,
                Err(_) => continue,
            };
            let attempt = MediaOptimizationAttempt {
                buffer: out,
                format: "jpeg",
                resize_side: *resize_side,
                quality: Some(*quality),
                compression_level: None,
                converted_from_heic,
                preserved_alpha: false,
            };
            if smallest
                .as_ref()
                .map(|row| attempt.buffer.len() < row.buffer.len())
                .unwrap_or(true)
            {
                smallest = Some(MediaOptimizationAttempt {
                    buffer: attempt.buffer.clone(),
                    ..attempt
                });
            }
            if attempt.buffer.len() <= max_bytes {
                return Ok(attempt);
            }
        }
    }
    smallest.ok_or_else(|| "Failed to optimize image".to_string())
}

fn web_media_image_optimization_contract() -> Value {
    json!({
        "request_fields": ["max_bytes", "optimize_images", "optimizeImages", "raw"],
        "default_optimize_images": true,
        "raw_alias_disables_optimization": true,
        "default_max_bytes_rule": "kind_budget_when_unspecified",
        "explicit_max_bytes_overrides_kind_budget": true,
        "prefetch_cap_rule": "max(request.max_bytes, document_kind_cap)_when_optimizing_else_request_or_document_cap",
        "gif_passthrough": true,
        "alpha_png_preserves_png": true,
        "heic_converts_to_jpeg": true,
        "jpeg_resize_sides": WEB_MEDIA_OPTIMIZE_JPEG_SIDES,
        "jpeg_quality_steps": WEB_MEDIA_OPTIMIZE_JPEG_QUALITY_STEPS,
        "png_compression_levels": WEB_MEDIA_OPTIMIZE_PNG_COMPRESSION_STEPS,
        "returns": [
            "optimize_images",
            "optimized",
            "original_bytes",
            "effective_max_bytes",
            "optimization.format",
            "optimization.resize_side",
            "optimization.quality",
            "optimization.compression_level",
            "optimization.converted_from_heic"
        ]
    })
}
