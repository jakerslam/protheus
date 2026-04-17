// SPDX-License-Identifier: Apache-2.0
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn is_date_memory_file(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() != 13 {
        return false;
    }
    for (idx, b) in bytes.iter().enumerate() {
        match idx {
            4 | 7 => {
                if *b != b'-' {
                    return false;
                }
            }
            10 => {
                if *b != b'.' {
                    return false;
                }
            }
            11 => {
                if *b != b'm' {
                    return false;
                }
            }
            12 => {
                if *b != b'd' {
                    return false;
                }
            }
            _ => {
                if !b.is_ascii_digit() {
                    return false;
                }
            }
        }
    }
    true
}

fn parse_node_id(chunk: &str) -> Option<String> {
    for line in chunk.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("node_id:") {
            continue;
        }
        let value = trimmed
            .split_once(':')
            .map(|(_, rhs)| rhs.trim())
            .unwrap_or_default();
        if value.is_empty() {
            return None;
        }
        let candidate: String = value
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
            .collect();
        if candidate.is_empty() {
            return None;
        }
        return Some(candidate);
    }
    None
}

fn parse_tags_inline(raw: &str) -> Vec<String> {
    let cleaned = raw
        .replace(['[', ']', '"', '\''], " ")
        .replace(',', " ")
        .replace('\t', " ");
    cleaned
        .split_whitespace()
        .map(|token| token.trim_start_matches('#').to_ascii_lowercase())
        .filter(|token| {
            !token.is_empty()
                && token.chars().all(|ch| {
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '_' | '-')
                })
        })
        .collect()
}

pub fn build_index_stats(root: &str) -> serde_json::Value {
    let memory_dir = Path::new(root).join("memory");
    if !memory_dir.exists() {
        return json!({
          "ok": true,
          "backend_used": "rust",
          "transport": "cli",
          "node_count": 0,
          "tag_count": 0,
          "files_scanned": 0
        });
    }

    let mut files: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&memory_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if is_date_memory_file(&name) {
                files.push(name);
            }
        }
    }
    files.sort();

    let mut seen = BTreeSet::new();
    let mut tags = BTreeSet::new();
    let mut files_scanned: u64 = 0;

    for name in &files {
        let file_path = memory_dir.join(name);
        let text = match fs::read_to_string(&file_path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        files_scanned += 1;
        for chunk in text.split("<!-- NODE -->") {
            if let Some(node_id) = parse_node_id(chunk) {
                seen.insert(format!("{node_id}@client/memory/{name}"));
            }
            for line in chunk.lines() {
                let trimmed = line.trim();
                if !trimmed.starts_with("tags:") {
                    continue;
                }
                let raw = trimmed
                    .split_once(':')
                    .map(|(_, rhs)| rhs.trim())
                    .unwrap_or_default();
                for tag in parse_tags_inline(raw) {
                    tags.insert(tag);
                }
            }
        }
    }

    json!({
      "ok": true,
      "backend_used": "rust",
      "transport": "cli",
      "node_count": seen.len(),
      "tag_count": tags.len(),
      "files_scanned": files_scanned
    })
}
