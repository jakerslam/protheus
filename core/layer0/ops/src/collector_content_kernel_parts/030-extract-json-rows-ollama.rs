
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
