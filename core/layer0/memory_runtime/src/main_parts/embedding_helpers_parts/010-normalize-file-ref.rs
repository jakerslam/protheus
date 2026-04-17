fn normalize_file_ref(v: &str) -> String {
    let mut raw = clean_cell(v)
        .trim_matches('"')
        .trim_matches('\'')
        .to_string();
    if raw.is_empty() {
        return String::new();
    }
    if raw.chars().any(char::is_control) {
        return String::new();
    }
    raw = raw.replace('\\', "/");
    while raw.starts_with("./") {
        raw = raw[2..].to_string();
    }
    let lowered = raw.to_ascii_lowercase();
    if lowered.starts_with("http:")
        || lowered.starts_with("https:")
        || lowered.starts_with("javascript:")
        || lowered.starts_with("data:")
        || lowered.starts_with("file:")
    {
        return String::new();
    }
    if raw.starts_with("client/memory/") {
        return raw;
    }
    if is_date_memory_file(&raw) {
        return format!("client/memory/{raw}");
    }
    if raw.starts_with("_archive/") {
        return format!("client/memory/{raw}");
    }
    if raw.ends_with(".md") {
        return raw;
    }
    String::new()
}

fn parse_tag_cell(v: &str) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    for token in v.replace(',', " ").split_whitespace() {
        let tag = normalize_tag(token);
        if !tag.is_empty() {
            set.insert(tag);
        }
    }
    set.into_iter().collect::<Vec<String>>()
}

fn parse_table_cells(trimmed: &str) -> Vec<String> {
    let inner = trimmed.trim_matches('|');
    if inner.is_empty() {
        return vec![];
    }
    inner.split('|').map(clean_cell).collect::<Vec<String>>()
}

fn parse_index_file(file_path: &Path) -> Vec<IndexEntry> {
    let Ok(text) = fs::read_to_string(file_path) else {
        return vec![];
    };
    let mut rows: Vec<IndexEntry> = vec![];
    let mut headers: Option<Vec<String>> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cells = parse_table_cells(trimmed);
        if cells.is_empty() {
            continue;
        }
        if cells
            .iter()
            .all(|c| c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '))
        {
            continue;
        }

        let normalized = cells
            .iter()
            .map(|c| normalize_header_cell(c))
            .collect::<Vec<String>>();
        if normalized.iter().any(|h| h == "node_id") && normalized.iter().any(|h| h == "file") {
            headers = Some(normalized);
            continue;
        }
        let Some(hdr) = headers.as_ref() else {
            continue;
        };

        let mut row: HashMap<String, String> = HashMap::new();
        for (idx, key) in hdr.iter().enumerate() {
            row.insert(
                key.clone(),
                clean_cell(cells.get(idx).unwrap_or(&String::new())),
            );
        }

        let node_id = normalize_node_id(row.get("node_id").map_or("", String::as_str));
        let file_rel = normalize_file_ref(row.get("file").map_or("", String::as_str));
        if node_id.is_empty() || file_rel.is_empty() {
            continue;
        }
        let uid = normalize_uid(row.get("uid").map_or("", String::as_str));
        let summary = clean_cell(row.get("summary").map_or("", String::as_str));
        let tags = parse_tag_cell(row.get("tags").map_or("", String::as_str));
        rows.push(IndexEntry {
            node_id,
            uid,
            file_rel,
            summary,
            tags,
        });
    }
    rows
}

fn rel_path(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

fn dedupe_sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn merge_tags(dst: &mut Vec<String>, src: &[String]) {
    let mut set: BTreeSet<String> = dst.iter().cloned().collect::<BTreeSet<String>>();
    for tag in src {
        if !tag.is_empty() {
            set.insert(tag.clone());
        }
    }
    *dst = set.into_iter().collect::<Vec<String>>();
}

fn load_memory_index(root: &Path) -> (Vec<String>, Vec<IndexEntry>) {
    let paths = vec![
        root.join("docs/workspace/MEMORY_INDEX.md"),
        root.join("client/memory/MEMORY_INDEX.md"),
        root.join("memory").join("MEMORY_INDEX.md"),
    ];
    let mut source = vec![];
    let mut merged: HashMap<String, IndexEntry> = HashMap::new();
    for p in paths {
        if !p.exists() {
            continue;
        }
        source.push(rel_path(root, &p));
        for row in parse_index_file(&p) {
            let key = format!("{}@{}", row.node_id, row.file_rel);
            if !merged.contains_key(&key) {
                merged.insert(key.clone(), row);
                continue;
            }
            if let Some(cur) = merged.get_mut(&key) {
                if cur.uid.is_empty() && !row.uid.is_empty() {
                    cur.uid = row.uid.clone();
                }
                if cur.summary.is_empty() && !row.summary.is_empty() {
                    cur.summary = row.summary.clone();
                }
                merge_tags(&mut cur.tags, &row.tags);
            }
        }
    }
    let mut entries = merged.into_values().collect::<Vec<IndexEntry>>();
    entries.sort_by(|a, b| {
        if a.file_rel != b.file_rel {
            return a.file_rel.cmp(&b.file_rel);
        }
        a.node_id.cmp(&b.node_id)
    });
    (source, entries)
}

fn parse_tags_file(file_path: &Path) -> HashMap<String, HashSet<String>> {
    let Ok(text) = fs::read_to_string(file_path) else {
        return HashMap::new();
    };
    let mut out: HashMap<String, HashSet<String>> = HashMap::new();
    let mut current_tag = String::new();
    for line in text.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("## ") {
            let mut raw = rest.trim().to_string();
            if raw.starts_with('`') && raw.ends_with('`') && raw.len() >= 2 {
                raw = raw[1..raw.len() - 1].to_string();
            }
            let tag = normalize_tag(&raw);
            current_tag = tag.clone();
            if !tag.is_empty() {
                out.entry(tag).or_default();
            }
            continue;
        }

        if !current_tag.is_empty() {
            if let Some(rest) = trimmed.strip_prefix("- ") {
                let mut raw = rest.trim().to_string();
                if raw.starts_with('`') && raw.ends_with('`') && raw.len() >= 2 {
                    raw = raw[1..raw.len() - 1].to_string();
                }
                let node_id = normalize_node_id(&raw);
                if !node_id.is_empty() {
                    out.entry(current_tag.clone()).or_default().insert(node_id);
                }
                continue;
            }
        }

        if trimmed.starts_with('#') {
            let sep = if trimmed.contains("->") {
                "->"
            } else if trimmed.contains("=>") {
                "=>"
            } else {
                ""
            };
            if !sep.is_empty() {
                let parts = trimmed.splitn(2, sep).collect::<Vec<&str>>();
                if parts.len() == 2 {
                    let tag = normalize_tag(parts[0]);
                    if !tag.is_empty() {
                        let entry = out.entry(tag).or_default();
                        for candidate in parts[1].split(',') {
                            let node_id = normalize_node_id(candidate);
                            if !node_id.is_empty() {
                                entry.insert(node_id);
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

fn load_tags_index(root: &Path) -> (Vec<String>, HashMap<String, HashSet<String>>) {
    let paths = vec![
        root.join("docs/workspace/TAGS_INDEX.md"),
        root.join("client/memory/TAGS_INDEX.md"),
        root.join("memory").join("TAGS_INDEX.md"),
    ];
    let mut source = vec![];
    let mut out: HashMap<String, HashSet<String>> = HashMap::new();
    for p in paths {
        if !p.exists() {
            continue;
        }
        source.push(rel_path(root, &p));
        let parsed = parse_tags_file(&p);
        for (tag, ids) in parsed {
            let entry = out.entry(tag).or_default();
            for id in ids {
                entry.insert(id);
            }
        }
    }
    (source, out)
}

fn tokenize(v: &str) -> Vec<String> {
    let mut normalized = String::with_capacity(v.len());
    for ch in v.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch.is_ascii_whitespace() {
            normalized.push(ch);
        } else {
            normalized.push(' ');
        }
    }
    let mut out = normalized
        .split_whitespace()
        .filter(|token| token.len() >= 2)
        .map(|token| token.to_string())
        .collect::<Vec<String>>();
    out = dedupe_sorted(out);
    out
}

fn normalize_vector(values: &[f32]) -> Vec<f32> {
    if values.is_empty() {
        return vec![];
    }
    let mut out = values
        .iter()
        .map(|value| if value.is_finite() { *value } else { 0.0f32 })
        .collect::<Vec<f32>>();
    let norm = out
        .iter()
        .fold(0.0f32, |acc, value| acc + (*value * *value))
        .sqrt();
    if norm > 0.0 {
        for value in out.iter_mut() {
            *value /= norm;
        }
    }
    out
}

fn hash_token_slot(token: &str, salt: u64, dims: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    salt.hash(&mut hasher);
    (hasher.finish() as usize) % dims.max(1)
}

fn vectorize_text(text: &str, dims: usize) -> Vec<f32> {
    if dims == 0 {
        return vec![];
    }
    let mut vec = vec![0.0f32; dims];
    let tokens = tokenize(text);
    if tokens.is_empty() {
        return vec;
    }
    for token in tokens {
        let idx = hash_token_slot(&token, 0, dims);
        let sign_idx = hash_token_slot(&token, 1, dims);
        let sign = if sign_idx.is_multiple_of(2) {
            1.0f32
        } else {
            -1.0f32
        };
        let weight = 1.0f32 + ((token.len().min(24) as f32) / 24.0f32);
        vec[idx] += sign * weight;
    }
    normalize_vector(&vec)
}

fn embedding_text_for_entry(entry: &IndexEntry) -> String {
    format!(
        "{} {} {} {}",
        entry.node_id,
        entry.uid,
        entry.summary,
        entry.tags.join(" ")
    )
}

fn build_entry_embedding(entry: &IndexEntry, dims: usize) -> Vec<f32> {
    vectorize_text(&embedding_text_for_entry(entry), dims)
}

fn parse_tag_filters(raw: &str) -> Vec<String> {
    let mut out = vec![];
    for token in raw.split(',') {
        let tag = normalize_tag(token);
        if !tag.is_empty() {
            out.push(tag);
        }
    }
    dedupe_sorted(out)
}

fn score_entry(
    entry: &IndexEntry,
    query_tokens: &[String],
    tag_filters: &[String],
    tag_node_ids: &HashSet<String>,
) -> (i64, Vec<String>) {
    let mut score: i64 = 0;
    let mut reasons: BTreeSet<String> = BTreeSet::new();

    if !tag_filters.is_empty() {
        let overlap = entry
            .tags
            .iter()
            .filter(|t| tag_filters.contains(t))
            .count() as i64;
        if overlap > 0 {
            score += overlap * 6;
            reasons.insert("tag_match".to_string());
        }
        if tag_node_ids.contains(&entry.node_id) {
            score += 4;
            reasons.insert("tag_index_match".to_string());
        }
    }

    let node_lower = entry.node_id.to_lowercase();
    let summary_lower = entry.summary.to_lowercase();
    let tags_lower = entry.tags.join(" ").to_lowercase();
    let file_lower = entry.file_rel.to_lowercase();
    for tok in query_tokens {
        if node_lower == *tok {
            score += 8;
        } else if node_lower.contains(tok) {
            score += 4;
        }
        if summary_lower.contains(tok) {
            score += 3;
        }
        if tags_lower.contains(tok) {
            score += 2;
        }
        if file_lower.contains(tok) {
            score += 1;
        }
    }
    if !query_tokens.is_empty() && score > 0 {
        reasons.insert("query_match".to_string());
    }
    (score, reasons.into_iter().collect::<Vec<String>>())
}

fn parse_clamped_usize(raw: &str, min: usize, max: usize, fallback: usize) -> usize {
    let parsed = raw.parse::<usize>().unwrap_or(fallback);
    parsed.clamp(min, max)
}

fn excerpt_lines(text: &str, lines: usize) -> String {
    if lines == 0 {
        return String::new();
    }
    text.lines().take(lines).collect::<Vec<&str>>().join("\n")
}

fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(Default)]
struct RuntimeIndexBundle {
    index_sources: Vec<String>,
    tag_sources: Vec<String>,
    entries: Vec<IndexEntry>,
    tag_map: HashMap<String, HashSet<String>>,
    embeddings: HashMap<String, Vec<f32>>,
    sqlite_path: Option<String>,
    sqlite_sync_rows: usize,
    sqlite_sync_applied: bool,
}

fn build_tag_map_from_entries(entries: &[IndexEntry]) -> HashMap<String, HashSet<String>> {
    let mut out: HashMap<String, HashSet<String>> = HashMap::new();
    for entry in entries {
        for raw_tag in &entry.tags {
            let tag = normalize_tag(raw_tag);
            if tag.is_empty() {
                continue;
            }
            out.entry(tag).or_default().insert(entry.node_id.clone());
        }
    }
    out
}
