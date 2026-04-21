const MAX_DATE_SCAN_PATH_CHARS: usize = 1024;
const MIN_VALID_DATE_YEAR: u32 = 1970;
const MAX_VALID_DATE_YEAR: u32 = 2100;

fn sanitize_path_for_date_scan(path_value: &str) -> Option<String> {
    let trimmed = path_value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_DATE_SCAN_PATH_CHARS {
        return None;
    }
    if trimmed
        .chars()
        .any(|ch| ch == '\0' || ch == '\r' || ch == '\n')
    {
        return None;
    }
    if trimmed
        .split(['/', '\\'])
        .any(|segment| segment.trim() == "..")
    {
        return None;
    }
    Some(
        trimmed
            .chars()
            .take(MAX_DATE_SCAN_PATH_CHARS)
            .collect::<String>(),
    )
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn max_day_for_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn parse_ascii_u32(chars: &[char]) -> Option<u32> {
    let mut value = 0u32;
    for ch in chars {
        if !ch.is_ascii_digit() {
            return None;
        }
        value = value
            .checked_mul(10)?
            .checked_add((*ch as u32) - ('0' as u32))?;
    }
    Some(value)
}

fn is_valid_extracted_date(year: u32, month: u32, day: u32) -> bool {
    let max_day = max_day_for_month(year, month);
    year >= MIN_VALID_DATE_YEAR
        && year <= MAX_VALID_DATE_YEAR
        && month >= 1
        && month <= 12
        && day >= 1
        && day <= max_day
}

fn extract_date_from_path(path_value: &str) -> String {
    let Some(safe_path) = sanitize_path_for_date_scan(path_value) else {
        return String::new();
    };
    let chars = safe_path.chars().collect::<Vec<char>>();
    if chars.len() < 10 {
        return String::new();
    }

    for idx in 0..=(chars.len() - 10) {
        for sep in ['-', '/', '_'] {
            let mut pattern_ok = true;
            for off in 0..10 {
                let ch = chars[idx + off];
                let expected_sep = off == 4 || off == 7;
                if expected_sep {
                    if ch != sep {
                        pattern_ok = false;
                        break;
                    }
                } else if !ch.is_ascii_digit() {
                    pattern_ok = false;
                    break;
                }
            }
            if !pattern_ok {
                continue;
            }

            let before_is_digit = idx > 0 && chars[idx - 1].is_ascii_digit();
            let after_is_digit = idx + 10 < chars.len() && chars[idx + 10].is_ascii_digit();
            if before_is_digit || after_is_digit {
                continue;
            }

            let year = parse_ascii_u32(&chars[idx..idx + 4]).unwrap_or(0);
            let month = parse_ascii_u32(&chars[idx + 5..idx + 7]).unwrap_or(0);
            let day = parse_ascii_u32(&chars[idx + 8..idx + 10]).unwrap_or(0);
            if is_valid_extracted_date(year, month, day) {
                return format!("{year:04}-{month:02}-{day:02}");
            }
        }
    }
    if chars.len() >= 8 {
        for idx in 0..=(chars.len() - 8) {
            let before_is_digit = idx > 0 && chars[idx - 1].is_ascii_digit();
            let after_is_digit = idx + 8 < chars.len() && chars[idx + 8].is_ascii_digit();
            if before_is_digit || after_is_digit {
                continue;
            }
            if !chars[idx..idx + 8].iter().all(|ch| ch.is_ascii_digit()) {
                continue;
            }
            let year = parse_ascii_u32(&chars[idx..idx + 4]).unwrap_or(0);
            let month = parse_ascii_u32(&chars[idx + 4..idx + 6]).unwrap_or(0);
            let day = parse_ascii_u32(&chars[idx + 6..idx + 8]).unwrap_or(0);
            if is_valid_extracted_date(year, month, day) {
                return format!("{year:04}-{month:02}-{day:02}");
            }
        }
    }
    String::new()
}

fn node_id_from_chunk(chunk: &str) -> String {
    for line in chunk.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("node_id:") {
            let normalized = normalize_node_id(rest);
            if !normalized.is_empty() {
                return normalized;
            }
        }
    }
    String::new()
}

fn extract_node_section(file_content: &str, node_id: &str) -> String {
    for chunk in file_content.split("<!-- NODE -->") {
        let detected = node_id_from_chunk(chunk);
        if !detected.is_empty() && detected == node_id {
            return chunk.trim().to_string();
        }
    }
    String::new()
}

fn parse_bool_flag(raw: &str) -> bool {
    matches!(
        raw.trim().to_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn sanitize_table_cell(v: &str) -> String {
    v.replace(['\n', '\r'], " ")
        .replace('|', "/")
        .trim()
        .to_string()
}

fn extract_uid_from_chunk(chunk: &str) -> String {
    for line in chunk.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("uid:") {
            let uid = normalize_uid(rest);
            if !uid.is_empty() {
                return uid;
            }
        }
    }
    String::new()
}

fn parse_tags_line(raw: &str) -> Vec<String> {
    let mut body = raw.trim().to_string();
    if body.starts_with('[') && body.ends_with(']') && body.len() >= 2 {
        body = body[1..body.len() - 1].to_string();
    }
    body = body.replace(['[', ']', '"', '\''], " ");
    let mut set: BTreeSet<String> = BTreeSet::new();
    for token in body.replace(',', " ").split_whitespace() {
        let tag = normalize_tag(token);
        if !tag.is_empty() {
            set.insert(tag);
        }
    }
    set.into_iter().collect::<Vec<String>>()
}
