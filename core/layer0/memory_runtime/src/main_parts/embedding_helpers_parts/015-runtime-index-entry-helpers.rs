fn dedupe_tags(tags: &[String]) -> Vec<String> {
    let mut out = tags
        .iter()
        .map(|tag| {
            let mut raw = strip_ticks(tag).to_lowercase();
            while raw.starts_with('#') {
                raw = raw[1..].to_string();
            }
            raw.chars()
                .filter(|ch| {
                    ch.is_ascii_lowercase()
                        || ch.is_ascii_digit()
                        || matches!(ch, '_' | '-' | ':' | '=')
                })
                .collect::<String>()
        })
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<String>>();
    out.sort();
    out.dedup();
    out
}

fn infer_db_memory_kind(tags: &[String]) -> String {
    let normalized = dedupe_tags(tags);
    if normalized
        .iter()
        .any(|tag| tag.starts_with("procedure") || tag == "procedural")
    {
        return "procedural".to_string();
    }
    if normalized
        .iter()
        .any(|tag| matches!(tag.as_str(), "semantic" | "fact" | "knowledge"))
    {
        return "semantic".to_string();
    }
    "episodic".to_string()
}

fn to_db_index_entry(entry: &IndexEntry) -> DbIndexEntry {
    DbIndexEntry {
        node_id: entry.node_id.clone(),
        uid: entry.uid.clone(),
        file_rel: entry.file_rel.clone(),
        summary: entry.summary.clone(),
        tags: dedupe_tags(&entry.tags),
        kind: infer_db_memory_kind(&entry.tags),
    }
}

fn from_db_index_entry(entry: &DbIndexEntry) -> IndexEntry {
    let mut tags = dedupe_tags(&entry.tags);
    if !entry.kind.trim().is_empty() && !tags.iter().any(|tag| tag == &entry.kind) {
        tags.push(entry.kind.clone());
        tags.sort();
        tags.dedup();
    }
    IndexEntry {
        node_id: entry.node_id.clone(),
        uid: entry.uid.clone(),
        file_rel: entry.file_rel.clone(),
        summary: entry.summary.clone(),
        tags,
    }
}
