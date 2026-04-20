fn fetch_url_shape_error_code(raw_requested_url: &str) -> &'static str {
    let lowered =
        fetch_strip_invisible_unicode(&clean_text(raw_requested_url, 2_400)).to_ascii_lowercase();
    let trimmed = lowered.trim();
    if trimmed.is_empty() {
        return "fetch_url_required";
    }
    if (trimmed.starts_with('{') && trimmed.contains(':'))
        || (trimmed.starts_with('[') && trimmed.contains('{'))
    {
        return "fetch_url_payload_dump_detected";
    }
    if trimmed.contains("```")
        || trimmed.contains("diff --git")
        || trimmed.contains("[patch v")
        || trimmed.contains("input specification")
        || trimmed.contains("sample output")
        || trimmed.contains("you are an expert")
    {
        return "fetch_url_payload_dump_detected";
    }
    if trimmed.contains(' ') {
        return "fetch_url_shape_invalid";
    }
    let line_count = trimmed.lines().count();
    if line_count > 6 || trimmed.len() > 2_100 {
        return "fetch_url_shape_invalid";
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return "fetch_url_invalid_scheme";
    }
    "none"
}
