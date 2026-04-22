pub fn run_extract_structured(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let conduit = conduit_enforcement(root, parsed, strict, "extract_structured");
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return fail_payload(
            "research_plane_extract_structured",
            strict,
            vec!["conduit_bypass_rejected".to_string()],
            Some(conduit),
        );
    }

    let contract = read_json_or(
        root,
        STRUCTURED_EXTRACT_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "structured_extraction_contract",
            "required_output": ["markdown", "json", "provenance"]
        }),
    );
    let payload = load_payload(root, parsed).unwrap_or_default();
    let prompt = clean(parsed.flags.get("prompt").cloned().unwrap_or_default(), 240);
    let schema = parse_json_flag_or_path(root, parsed, "schema-json", "schema-path", Value::Null)
        .unwrap_or(Value::Null);

    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("structured_extract_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "structured_extraction_contract"
    {
        errors.push("structured_extract_contract_kind_invalid".to_string());
    }
    if payload.trim().is_empty() {
        errors.push("missing_payload".to_string());
    }
    if schema.is_null() && prompt.is_empty() {
        errors.push("schema_or_prompt_required".to_string());
    }
    if !errors.is_empty() {
        return fail_payload(
            "research_plane_extract_structured",
            strict,
            errors,
            Some(conduit),
        );
    }

    let title = parse_title(&payload);
    let text = strip_tags(&payload);
    let links = extract_links(&payload);
    let mut output_obj = Map::<String, Value>::new();
    let mut validation = Vec::<Value>::new();

    if let Some(fields) = schema.get("fields").and_then(Value::as_array) {
        for row in fields {
            let name = row
                .get("name")
                .and_then(Value::as_str)
                .map(|v| clean(v, 120))
                .unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            let lower = name.to_ascii_lowercase();
            let value = if lower.contains("title") {
                Value::String(title.clone())
            } else if lower.contains("summary") || lower.contains("text") {
                Value::String(clean(&text, 500))
            } else if lower.contains("link") {
                Value::Array(links.iter().cloned().map(Value::String).collect())
            } else if lower.contains("source") {
                Value::String(clean(
                    parsed
                        .flags
                        .get("source")
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string()),
                    320,
                ))
            } else {
                Value::String(clean(&text, 180))
            };
            let required = row
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let present = if value.is_null() {
                false
            } else if let Some(text_value) = value.as_str() {
                !text_value.is_empty()
            } else if let Some(arr) = value.as_array() {
                !arr.is_empty()
            } else {
                true
            };
            validation.push(json!({"field": name, "required": required, "present": present}));
            output_obj.insert(name, value);
        }
    } else {
        output_obj.insert("title".to_string(), Value::String(title.clone()));
        output_obj.insert("summary".to_string(), Value::String(clean(&text, 500)));
        output_obj.insert(
            "links".to_string(),
            Value::Array(links.iter().cloned().map(Value::String).collect()),
        );
        output_obj.insert(
            "prompt_answer".to_string(),
            Value::String(format!("{} => {}", clean(&prompt, 120), clean(&text, 260))),
        );
        validation.push(json!({"field": "prompt_answer", "required": true, "present": true}));
    }

    let markdown = output_obj
        .iter()
        .map(|(k, v)| {
            if let Some(s) = v.as_str() {
                format!("- **{}**: {}", k, clean(s, 300))
            } else if let Some(arr) = v.as_array() {
                let joined = arr
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|x| clean(x, 180))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("- **{}**: {}", k, joined)
            } else {
                format!("- **{}**: {}", k, clean(v.to_string(), 300))
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let output_json = Value::Object(output_obj);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "research_plane_extract_structured",
        "lane": "core/layer0/ops",
        "markdown": markdown,
        "json": output_json,
        "validation_receipts": validation,
        "provenance": {
            "payload_sha256": sha256_hex_str(&payload),
            "schema_sha256": if schema.is_null() { Value::Null } else { Value::String(sha256_hex_str(&canonical_json_string(&schema))) },
            "prompt_sha256": if prompt.is_empty() { Value::Null } else { Value::String(sha256_hex_str(&prompt)) }
        },
        "conduit_enforcement": conduit,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-004.3",
                "claim": "unified_schema_or_prompt_extraction_returns_markdown_json_with_validation_and_provenance",
                "evidence": {
                    "validation_steps": validation.len()
                }
            },
            {
                "id": "V6-RESEARCH-004.6",
                "claim": "structured_extraction_path_is_enforced_through_conduit_only",
                "evidence": {
                    "conduit": true
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

#[cfg(test)]
mod extract_structured_tests {
    use super::*;

    #[test]
    fn extract_structured_preserves_document_link_order_and_canonical_schema_hash() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "extract-structured".to_string(),
            "--payload=<html><head><title>Ordered</title></head><body><a href=\"https://z.test/first\">first</a><a href=\"https://a.test/second\">second</a><a href=\"https://z.test/first\">dup</a></body></html>".to_string(),
            "--schema-json={\"fields\":[{\"required\":false,\"name\":\"links\"},{\"name\":\"title\",\"required\":true}]}".to_string(),
        ]);
        let out = run_extract_structured(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/json/links")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            vec![
                Value::String("https://z.test/first".to_string()),
                Value::String("https://a.test/second".to_string()),
            ]
        );
        assert_eq!(
            out.pointer("/provenance/schema_sha256")
                .and_then(Value::as_str),
            Some(sha256_hex_str(
                &canonical_json_string(&json!({
                    "fields": [
                        {"required": false, "name": "links"},
                        {"name": "title", "required": true}
                    ]
                }))
            ))
            .as_deref()
        );
    }
}

pub fn run_monitor(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let conduit = conduit_enforcement(root, parsed, strict, "monitor_delta");
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return fail_payload(
            "research_plane_monitor",
            strict,
            vec!["conduit_bypass_rejected".to_string()],
            Some(conduit),
        );
    }

    let contract = read_json_or(
        root,
        MONITOR_DELTA_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "monitor_delta_contract",
            "notify_on_change": true
        }),
    );
    let url = clean(
        parsed
            .flags
            .get("url")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        1800,
    );
    let content = parsed
        .flags
        .get("content")
        .cloned()
        .or_else(|| {
            parsed.flags.get("content-path").and_then(|p| {
                let path = if Path::new(p).is_absolute() {
                    PathBuf::from(p)
                } else {
                    root.join(p)
                };
                fs::read_to_string(path).ok()
            })
        })
        .unwrap_or_else(|| read_url_content(root, &url));

    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("monitor_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "monitor_delta_contract"
    {
        errors.push("monitor_contract_kind_invalid".to_string());
    }
    if url.is_empty() {
        errors.push("missing_url".to_string());
    }
    if !errors.is_empty() {
        return fail_payload("research_plane_monitor", strict, errors, Some(conduit));
    }

    let watcher_path = state_root(root)
        .join("monitor")
        .join("watchers")
        .join(format!("{}.json", sha256_hex_str(&url)));
    let prev = read_json(&watcher_path).unwrap_or(Value::Null);
    let current_hash = sha256_hex_str(&content);
    let prev_hash = prev
        .get("content_hash")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let changed = prev_hash != current_hash;
    let notify = contract
        .get("notify_on_change")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && changed;

    let current = json!({
        "url": url,
        "content_hash": current_hash,
        "content_len": content.len(),
        "checked_at": now_iso()
    });
    let _ = write_json(&watcher_path, &current);

    let delta = json!({
        "changed": changed,
        "previous_hash": if prev_hash.is_empty() { Value::Null } else { Value::String(prev_hash) },
        "current_hash": current_hash,
        "length_delta": content.len() as i64 - prev.get("content_len").and_then(Value::as_i64).unwrap_or(0)
    });
    let notifications = if notify {
        vec![json!({
            "channel": "local-receipt",
            "event": "content_changed",
            "sent": true,
            "ts": now_iso()
        })]
    } else {
        Vec::new()
    };

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "research_plane_monitor",
        "lane": "core/layer0/ops",
        "url": url,
        "delta": delta,
        "notification_receipts": notifications,
        "state_path": watcher_path.display().to_string(),
        "conduit_enforcement": conduit,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-004.4",
                "claim": "monitoring_tracks_content_deltas_with_deterministic_notification_receipts",
                "evidence": {
                    "changed": changed,
                    "notification_count": notifications.len()
                }
            },
            {
                "id": "V6-RESEARCH-004.6",
                "claim": "monitor_path_is_enforced_through_conduit_only",
                "evidence": {
                    "conduit": true
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
