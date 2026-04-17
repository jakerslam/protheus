// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::Utc;
use regex::Regex;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::Path;

use crate::contract_lane_utils as lane_utils;

fn usage() {
    println!("collector-content-kernel commands:");
    println!("  protheus-ops collector-content-kernel extract-entries --payload-base64=<json>");
    println!("  protheus-ops collector-content-kernel extract-json-rows --payload-base64=<json>");
    println!("  protheus-ops collector-content-kernel map-feed-items --payload-base64=<json>");
    println!("  protheus-ops collector-content-kernel map-json-items --payload-base64=<json>");
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(raw, max_len)
}

fn clean_collector_id(payload: &Map<String, Value>) -> String {
    lane_utils::clean_token(
        payload.get("collector_id").and_then(Value::as_str),
        "collector",
    )
}

fn clamp_u64(payload: &Map<String, Value>, key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn value_text(value: Option<&Value>, max_len: usize) -> String {
    let raw = match value {
        Some(Value::String(v)) => v.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    };
    clean_text(Some(&raw), max_len)
}

fn sha16(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(digest)[..16].to_string()
}

fn html_decode(raw: &str) -> String {
    let mut out = raw.to_string();
    if let Ok(cdata_re) = Regex::new(r#"(?is)<!\[CDATA\[(.*?)\]\]>"#) {
        out = cdata_re.replace_all(&out, "$1").to_string();
    }
    out = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x2F;", "/")
        .replace("&#x2f;", "/");
    out
}

fn strip_tags(raw: &str) -> String {
    let without_tags = if let Ok(tags_re) = Regex::new(r#"(?is)<[^>]*>"#) {
        tags_re.replace_all(raw, " ").to_string()
    } else {
        raw.to_string()
    };
    clean_text(Some(&html_decode(&without_tags)), 8_192)
}

fn extract_tag_value(block: &str, tag: &str) -> String {
    let escaped = regex::escape(tag);
    let pattern = format!(r"(?is)<{}\b[^>]*>(.*?)</{}\s*>", escaped, escaped);
    match Regex::new(&pattern) {
        Ok(re) => re
            .captures(block)
            .and_then(|caps| caps.get(1).map(|m| strip_tags(m.as_str())))
            .unwrap_or_default(),
        Err(_) => String::new(),
    }
}

fn extract_tag_attr(block: &str, tag: &str, attr: &str) -> String {
    let escaped_tag = regex::escape(tag);
    let escaped_attr = regex::escape(attr);
    let pattern = format!(
        r#"(?is)<{}\b[^>]*\b{}\s*=\s*"([^"]+)"[^>]*>"#,
        escaped_tag, escaped_attr
    );
    match Regex::new(&pattern) {
        Ok(re) => re
            .captures(block)
            .and_then(|caps| caps.get(1).map(|m| html_decode(m.as_str())))
            .unwrap_or_default(),
        Err(_) => String::new(),
    }
}

fn extract_entries(xml: &str) -> Vec<Value> {
    let mut items: Vec<Value> = Vec::new();

    if let Ok(item_re) = Regex::new(r#"(?is)<item\b.*?</item>"#) {
        for mat in item_re.find_iter(xml) {
            let block = mat.as_str();
            let title = extract_tag_value(block, "title");
            let link = {
                let direct = extract_tag_value(block, "link");
                if direct.is_empty() {
                    extract_tag_value(block, "guid")
                } else {
                    direct
                }
            };
            let description = {
                let desc = extract_tag_value(block, "description");
                if desc.is_empty() {
                    extract_tag_value(block, "content:encoded")
                } else {
                    desc
                }
            };
            let published = {
                let pd = extract_tag_value(block, "pubDate");
                if pd.is_empty() {
                    extract_tag_value(block, "dc:date")
                } else {
                    pd
                }
            };
            if title.is_empty() && link.is_empty() {
                continue;
            }
            items.push(json!({
                "title": clean_text(Some(&title), 220),
                "link": clean_text(Some(&link), 500),
                "description": clean_text(Some(&description), 420),
                "published": clean_text(Some(&published), 120),
            }));
        }
    }

    if let Ok(entry_re) = Regex::new(r#"(?is)<entry\b.*?</entry>"#) {
        for mat in entry_re.find_iter(xml) {
            let block = mat.as_str();
            let title = extract_tag_value(block, "title");
            let link = {
                let href = extract_tag_attr(block, "link", "href");
                if href.is_empty() {
                    extract_tag_value(block, "id")
                } else {
                    href
                }
            };
            let description = {
                let summary = extract_tag_value(block, "summary");
                if summary.is_empty() {
                    extract_tag_value(block, "content")
                } else {
                    summary
                }
            };
            let published = {
                let updated = extract_tag_value(block, "updated");
                if updated.is_empty() {
                    extract_tag_value(block, "published")
                } else {
                    updated
                }
            };
            if title.is_empty() && link.is_empty() {
                continue;
            }
            items.push(json!({
                "title": clean_text(Some(&title), 220),
                "link": clean_text(Some(&link), 500),
                "description": clean_text(Some(&description), 420),
                "published": clean_text(Some(&published), 120),
            }));
        }
    }

    items
}

fn topics_from_payload(payload: &Map<String, Value>) -> Vec<Value> {
    payload
        .get("topics")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| Value::String(clean_text(Some(v), 80)))
                .filter(|v| v.as_str().map(|s| !s.is_empty()).unwrap_or(false))
                .take(8)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn seen_ids_from_payload(payload: &Map<String, Value>) -> HashSet<String> {
    payload
        .get("seen_ids")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| clean_text(Some(v), 120))
                .filter(|s| !s.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

fn finalize_seen_ids(mut seen: HashSet<String>) -> Vec<String> {
    let mut seen_ids = seen.drain().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 2000 {
        let drop_count = seen_ids.len() - 2000;
        seen_ids.drain(0..drop_count);
    }
    seen_ids
}

fn value_topics(raw: Option<&Value>, fallback: &[Value]) -> Vec<Value> {
    raw.and_then(Value::as_array)
        .map(|topics| {
            topics
                .iter()
                .filter_map(Value::as_str)
                .map(|topic| Value::String(clean_text(Some(topic), 80)))
                .filter(|topic| topic.as_str().map(|v| !v.is_empty()).unwrap_or(false))
                .take(8)
                .collect::<Vec<_>>()
        })
        .filter(|topics| !topics.is_empty())
        .unwrap_or_else(|| fallback.to_vec())
}

fn tags_for_row(row: &Map<String, Value>, collector_id: &str) -> Vec<Value> {
    let from_row = row
        .get("tags")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| Value::String(clean_text(Some(v), 80)))
                .filter(|v| v.as_str().map(|s| !s.is_empty()).unwrap_or(false))
                .take(6)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if from_row.is_empty() {
        vec![Value::String(collector_id.to_string())]
    } else {
        from_row
    }
}

fn map_feed_items(payload: &Map<String, Value>) -> Value {
    let collector_id = clean_collector_id(payload);
    let entries = payload
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let bytes_per_entry = clamp_u64(payload, "bytes_per_entry", 64, 64, 1_048_576);
    let topics = topics_from_payload(payload);

    let signal_re = payload
        .get("signal_regex")
        .and_then(Value::as_str)
        .and_then(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Regex::new(&format!("(?i){trimmed}")).ok()
            }
        });

    let mut seen = seen_ids_from_payload(payload);

    let mut items = Vec::<Value>::new();
    for entry in entries {
        if items.len() >= max_items {
            break;
        }
        let obj = match entry.as_object() {
            Some(v) => v,
            None => continue,
        };
        let title = clean_text(obj.get("title").and_then(Value::as_str), 220);
        let url = clean_text(obj.get("link").and_then(Value::as_str), 500);
        if title.is_empty() || url.is_empty() {
            continue;
        }
        let description = clean_text(obj.get("description").and_then(Value::as_str), 420);
        let published = clean_text(obj.get("published").and_then(Value::as_str), 120);
        let id = sha16(&format!("{collector_id}|{url}|{title}"));
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id.clone());
        let signal = signal_re
            .as_ref()
            .map(|re| re.is_match(&format!("{title} {description}")))
            .unwrap_or(false);
        items.push(json!({
            "id": id,
            "collected_at": now_iso(),
            "url": url,
            "title": title,
            "description": description,
            "published_at": if published.is_empty() { Value::Null } else { Value::String(published) },
            "source": collector_id,
            "signal": signal,
            "signal_type": if signal { "high_signal" } else { "feed_item" },
            "topics": topics,
            "tags": [collector_id, if signal { "signal" } else { "watch" }],
            "bytes": bytes_per_entry
        }));
    }

    json!({
        "ok": true,
        "collector_id": collector_id,
        "items": items,
        "seen_ids": finalize_seen_ids(seen),
    })
}

fn map_json_items(payload: &Map<String, Value>) -> Value {
    let collector_id = clean_collector_id(payload);
    let rows = payload
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let default_topics = topics_from_payload(payload);

    let mut seen = seen_ids_from_payload(payload);

    let mut out = Vec::<Value>::new();
    for row in rows {
        if out.len() >= max_items {
            break;
        }
        let obj = match row.as_object() {
            Some(v) => v,
            None => continue,
        };
        let title = clean_text(obj.get("title").and_then(Value::as_str), 220);
        let url = clean_text(obj.get("url").and_then(Value::as_str), 500);
        if title.is_empty() || url.is_empty() {
            continue;
        }
        let id = sha16(&format!("{collector_id}|{url}|{title}"));
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id.clone());

        let signal = obj.get("signal").and_then(Value::as_bool).unwrap_or(false);
        let signal_type = clean_text(obj.get("signal_type").and_then(Value::as_str), 80);
        let topics = value_topics(obj.get("topics"), &default_topics);

        out.push(json!({
            "id": id,
            "collected_at": now_iso(),
            "url": url,
            "title": title,
            "description": clean_text(obj.get("description").and_then(Value::as_str), 420),
            "source": collector_id,
            "signal": signal,
            "signal_type": if signal_type.is_empty() {
                if signal { "high_signal" } else { "feed_item" }
            } else {
                signal_type.as_str()
            },
            "topics": topics,
            "tags": tags_for_row(obj, &collector_id),
            "bytes": clamp_u64(obj, "bytes", 64, 64, 1_048_576),
            "published_at": clean_text(obj.get("published_at").and_then(Value::as_str), 120)
        }));
    }

    json!({
        "ok": true,
        "collector_id": collector_id,
        "items": out,
        "seen_ids": finalize_seen_ids(seen)
    })
}

fn format_size_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "n/a".to_string();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut idx = 0usize;
    while value >= 1024.0 && idx < units.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    if value >= 10.0 || idx == 0 {
        format!("{:.0} {}", value, units[idx])
    } else {
        format!("{:.1} {}", value, units[idx])
    }
}

fn extract_json_rows_huggingface(
    collector_id: &str,
    payload_value: &Value,
    topics: &[Value],
) -> Vec<Value> {
    let rows = payload_value.as_array().cloned().unwrap_or_default();
    rows.into_iter()
        .filter_map(|row| {
            let obj = row.as_object()?;
            let id = clean_text(obj.get("id").and_then(Value::as_str), 80);
            let title = clean_text(obj.get("title").and_then(Value::as_str), 220);
            if title.is_empty() || id.is_empty() {
                return None;
            }
            let url = format!("https://huggingface.co/papers/{}", urlencoding::encode(&id));
            let summary = clean_text(
                obj.get("summary")
                    .and_then(Value::as_str)
                    .or_else(|| obj.get("ai_summary").and_then(Value::as_str)),
                420,
            );
            let upvotes = obj
                .get("upvotes")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0) as u64;
            let github_repo = clean_text(obj.get("githubRepo").and_then(Value::as_str), 500);
            let (signal, signal_type, tags) = if collector_id == "papers_with_code" {
                let mut tags = vec![Value::String("papers_with_code_mirror".to_string())];
                if !github_repo.is_empty() {
                    tags.push(Value::String("code_available".to_string()));
                }
                (
                    !github_repo.is_empty(),
                    if !github_repo.is_empty() {
                        "paper_with_repo"
                    } else {
                        "paper"
                    },
                    tags,
                )
            } else {
                (
                    upvotes >= 20,
                    if upvotes >= 20 {
                        "high_upvote_paper"
                    } else {
                        "paper"
                    },
                    vec![
                        Value::String("huggingface".to_string()),
                        Value::String(format!("upvotes:{upvotes}")),
                    ],
                )
            };
            let published_at = clean_text(obj.get("publishedAt").and_then(Value::as_str), 120);
            let extra_len = if collector_id == "papers_with_code" {
                github_repo.len()
            } else {
                24usize
            };
            Some(json!({
                "title": title,
                "url": url,
                "description": summary,
                "signal": signal,
                "signal_type": signal_type,
                "topics": topics,
                "tags": tags,
                "published_at": published_at,
                "bytes": std::cmp::max(96usize, summary.len() + title.len() + extra_len)
            }))
        })
        .collect::<Vec<_>>()
}

fn extract_json_rows_ollama(payload_value: &Value, topics: &[Value]) -> Vec<Value> {
    let models = payload_value
        .as_object()
        .and_then(|obj| obj.get("models"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let signal_re = Regex::new(r"(?i)(coder|reasoning|instruct|vision|multimodal|agent)").ok();
    models
        .into_iter()
        .filter_map(|row| {
            let obj = row.as_object()?;
            let name = clean_text(
                obj.get("name")
                    .and_then(Value::as_str)
                    .or_else(|| obj.get("model").and_then(Value::as_str)),
                200,
            );
            if name.is_empty() {
                return None;
            }
            let base = name.split(':').next().unwrap_or(name.as_str());
            let url = format!("https://ollama.com/library/{}", urlencoding::encode(base));
            let modified = clean_text(obj.get("modified_at").and_then(Value::as_str), 120);
            let size = obj.get("size").and_then(Value::as_u64).unwrap_or(0);
            let size_text = format_size_bytes(size);
            let description = format!(
                "Model {} ({}) updated {}",
                name,
                size_text,
                if modified.is_empty() {
                    "unknown"
                } else {
                    modified.as_str()
                }
            );
            let signal = signal_re
                .as_ref()
                .map(|re| re.is_match(&name))
                .unwrap_or(false);
            Some(json!({
                "title": name,
                "url": url,
                "description": description,
                "signal": signal,
                "signal_type": "model_release",
                "topics": topics,
                "tags": ["ollama", size_text],
                "published_at": modified,
                "bytes": std::cmp::max(96usize, name.len() + 48)
            }))
        })
        .collect::<Vec<_>>()
}

fn extract_json_rows_openreview(payload_value: &Value, topics: &[Value]) -> Vec<Value> {
    let notes = payload_value
        .as_object()
        .and_then(|obj| obj.get("notes"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let signal_re =
        Regex::new(r"(?i)(agent|llm|reasoning|retrieval|safety|alignment|benchmark)").ok();
    notes
        .into_iter()
        .filter_map(|note| {
            let note_obj = note.as_object()?;
            let id = clean_text(note_obj.get("id").and_then(Value::as_str), 120);
            if id.is_empty() {
                return None;
            }
            let content = note_obj
                .get("content")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let title = content
                .get("title")
                .and_then(Value::as_object)
                .and_then(|v| v.get("value"))
                .map(|v| value_text(Some(v), 240))
                .unwrap_or_default();
            if title.is_empty() {
                return None;
            }
            let abstract_text = content
                .get("abstract")
                .and_then(Value::as_object)
                .and_then(|v| v.get("value"))
                .map(|v| value_text(Some(v), 420))
                .unwrap_or_default();
            let venue = content
                .get("venue")
                .and_then(Value::as_object)
                .and_then(|v| v.get("value"))
                .map(|v| value_text(Some(v), 120))
                .unwrap_or_default();
            let url = format!(
                "https://openreview.net/forum?id={}",
                urlencoding::encode(&id)
            );
            let keywords = content
                .get("keywords")
                .and_then(Value::as_object)
                .and_then(|v| v.get("value"))
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .take(3)
                        .map(|v| value_text(Some(v), 60))
                        .filter(|v| !v.is_empty())
                        .map(Value::String)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let combined = format!("{title} {abstract_text}");
            let signal = signal_re
                .as_ref()
                .map(|re| re.is_match(&combined))
                .unwrap_or(false);
            let published_at = value_text(
                note_obj
                    .get("pdate")
                    .or_else(|| note_obj.get("cdate"))
                    .or_else(|| note_obj.get("mdate")),
                120,
            );
            let tags = {
                let mut out = vec![Value::String("openreview".to_string())];
                out.extend(keywords);
                out
            };
            let description = if !abstract_text.is_empty() {
                abstract_text.clone()
            } else {
                venue
            };
            Some(json!({
                "title": title,
                "url": url,
                "description": description,
                "signal": signal,
                "signal_type": "peer_review_paper",
                "topics": topics,
                "tags": tags,
                "published_at": published_at,
                "bytes": std::cmp::max(96usize, title.len() + abstract_text.len() + 64)
            }))
        })
        .collect::<Vec<_>>()
}

fn extract_json_rows(payload: &Map<String, Value>) -> Value {
    let collector_id = clean_collector_id(payload);
    let payload_value = payload.get("payload").cloned().unwrap_or(Value::Null);
    let topics = topics_from_payload(payload);
    let rows = match collector_id.as_str() {
        "huggingface_papers" | "papers_with_code" => {
            extract_json_rows_huggingface(&collector_id, &payload_value, &topics)
        }
        "ollama_search" => extract_json_rows_ollama(&payload_value, &topics),
        "openreview_venues" => extract_json_rows_openreview(&payload_value, &topics),
        _ => payload_value.as_array().cloned().unwrap_or_default(),
    };
    json!({
        "ok": true,
        "collector_id": collector_id,
        "rows": rows
    })
}

fn dispatch(command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "extract-entries" => {
            let xml = payload.get("xml").and_then(Value::as_str).unwrap_or("");
            Ok(json!({
                "ok": true,
                "entries": extract_entries(xml)
            }))
        }
        "extract-json-rows" => Ok(extract_json_rows(payload)),
        "map-feed-items" => Ok(map_feed_items(payload)),
        "map-json-items" => Ok(map_json_items(payload)),
        _ => Err("collector_content_kernel_unknown_command".to_string()),
    }
}

pub fn run(_root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "collector_content_kernel") {
        Ok(value) => value,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "collector_content_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);

    match dispatch(&command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt("collector_content_kernel", out));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "collector_content_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_entries_parses_rss_and_atom() {
        let xml = r#"
        <rss><channel>
          <item><title>RSS One</title><link>https://example.com/a</link><description>A</description></item>
        </channel></rss>
        <feed>
          <entry><title>Atom One</title><link href="https://example.com/b"/><summary>B</summary></entry>
        </feed>
        "#;
        let out = extract_entries(xml);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn map_feed_items_dedupes_and_marks_signal() {
        let payload = json!({
            "collector_id": "demo",
            "entries": [
                { "title": "Alpha", "link": "https://x/a", "description": "urgent event", "published": "" },
                { "title": "Alpha", "link": "https://x/a", "description": "urgent event", "published": "" }
            ],
            "seen_ids": [],
            "signal_regex": "urgent",
            "topics": ["ops"],
            "max_items": 20,
            "bytes_per_entry": 128
        });
        let out = map_feed_items(lane_utils::payload_obj(&payload));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .and_then(|row| row.get("signal"))
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn extract_json_rows_handles_known_collectors() {
        let hf = extract_json_rows(lane_utils::payload_obj(&json!({
            "collector_id": "huggingface_papers",
            "topics": ["research"],
            "payload": [
                { "id": "abc", "title": "A paper", "summary": "sum", "upvotes": 25, "publishedAt": "2026-03-01" }
            ]
        })));
        assert_eq!(
            hf.get("rows").and_then(Value::as_array).map(|v| v.len()),
            Some(1)
        );

        let ollama = extract_json_rows(lane_utils::payload_obj(&json!({
            "collector_id": "ollama_search",
            "topics": ["ai"],
            "payload": { "models": [{ "name": "qwen:4b", "size": 123456, "modified_at": "2026-03-01" }] }
        })));
        assert_eq!(
            ollama
                .get("rows")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .and_then(|row| row.get("title"))
                .and_then(Value::as_str),
            Some("qwen:4b")
        );
    }
}
