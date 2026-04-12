fn web_media_image_ops_contract() -> Value {
    json!({
        "request_fields": ["url", "path", "workspace_dir", "local_roots", "host_read_capability", "max_bytes", "summary_only"],
        "returns": ["width", "height", "pixels", "resize_side_grid", "quality_steps", "backend", "temp_root"],
        "max_input_pixels": MAX_IMAGE_INPUT_PIXELS,
        "quality_steps": IMAGE_REDUCE_QUALITY_STEPS,
        "resize_side_grid_contract": {
            "inputs": ["max_side", "side_start"],
            "descending": true,
            "default_candidates": [1800, 1600, 1400, 1200, 1000, 800]
        },
        "header_probe_formats": ["png", "gif", "webp", "jpeg"],
        "preferred_backend": if prefers_sips_backend() { "sips" } else { "header_only" },
        "secure_temp_root": preferred_openclaw_tmp_dir().display().to_string(),
        "secure_temp_prefix": "openclaw-img-"
    })
}

fn append_web_media_image_ops_tool_entry(tool_catalog: &mut Value, policy: &Value) {
    if let Some(rows) = tool_catalog.as_array_mut() {
        rows.push(json!({
            "tool": "web_media_image_metadata",
            "label": "Web Media Image Metadata",
            "family": "media",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "request_contract": web_media_image_ops_contract()
        }));
    }
}

fn api_image_metadata(root: &Path, request: &Value) -> Value {
    let raw = media_request_source(request);
    if raw.is_empty() {
        return json!({"ok": false, "error": "media_source_required"});
    }
    let requested_source = if raw.starts_with("data:") {
        media_redacted_inline_source(&raw)
    } else {
        raw.clone()
    };
    let loaded = if raw.starts_with("http://") || raw.starts_with("https://") {
        fetch_remote_media_binary(root, request)
    } else if raw.starts_with("data:") {
        load_inline_media_binary(request)
    } else {
        load_local_media_binary(root, request)
    };
    match loaded {
        Ok(loaded) => {
            if loaded.kind != "image" {
                let receipt = build_receipt(
                    &requested_source,
                    "deny",
                    None,
                    loaded.status_code,
                    "not-image",
                    None,
                );
                let _ = append_jsonl(&receipts_path(root), &receipt);
                return json!({
                    "ok": false,
                    "type": "web_conduit_image_metadata",
                    "error": "not-image",
                    "requested_source": requested_source,
                    "resolved_source": loaded.resolved_source,
                    "content_type": loaded.content_type,
                    "kind": loaded.kind,
                    "image_ops_contract": web_media_image_ops_contract(),
                    "receipt": receipt
                });
            }
            if let Err(err) = assert_image_pixel_limit(&loaded.buffer) {
                let receipt = build_receipt(
                    &requested_source,
                    "deny",
                    None,
                    loaded.status_code,
                    "image_pixel_limit",
                    Some(&err),
                );
                let _ = append_jsonl(&receipts_path(root), &receipt);
                return json!({
                    "ok": false,
                    "type": "web_conduit_image_metadata",
                    "error": "image_pixel_limit",
                    "reason": err,
                    "requested_source": requested_source,
                    "resolved_source": loaded.resolved_source,
                    "content_type": loaded.content_type,
                    "image_ops_contract": web_media_image_ops_contract(),
                    "receipt": receipt
                });
            }
            let Some(meta) = get_image_metadata(&loaded.buffer) else {
                let receipt = build_receipt(
                    &requested_source,
                    "deny",
                    None,
                    loaded.status_code,
                    "image_dimensions_unknown",
                    None,
                );
                let _ = append_jsonl(&receipts_path(root), &receipt);
                return json!({
                    "ok": false,
                    "type": "web_conduit_image_metadata",
                    "error": "image_dimensions_unknown",
                    "requested_source": requested_source,
                    "resolved_source": loaded.resolved_source,
                    "content_type": loaded.content_type,
                    "image_ops_contract": web_media_image_ops_contract(),
                    "receipt": receipt
                });
            };
            let pixels = count_image_pixels(meta).unwrap_or(0);
            let resize_side_grid = build_image_resize_side_grid(
                meta.width.max(meta.height),
                meta.width.min(meta.height),
            );
            let response_hash = sha256_hex(&format!(
                "{}x{}:{}",
                meta.width, meta.height, requested_source
            ));
            let receipt = build_receipt(
                &requested_source,
                "allow",
                Some(&response_hash),
                loaded.status_code,
                "image_metadata_loaded",
                None,
            );
            let _ = append_jsonl(&receipts_path(root), &receipt);
            json!({
                "ok": true,
                "type": "web_conduit_image_metadata",
                "requested_source": requested_source,
                "resolved_source": loaded.resolved_source,
                "source_kind": loaded.source_kind,
                "provider": loaded.provider,
                "provider_hint": loaded.provider_hint,
                "citation_redirect_resolved": loaded.citation_redirect_resolved,
                "redirect_count": loaded.redirect_count,
                "status_code": loaded.status_code,
                "content_type": loaded.content_type,
                "file_name": loaded.file_name,
                "kind": loaded.kind,
                "bytes": loaded.buffer.len(),
                "width": meta.width,
                "height": meta.height,
                "pixels": pixels,
                "max_input_pixels": MAX_IMAGE_INPUT_PIXELS,
                "resize_side_grid": resize_side_grid,
                "quality_steps": IMAGE_REDUCE_QUALITY_STEPS,
                "backend": if prefers_sips_backend() { "sips" } else { "header_only" },
                "temp_root": preferred_openclaw_tmp_dir().display().to_string(),
                "summary": format!("Image {}x{} ({} pixels).", meta.width, meta.height, pixels),
                "image_ops_contract": web_media_image_ops_contract(),
                "receipt": receipt
            })
        }
        Err(mut err) => {
            let status_code = err.get("status_code").and_then(Value::as_i64).unwrap_or(0);
            let reason = clean_text(
                err.get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("web_image_metadata_failed"),
                180,
            );
            let receipt = build_receipt(
                &requested_source,
                "deny",
                None,
                status_code,
                &reason,
                err.get("body_snippet").and_then(Value::as_str),
            );
            let _ = append_jsonl(&receipts_path(root), &receipt);
            if let Some(obj) = err.as_object_mut() {
                obj.insert("type".to_string(), json!("web_conduit_image_metadata"));
                obj.insert("requested_source".to_string(), json!(requested_source));
                obj.insert(
                    "image_ops_contract".to_string(),
                    web_media_image_ops_contract(),
                );
                obj.insert("receipt".to_string(), receipt);
            }
            err
        }
    }
}
