fn media_cap_error_payload(
    loaded: &LoadedMedia,
    request: &Value,
    optimize_images: bool,
    effective_max_bytes: usize,
    bytes: usize,
    original_bytes: usize,
    optimization: Value,
    reason: &str,
) -> Value {
    let explicit_max = media_requested_max_bytes(request);
    let error = if explicit_max.is_some() {
        "max_bytes"
    } else {
        "kind_max_bytes"
    };
    json!({
        "ok": false,
        "error": error,
        "reason": clean_text(reason, 240),
        "resolved_source": loaded.resolved_source,
        "status_code": loaded.status_code,
        "detected_kind": loaded.kind,
        "content_type": loaded.content_type,
        "kind_max_bytes": max_bytes_for_media_kind(&loaded.kind),
        "effective_max_bytes": effective_max_bytes,
        "bytes": bytes,
        "declared_size": original_bytes,
        "original_bytes": original_bytes,
        "optimize_images": optimize_images,
        "optimized": false,
        "optimization": optimization,
        "redirect_count": loaded.redirect_count
    })
}

fn media_optimization_error_payload(
    loaded: &LoadedMedia,
    request: &Value,
    optimize_images: bool,
    original_bytes: usize,
    reason: &str,
) -> Value {
    json!({
        "ok": false,
        "error": "image_optimization_failed",
        "reason": clean_text(reason, 240),
        "resolved_source": loaded.resolved_source,
        "status_code": loaded.status_code,
        "detected_kind": loaded.kind,
        "content_type": loaded.content_type,
        "kind_max_bytes": max_bytes_for_media_kind(&loaded.kind),
        "effective_max_bytes": media_effective_output_max_bytes(request, &loaded.kind),
        "bytes": loaded.buffer.len(),
        "original_bytes": original_bytes,
        "optimize_images": optimize_images,
        "optimized": false,
        "optimization": {
            "enabled": optimize_images,
            "optimized": false,
            "reason": "image_optimization_failed"
        },
        "redirect_count": loaded.redirect_count
    })
}

fn finalize_loaded_media_for_request(
    loaded: LoadedMedia,
    request: &Value,
) -> Result<FinalizedMedia, Value> {
    let optimize_images = media_request_optimize_images(request);
    let effective_max_bytes = media_effective_output_max_bytes(request, &loaded.kind);
    let original_bytes = loaded.buffer.len();
    if loaded.kind != "image" {
        if loaded.buffer.len() > effective_max_bytes {
            return Err(media_cap_error_payload(
                &loaded,
                request,
                optimize_images,
                effective_max_bytes,
                loaded.buffer.len(),
                original_bytes,
                json!({
                    "enabled": optimize_images,
                    "optimized": false,
                    "reason": "non_image_passthrough"
                }),
                &media_format_cap_limit("Media", effective_max_bytes, loaded.buffer.len()),
            ));
        }
        return Ok(FinalizedMedia {
            loaded,
            effective_max_bytes,
            original_bytes,
            optimize_images,
            optimized: false,
            optimization: json!({
                "enabled": optimize_images,
                "optimized": false,
                "reason": "non_image_passthrough"
            }),
        });
    }
    if media_is_gif_source(&loaded.content_type) || !optimize_images {
        let passthrough_reason = if media_is_gif_source(&loaded.content_type) {
            "gif_passthrough"
        } else {
            "raw_passthrough"
        };
        if loaded.buffer.len() > effective_max_bytes {
            return Err(media_cap_error_payload(
                &loaded,
                request,
                optimize_images,
                effective_max_bytes,
                loaded.buffer.len(),
                original_bytes,
                json!({
                    "enabled": optimize_images,
                    "optimized": false,
                    "reason": passthrough_reason
                }),
                &media_format_cap_limit(
                    if media_is_gif_source(&loaded.content_type) {
                        "GIF"
                    } else {
                        "Media"
                    },
                    effective_max_bytes,
                    loaded.buffer.len(),
                ),
            ));
        }
        return Ok(FinalizedMedia {
            loaded,
            effective_max_bytes,
            original_bytes,
            optimize_images,
            optimized: false,
            optimization: json!({
                "enabled": optimize_images,
                "optimized": false,
                "reason": passthrough_reason
            }),
        });
    }
    let try_alpha_png = media_is_png_source(&loaded.content_type, &loaded.file_name)
        && png_has_alpha_channel(&loaded.buffer);
    if try_alpha_png {
        let optimized = match optimize_image_to_png(&loaded.buffer, effective_max_bytes) {
            Ok(row) => row,
            Err(err) if original_bytes <= effective_max_bytes => {
                return Ok(FinalizedMedia {
                    loaded,
                    effective_max_bytes,
                    original_bytes,
                    optimize_images,
                    optimized: false,
                    optimization: json!({
                        "enabled": true,
                        "optimized": false,
                        "reason": "already_within_cap"
                    }),
                });
            }
            Err(err) => {
                return Err(media_optimization_error_payload(
                    &loaded,
                    request,
                    optimize_images,
                    original_bytes,
                    &err,
                ));
            }
        };
        if optimized.buffer.len() <= effective_max_bytes {
            let optimized_size = optimized.buffer.len();
            let transformed = LoadedMedia {
                buffer: optimized.buffer,
                content_type: "image/png".to_string(),
                kind: "image".to_string(),
                file_name: loaded.file_name.clone(),
                resolved_source: loaded.resolved_source.clone(),
                source_kind: loaded.source_kind.clone(),
                status_code: loaded.status_code,
                provider: loaded.provider.clone(),
                provider_hint: loaded.provider_hint.clone(),
                citation_redirect_resolved: loaded.citation_redirect_resolved,
                redirect_count: loaded.redirect_count,
            };
            let optimization = json!({
                "enabled": true,
                "optimized": optimized_size != original_bytes,
                "format": optimized.format,
                "resize_side": optimized.resize_side,
                "compression_level": optimized.compression_level,
                "converted_from_heic": false,
                "preserved_alpha": true,
                "reason": "alpha_png"
            });
            return Ok(FinalizedMedia {
                loaded: transformed,
                effective_max_bytes,
                original_bytes,
                optimize_images,
                optimized: optimized_size != original_bytes,
                optimization,
            });
        }
    }
    let optimized = optimize_image_to_jpeg(
        &loaded.buffer,
        effective_max_bytes,
        &loaded.content_type,
        &loaded.file_name,
    );
    let optimized = match optimized {
        Ok(row) => row,
        Err(err) if original_bytes <= effective_max_bytes => {
            return Ok(FinalizedMedia {
                loaded,
                effective_max_bytes,
                original_bytes,
                optimize_images,
                optimized: false,
                optimization: json!({
                    "enabled": true,
                    "optimized": false,
                    "reason": "already_within_cap"
                }),
            });
        }
        Err(err) => {
            return Err(media_optimization_error_payload(
                &loaded,
                request,
                optimize_images,
                original_bytes,
                &err,
            ));
        }
    };
    if optimized.buffer.len() > effective_max_bytes {
        return Err(media_cap_error_payload(
            &loaded,
            request,
            optimize_images,
            effective_max_bytes,
            optimized.buffer.len(),
            original_bytes,
            json!({
                "enabled": true,
                "optimized": false,
                "format": optimized.format,
                "resize_side": optimized.resize_side,
                "quality": optimized.quality,
                "converted_from_heic": optimized.converted_from_heic,
                "reason": "jpeg_optimized"
            }),
            &media_format_cap_reduce("Media", effective_max_bytes, optimized.buffer.len()),
        ));
    }
    let optimized_size = optimized.buffer.len();
    let file_name = if optimized.converted_from_heic {
        media_to_jpeg_file_name(&loaded.file_name)
    } else {
        loaded.file_name.clone()
    };
    let transformed = LoadedMedia {
        buffer: optimized.buffer,
        content_type: "image/jpeg".to_string(),
        kind: "image".to_string(),
        file_name,
        resolved_source: loaded.resolved_source.clone(),
        source_kind: loaded.source_kind.clone(),
        status_code: loaded.status_code,
        provider: loaded.provider.clone(),
        provider_hint: loaded.provider_hint.clone(),
        citation_redirect_resolved: loaded.citation_redirect_resolved,
        redirect_count: loaded.redirect_count,
    };
    let optimization = json!({
        "enabled": true,
        "optimized": optimized_size != original_bytes || optimized.converted_from_heic,
        "format": optimized.format,
        "resize_side": optimized.resize_side,
        "quality": optimized.quality,
        "converted_from_heic": optimized.converted_from_heic,
        "preserved_alpha": optimized.preserved_alpha,
        "reason": "jpeg_optimized"
    });
    Ok(FinalizedMedia {
        loaded: transformed,
        effective_max_bytes,
        original_bytes,
        optimize_images,
        optimized: optimized_size != original_bytes || optimized.converted_from_heic,
        optimization,
    })
}
