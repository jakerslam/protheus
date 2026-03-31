
fn extract_date_from_path(path_value: &str) -> String {
    let chars = path_value.chars().collect::<Vec<char>>();
    if chars.len() < 10 {
        return String::new();
    }
    for idx in 0..=(chars.len() - 10) {
        let mut ok = true;
        for off in 0..10 {
            let ch = chars[idx + off];
            let expected_dash = off == 4 || off == 7;
            if expected_dash {
                if ch != '-' {
                    ok = false;
                    break;
                }
            } else if !ch.is_ascii_digit() {
                ok = false;
                break;
            }
        }
        if ok {
            return chars[idx..idx + 10].iter().collect::<String>();
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
