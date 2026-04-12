#[cfg(test)]
mod openclaw_image_ops_tests {
    use super::*;
    use std::fs;

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
            vec![
                0x00, 0x00, 0x00, 0x00, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82,
            ],
        ]
        .concat()
    }

    #[test]
    fn openclaw_image_ops_keeps_expected_quality_ladder() {
        assert_eq!(IMAGE_REDUCE_QUALITY_STEPS, &[85, 75, 65, 55, 45, 35]);
    }

    #[test]
    fn openclaw_image_ops_builds_resize_side_grid() {
        assert_eq!(
            build_image_resize_side_grid(1200, 900),
            vec![1200, 1000, 900, 800]
        );
        assert!(build_image_resize_side_grid(0, 0).is_empty());
    }

    #[test]
    fn openclaw_image_ops_returns_null_metadata_for_images_above_pixel_limit() {
        let oversized = create_png_buffer_with_dimensions(8_000, 4_000);
        assert!(8_000u64 * 4_000u64 > MAX_IMAGE_INPUT_PIXELS);
        assert_eq!(get_image_metadata(&oversized), None);
    }

    #[test]
    fn openclaw_image_ops_rejects_oversized_images_before_resize_work_starts() {
        let oversized = create_png_buffer_with_dimensions(8_000, 4_000);
        let err = resize_image_to_jpeg(&oversized, 2_048, 80, true).expect_err("oversized");
        assert!(err.to_ascii_lowercase().contains("pixel input limit"));
    }

    #[test]
    fn openclaw_image_ops_rejects_overflowed_pixel_counts_before_resize_work_starts() {
        let overflowed = create_png_buffer_with_dimensions(u32::MAX, u32::MAX);
        let err = resize_image_to_jpeg(&overflowed, 2_048, 80, true).expect_err("overflowed");
        assert!(err.to_ascii_lowercase().contains("pixel input limit"));
    }

    #[test]
    fn openclaw_image_ops_fails_closed_when_sips_cannot_determine_image_dimensions() {
        let previous = std::env::var("OPENCLAW_IMAGE_BACKEND").ok();
        std::env::set_var("OPENCLAW_IMAGE_BACKEND", "sips");
        let err = resize_image_to_jpeg(b"not-an-image", 2_048, 80, true).expect_err("invalid");
        match previous {
            Some(value) => std::env::set_var("OPENCLAW_IMAGE_BACKEND", value),
            None => std::env::remove_var("OPENCLAW_IMAGE_BACKEND"),
        }
        assert!(err
            .to_ascii_lowercase()
            .contains("unable to determine image dimensions"));
    }

    #[test]
    fn openclaw_image_ops_creates_temp_dirs_under_secure_root_and_cleans_them_up() {
        let secure_root = preferred_openclaw_tmp_dir();
        let mut created = PathBuf::new();
        with_openclaw_image_temp_dir(|dir| {
            created = dir.to_path_buf();
            assert!(dir.starts_with(&secure_root));
            Ok(())
        })
        .expect("tempdir");
        assert!(!created.exists());
    }

    #[test]
    fn openclaw_image_metadata_api_reports_dimensions_and_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let workspace = tmp.path().join("workspace");
        fs::create_dir_all(&workspace).expect("workspace");
        let target = workspace.join("chart.png");
        fs::write(&target, create_png_buffer_with_dimensions(1200, 900)).expect("png");
        let out = api_image_metadata(
            tmp.path(),
            &json!({
                "path": "chart.png",
                "workspace_dir": workspace.display().to_string()
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("width").and_then(Value::as_u64), Some(1200));
        assert_eq!(out.get("height").and_then(Value::as_u64), Some(900));
        assert_eq!(
            out.pointer("/image_ops_contract/max_input_pixels")
                .and_then(Value::as_u64),
            Some(MAX_IMAGE_INPUT_PIXELS)
        );
        assert_eq!(
            out.get("resize_side_grid")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(4)
        );
    }

    #[test]
    fn openclaw_media_contract_reports_image_ops_surface() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let status = api_status(tmp.path());
        assert_eq!(
            status
                .pointer("/media_request_contract/image_ops_contract/max_input_pixels")
                .and_then(Value::as_u64),
            Some(MAX_IMAGE_INPUT_PIXELS)
        );
        assert!(status
            .pointer("/tool_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.get("tool").and_then(Value::as_str) == Some("web_media_image_metadata")
            }))
            .unwrap_or(false));
    }
}
