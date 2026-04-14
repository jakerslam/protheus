const IMAGE_REDUCE_QUALITY_STEPS: &[u64] = &[85, 75, 65, 55, 45, 35];
const MAX_IMAGE_INPUT_PIXELS: u64 = 25_000_000;
const POSIX_OPENCLAW_TMP_DIR: &str = "/tmp/openclaw";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ImageMetadata {
    width: u64,
    height: u64,
}

fn build_image_resize_side_grid(max_side: u64, side_start: u64) -> Vec<u64> {
    let mut rows = Vec::new();
    for value in [side_start, 1800, 1600, 1400, 1200, 1000, 800] {
        let clamped = value.min(max_side);
        if clamped > 0 && !rows.contains(&clamped) {
            rows.push(clamped);
        }
    }
    rows.sort_unstable_by(|a, b| b.cmp(a));
    rows
}

fn clean_error_snippet(raw: &[u8]) -> String {
    clean_text(&String::from_utf8_lossy(raw), 240)
}

fn prefers_sips_backend() -> bool {
    match std::env::var("OPENCLAW_IMAGE_BACKEND") {
        Ok(value) if value.trim().eq_ignore_ascii_case("sips") => true,
        Ok(value) if value.trim().eq_ignore_ascii_case("sharp") => false,
        _ => cfg!(target_os = "macos"),
    }
}

fn set_private_dir_permissions(_path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o700);
            let _ = fs::set_permissions(_path, perms);
        }
    }
}

fn ensure_openclaw_tmp_root(candidate: &Path) -> Result<(), String> {
    match fs::symlink_metadata(candidate) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err("symlink_tmp_root".to_string());
            }
            if !metadata.is_dir() {
                return Err("non_directory_tmp_root".to_string());
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(candidate).map_err(|err| format!("create_tmp_root_failed:{err}"))?;
        }
        Err(err) => return Err(format!("inspect_tmp_root_failed:{err}")),
    }
    set_private_dir_permissions(candidate);
    let metadata =
        fs::symlink_metadata(candidate).map_err(|err| format!("verify_tmp_root_failed:{err}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("invalid_tmp_root".to_string());
    }
    Ok(())
}

fn preferred_openclaw_tmp_dir() -> PathBuf {
    let preferred = PathBuf::from(POSIX_OPENCLAW_TMP_DIR);
    if ensure_openclaw_tmp_root(&preferred).is_ok() {
        return preferred;
    }
    let fallback = std::env::temp_dir().join("openclaw");
    let _ = ensure_openclaw_tmp_root(&fallback);
    fallback
}

fn create_openclaw_image_temp_dir() -> Result<PathBuf, String> {
    let root = preferred_openclaw_tmp_dir();
    ensure_openclaw_tmp_root(&root)?;
    for attempt in 0..64 {
        let nonce = Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
        let candidate = root.join(format!(
            "openclaw-img-{}-{}-{}",
            std::process::id(),
            nonce,
            attempt
        ));
        match fs::create_dir(&candidate) {
            Ok(_) => {
                set_private_dir_permissions(&candidate);
                return Ok(candidate);
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("create_image_temp_dir_failed:{err}")),
        }
    }
    Err("create_image_temp_dir_exhausted".to_string())
}

fn with_openclaw_image_temp_dir<T, F>(callback: F) -> Result<T, String>
where
    F: FnOnce(&Path) -> Result<T, String>,
{
    let dir = create_openclaw_image_temp_dir()?;
    let result = callback(&dir);
    let _ = fs::remove_dir_all(&dir);
    result
}

fn build_image_metadata(width: u64, height: u64) -> Option<ImageMetadata> {
    if width == 0 || height == 0 {
        return None;
    }
    Some(ImageMetadata { width, height })
}

fn read_png_metadata(bytes: &[u8]) -> Option<ImageMetadata> {
    if bytes.len() < 24 {
        return None;
    }
    if &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    build_image_metadata(
        u32::from_be_bytes(bytes[16..20].try_into().ok()?) as u64,
        u32::from_be_bytes(bytes[20..24].try_into().ok()?) as u64,
    )
}

fn read_gif_metadata(bytes: &[u8]) -> Option<ImageMetadata> {
    if bytes.len() < 10 {
        return None;
    }
    let signature = &bytes[..6];
    if signature != b"GIF87a" && signature != b"GIF89a" {
        return None;
    }
    build_image_metadata(
        u16::from_le_bytes(bytes[6..8].try_into().ok()?) as u64,
        u16::from_le_bytes(bytes[8..10].try_into().ok()?) as u64,
    )
}

fn read_webp_metadata(bytes: &[u8]) -> Option<ImageMetadata> {
    if bytes.len() < 30 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return None;
    }
    let chunk = &bytes[12..16];
    if chunk == b"VP8X" {
        let width =
            u32::from_le_bytes([bytes[24], bytes[25], bytes[26], 0]).saturating_add(1) as u64;
        let height =
            u32::from_le_bytes([bytes[27], bytes[28], bytes[29], 0]).saturating_add(1) as u64;
        return build_image_metadata(width, height);
    }
    if chunk == b"VP8 " {
        let width = (u16::from_le_bytes(bytes[26..28].try_into().ok()?) & 0x3fff) as u64;
        let height = (u16::from_le_bytes(bytes[28..30].try_into().ok()?) & 0x3fff) as u64;
        return build_image_metadata(width, height);
    }
    if chunk == b"VP8L" && bytes.len() >= 25 && bytes[20] == 0x2f {
        let bits = u32::from_le_bytes(bytes[21..25].try_into().ok()?);
        let width = ((bits & 0x3fff) + 1) as u64;
        let height = (((bits >> 14) & 0x3fff) + 1) as u64;
        return build_image_metadata(width, height);
    }
    None
}

fn read_jpeg_metadata(bytes: &[u8]) -> Option<ImageMetadata> {
    if bytes.len() < 4 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return None;
    }
    let mut offset = 2usize;
    while offset + 8 < bytes.len() {
        while offset < bytes.len() && bytes[offset] == 0xff {
            offset += 1;
        }
        if offset >= bytes.len() {
            return None;
        }
        let marker = bytes[offset];
        offset += 1;
        if marker == 0xd8 || marker == 0xd9 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        if offset + 1 >= bytes.len() {
            return None;
        }
        let segment_len = u16::from_be_bytes(bytes[offset..offset + 2].try_into().ok()?) as usize;
        if segment_len < 2 || offset + segment_len > bytes.len() {
            return None;
        }
        let is_sof =
            (0xc0..=0xcf).contains(&marker) && marker != 0xc4 && marker != 0xc8 && marker != 0xcc;
        if is_sof {
            if segment_len < 7 || offset + 6 >= bytes.len() {
                return None;
            }
            let height = u16::from_be_bytes(bytes[offset + 3..offset + 5].try_into().ok()?) as u64;
            let width = u16::from_be_bytes(bytes[offset + 5..offset + 7].try_into().ok()?) as u64;
            return build_image_metadata(width, height);
        }
        offset += segment_len;
    }
    None
}

fn read_image_metadata_from_header(bytes: &[u8]) -> Option<ImageMetadata> {
    read_png_metadata(bytes)
        .or_else(|| read_gif_metadata(bytes))
        .or_else(|| read_webp_metadata(bytes))
        .or_else(|| read_jpeg_metadata(bytes))
}

fn count_image_pixels(meta: ImageMetadata) -> Option<u64> {
    meta.width.checked_mul(meta.height)
}

fn create_image_pixel_limit_error(meta: ImageMetadata) -> String {
    match count_image_pixels(meta) {
        Some(pixels) => format!(
            "Image dimensions exceed the {} pixel input limit: {}x{} ({} pixels)",
            MAX_IMAGE_INPUT_PIXELS, meta.width, meta.height, pixels
        ),
        None => format!(
            "Image dimensions exceed the {} pixel input limit: {}x{}",
            MAX_IMAGE_INPUT_PIXELS, meta.width, meta.height
        ),
    }
}

fn validate_image_pixel_limit(meta: ImageMetadata) -> Result<ImageMetadata, String> {
    let exceeded = match count_image_pixels(meta) {
        Some(pixels) => pixels > MAX_IMAGE_INPUT_PIXELS,
        None => true,
    };
    if exceeded {
        Err(create_image_pixel_limit_error(meta))
    } else {
        Ok(meta)
    }
}

fn run_sips_capture(args: &[String]) -> Result<Vec<u8>, String> {
    let output = Command::new("/usr/bin/sips")
        .args(args)
        .output()
        .map_err(|err| format!("sips_exec_failed:{err}"))?;
    if !output.status.success() {
        let snippet = if !output.stderr.is_empty() {
            clean_error_snippet(&output.stderr)
        } else {
            clean_error_snippet(&output.stdout)
        };
        return Err(format!("sips_failed:{snippet}"));
    }
    Ok(output.stdout)
}

fn sips_image_metadata_from_buffer(bytes: &[u8]) -> Option<ImageMetadata> {
    with_openclaw_image_temp_dir(|dir| {
        let input = dir.join("in.img");
        fs::write(&input, bytes).map_err(|err| format!("write_sips_input_failed:{err}"))?;
        let stdout = run_sips_capture(&[
            "-g".to_string(),
            "pixelWidth".to_string(),
            "-g".to_string(),
            "pixelHeight".to_string(),
            input.display().to_string(),
        ])?;
        let text = String::from_utf8_lossy(&stdout);
        let width = text
            .lines()
            .find_map(|line| line.trim().strip_prefix("pixelWidth:"))
            .and_then(|value| value.trim().parse::<u64>().ok());
        let height = text
            .lines()
            .find_map(|line| line.trim().strip_prefix("pixelHeight:"))
            .and_then(|value| value.trim().parse::<u64>().ok());
        build_image_metadata(width.unwrap_or(0), height.unwrap_or(0))
            .ok_or_else(|| "sips_dimensions_missing".to_string())
    })
    .ok()
}

fn get_image_metadata(bytes: &[u8]) -> Option<ImageMetadata> {
    if let Some(meta) = read_image_metadata_from_header(bytes) {
        return validate_image_pixel_limit(meta).ok();
    }
    if prefers_sips_backend() {
        return sips_image_metadata_from_buffer(bytes)
            .and_then(|meta| validate_image_pixel_limit(meta).ok());
    }
    None
}

fn assert_image_pixel_limit(bytes: &[u8]) -> Result<(), String> {
    if let Some(meta) = read_image_metadata_from_header(bytes) {
        validate_image_pixel_limit(meta)?;
        return Ok(());
    }
    if prefers_sips_backend() {
        let Some(meta) = sips_image_metadata_from_buffer(bytes) else {
            return Err("Unable to determine image dimensions; refusing to process".to_string());
        };
        validate_image_pixel_limit(meta)?;
    }
    Ok(())
}

fn resize_image_to_jpeg(
    bytes: &[u8],
    max_side: u64,
    quality: u64,
    without_enlargement: bool,
) -> Result<Vec<u8>, String> {
    assert_image_pixel_limit(bytes)?;
    if !prefers_sips_backend() {
        return Err("image_resize_backend_unavailable".to_string());
    }
    with_openclaw_image_temp_dir(|dir| {
        let normalized = normalize_exif_orientation_for_sips(bytes);
        let input = dir.join("in.img");
        let output = dir.join("out.jpg");
        fs::write(&input, &normalized).map_err(|err| format!("write_resize_input_failed:{err}"))?;
        let effective_side = if without_enlargement {
            if let Some(meta) = get_image_metadata(&normalized) {
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
            "jpeg".to_string(),
            "-s".to_string(),
            "formatOptions".to_string(),
            quality.clamp(1, 100).to_string(),
            input.display().to_string(),
            "--out".to_string(),
            output.display().to_string(),
        ])?;
        fs::read(&output).map_err(|err| format!("read_resize_output_failed:{err}"))
    })
}
