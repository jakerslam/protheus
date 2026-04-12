const DEFAULT_QR_SCALE: u32 = 6;
const DEFAULT_QR_MARGIN_MODULES: u32 = 4;
const MAX_QR_SCALE: u32 = 32;
const MAX_QR_MARGIN_MODULES: u32 = 16;
const MAX_QR_TEXT_CHARS: usize = 2048;
const PROMPT_IMAGE_ORDER_VALUES: &[&str] = &["inline", "offloaded"];

fn web_media_prompt_image_order_contract() -> Value {
    json!({
        "supported": PROMPT_IMAGE_ORDER_VALUES,
        "default": "inline",
        "inline_delivery": "data_url_when_not_summary_only",
        "offloaded_delivery": "artifact_only"
    })
}

fn web_media_audio_tag_contract() -> Value {
    json!({
        "tag": "[[audio_as_voice]]",
        "fields": ["audio_as_voice", "had_audio_tag", "audio_delivery_mode"],
        "default_delivery_mode": "file",
        "voice_delivery_mode": "voice"
    })
}

fn normalize_prompt_image_order(raw: &str) -> Option<String> {
    let normalized = clean_text(raw, 40).to_ascii_lowercase();
    match normalized.as_str() {
        "" => Some("inline".to_string()),
        "inline" | "offloaded" => Some(normalized),
        _ => None,
    }
}

fn prompt_image_order_from_request(request: &Value) -> Result<String, Value> {
    let raw = request
        .get("prompt_image_order")
        .or_else(|| request.get("promptImageOrder"))
        .and_then(Value::as_str)
        .unwrap_or("");
    normalize_prompt_image_order(raw).ok_or_else(|| {
        json!({
            "ok": false,
            "error": "invalid_prompt_image_order",
            "supported": PROMPT_IMAGE_ORDER_VALUES
        })
    })
}

fn web_media_qr_contract() -> Value {
    json!({
        "request_fields": ["text", "scale", "margin_modules", "prompt_image_order", "summary_only"],
        "content_type": "image/png",
        "default_scale": DEFAULT_QR_SCALE,
        "max_scale": MAX_QR_SCALE,
        "default_margin_modules": DEFAULT_QR_MARGIN_MODULES,
        "max_margin_modules": MAX_QR_MARGIN_MODULES,
        "prompt_image_order_contract": web_media_prompt_image_order_contract(),
        "returns": ["artifact.path", "content_type", "bytes", "prompt_image_order", "data_url"]
    })
}

fn append_web_media_qr_tool_entry(tool_catalog: &mut Value, policy: &Value) {
    if let Some(rows) = tool_catalog.as_array_mut() {
        rows.push(json!({
            "tool": "web_media_qr_image",
            "label": "Web Media QR Image",
            "family": "media",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "request_contract": web_media_qr_contract()
        }));
    }
}

fn parse_qr_u32(request: &Value, key: &str, fallback: u32, min: u32, max: u32) -> u32 {
    request
        .get(key)
        .and_then(Value::as_u64)
        .map(|row| row.clamp(min as u64, max as u64) as u32)
        .unwrap_or(fallback)
}

fn encode_png_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, Value> {
    let mut output = Vec::new();
    let mut encoder = png::Encoder::new(&mut output, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|err| {
        json!({
            "ok": false,
            "error": "png_encode_failed",
            "reason": clean_text(&err.to_string(), 240)
        })
    })?;
    writer.write_image_data(rgba).map_err(|err| {
        json!({
            "ok": false,
            "error": "png_encode_failed",
            "reason": clean_text(&err.to_string(), 240)
        })
    })?;
    drop(writer);
    Ok(output)
}

fn render_qr_png_bytes(text: &str, scale: u32, margin_modules: u32) -> Result<Vec<u8>, Value> {
    let qr = qrcodegen::QrCode::encode_text(text, qrcodegen::QrCodeEcc::Low).map_err(|err| {
        json!({
            "ok": false,
            "error": "qr_encode_failed",
            "reason": clean_text(&format!("{err:?}"), 240)
        })
    })?;
    let modules = qr.size().max(1) as u32;
    let size = (modules + (margin_modules * 2)).saturating_mul(scale.max(1));
    if size == 0 || size > 4096 {
        return Err(json!({
            "ok": false,
            "error": "qr_size_out_of_range",
            "size": size
        }));
    }
    let mut rgba = vec![255u8; (size as usize) * (size as usize) * 4];
    for row in 0..modules {
        for col in 0..modules {
            if !qr.get_module(col as i32, row as i32) {
                continue;
            }
            let start_x = (col + margin_modules) * scale;
            let start_y = (row + margin_modules) * scale;
            for y in 0..scale {
                let pixel_y = start_y + y;
                for x in 0..scale {
                    let pixel_x = start_x + x;
                    let idx = (((pixel_y * size) + pixel_x) * 4) as usize;
                    rgba[idx] = 0;
                    rgba[idx + 1] = 0;
                    rgba[idx + 2] = 0;
                    rgba[idx + 3] = 255;
                }
            }
        }
    }
    encode_png_rgba(size, size, &rgba)
}

fn persist_qr_artifact(root: &Path, png_bytes: &[u8]) -> Result<Value, Value> {
    let response_hash = hex::encode(Sha256::digest(png_bytes));
    let artifact_id = format!("web-media-qr-{}", &response_hash[..16]);
    let path = artifacts_dir_path(root).join(format!("{artifact_id}.png"));
    fs::create_dir_all(path.parent().unwrap_or(root)).map_err(|err| {
        json!({
            "ok": false,
            "error": "artifact_dir_create_failed",
            "reason": clean_text(&err.to_string(), 240)
        })
    })?;
    fs::write(&path, png_bytes).map_err(|err| {
        json!({
            "ok": false,
            "error": "artifact_write_failed",
            "reason": clean_text(&err.to_string(), 240)
        })
    })?;
    Ok(json!({
        "artifact_id": artifact_id,
        "path": path.display().to_string(),
        "bytes": png_bytes.len(),
        "content_type": "image/png",
        "file_name": "qr.png"
    }))
}

fn api_qr_image(root: &Path, request: &Value) -> Value {
    let raw_text = request
        .get("text")
        .or_else(|| request.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let text = raw_text.trim().chars().take(MAX_QR_TEXT_CHARS).collect::<String>();
    if text.is_empty() {
        return json!({
            "ok": false,
            "error": "missing_text",
            "qr_contract": web_media_qr_contract()
        });
    }
    let prompt_image_order = match prompt_image_order_from_request(request) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let scale = parse_qr_u32(
        request,
        "scale",
        DEFAULT_QR_SCALE,
        1,
        MAX_QR_SCALE,
    );
    let margin_modules = parse_qr_u32(
        request,
        "margin_modules",
        DEFAULT_QR_MARGIN_MODULES,
        0,
        MAX_QR_MARGIN_MODULES,
    );
    let summary_only = request
        .get("summary_only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let png_bytes = match render_qr_png_bytes(&text, scale, margin_modules) {
        Ok(bytes) => bytes,
        Err(err) => return err,
    };
    let artifact = match persist_qr_artifact(root, &png_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let data_url = if !summary_only && prompt_image_order == "inline" {
        use base64::Engine;
        Value::String(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png_bytes)
        ))
    } else {
        Value::Null
    };
    json!({
        "ok": true,
        "type": "web_conduit_qr_image",
        "text": text,
        "content_type": "image/png",
        "bytes": png_bytes.len(),
        "prompt_image_order": prompt_image_order,
        "artifact": artifact,
        "data_url": data_url,
        "qr_contract": web_media_qr_contract()
    })
}
