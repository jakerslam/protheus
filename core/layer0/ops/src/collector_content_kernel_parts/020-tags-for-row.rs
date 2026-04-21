
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
