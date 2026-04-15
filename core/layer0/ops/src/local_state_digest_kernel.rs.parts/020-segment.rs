fn item_for(
    kind: &str,
    date_str: &str,
    title: &str,
    preview: &str,
    topics: &[Value],
    source_path: &str,
) -> Value {
    let safe_kind = kind
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    let url = format!("https://local.workspace/signals/{date_str}/{safe_kind}");
    let id = sha16(&format!("{date_str}|{safe_kind}|{url}"));
    let title_clean = clean_text(Some(title), 180);
    let preview_clean = clean_text(Some(preview), 240);
    let bytes = (title_clean.len() + preview_clean.len() + source_path.len() + 96).min(1024) as u64;
    json!({
        "collected_at": now_iso(),
        "id": id,
        "url": url,
        "title": title_clean,
        "content_preview": preview_clean,
        "topics": topics.iter().take(5).cloned().collect::<Vec<_>>(),
        "bytes": bytes
    })
}

fn collect(payload: &Map<String, Value>, state_dir: &Path) -> Value {
    let started = Utc::now().timestamp_millis();
    let pf = preflight(payload, state_dir);
    if pf.get("ok").and_then(Value::as_bool) != Some(true) {
        let first = pf
            .get("failures")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .cloned()
            .unwrap_or_else(
                || json!({ "code": "local_state_preflight_failed", "message": "unknown" }),
            );
        return json!({
            "ok": false,
            "error": first,
            "preflight": pf
        });
    }

    let date_str = resolve_date(payload);
    let max_items = nested_u64(payload, "max_items").unwrap_or(4).clamp(1, 8) as usize;
    let topics = base_topics(payload);

    let backlog_threshold = payload
        .get("backlog_threshold")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            std::env::var("LOCAL_STATE_BACKLOG_ALERT_THRESHOLD")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(6)
        })
        .max(1);
    let outcome_gap_min = payload
        .get("outcome_gap_accepted_min")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            std::env::var("LOCAL_STATE_OUTCOME_GAP_ACCEPTED_MIN")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(2)
        })
        .max(1);
    let tagging_gap_min = payload
        .get("tagging_gap_accepted_min")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            std::env::var("LOCAL_STATE_TAGGING_GAP_ACCEPTED_MIN")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1)
        })
        .max(1);

    let p = proposal_stats(state_dir, &date_str);
    let d = decision_stats(state_dir, &date_str);
    let g = git_outcome_stats(state_dir, &date_str);
    let o = outage_stats(state_dir);

    let mut candidates = Vec::<Value>::new();

    if o.get("active").and_then(Value::as_bool) == Some(true) {
        candidates.push(item_for(
            "infra_outage",
            &date_str,
            &format!(
                "Stabilize automation infrastructure: outage mode active across {} sensors",
                o.get("failed_transport_eyes")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            &format!(
                "Outage mode has been active since {}. Prioritize resilient transport recovery and deterministic fallback routing.",
                o.get("since").and_then(Value::as_str).unwrap_or("unknown")
            ),
            &topics,
            o.get("path").and_then(Value::as_str).unwrap_or(""),
        ));
    }

    if p.get("open").and_then(Value::as_u64).unwrap_or(0) >= backlog_threshold {
        candidates.push(item_for(
            "proposal_backlog",
            &date_str,
            &format!(
                "Remediate backlog saturation: open={} (threshold={})",
                p.get("open").and_then(Value::as_u64).unwrap_or(0),
                backlog_threshold
            ),
            &format!(
                "Queue backlog exceeded threshold. Snapshot total={}, open={}, resolved={}. Reduce queue pressure with deterministic admission and closeout discipline.",
                p.get("total").and_then(Value::as_u64).unwrap_or(0),
                p.get("open").and_then(Value::as_u64).unwrap_or(0),
                p.get("resolved").and_then(Value::as_u64).unwrap_or(0)
            ),
            &topics,
            p.get("path").and_then(Value::as_str).unwrap_or(""),
        ));
    }

    if d.get("accepted").and_then(Value::as_u64).unwrap_or(0) >= outcome_gap_min
        && d.get("shipped").and_then(Value::as_u64).unwrap_or(0) == 0
    {
        candidates.push(item_for(
            "outcome_gap",
            &date_str,
            &format!(
                "Remediate execution gap: accepted={}, shipped={}",
                d.get("accepted").and_then(Value::as_u64).unwrap_or(0),
                d.get("shipped").and_then(Value::as_u64).unwrap_or(0)
            ),
            &format!(
                "Accepted proposals are not converting to shipped outcomes. no_change={}, reverted={}, recorded={}. Prioritize one accepted proposal to completion with verifiable evidence.",
                d.get("no_change").and_then(Value::as_u64).unwrap_or(0),
                d.get("reverted").and_then(Value::as_u64).unwrap_or(0),
                g.get("outcomes_recorded").and_then(Value::as_u64).unwrap_or(0)
            ),
            &topics,
            d.get("path").and_then(Value::as_str).unwrap_or(""),
        ));
    }

    if d.get("accepted").and_then(Value::as_u64).unwrap_or(0) >= tagging_gap_min
        && g.get("tags_found").and_then(Value::as_u64).unwrap_or(0) == 0
    {
        candidates.push(item_for(
            "tagging_gap",
            &date_str,
            &format!(
                "Increase automation reliability: enforce proposal traceability (accepted={}, git_tags={})",
                d.get("accepted").and_then(Value::as_u64).unwrap_or(0),
                g.get("tags_found").and_then(Value::as_u64).unwrap_or(0)
            ),
            &format!(
                "No proposal:<ID> commit tags were detected for accepted={}. Enforce deterministic proposal tagging to improve shipped outcome attribution.",
                d.get("accepted").and_then(Value::as_u64).unwrap_or(0)
            ),
            &topics,
            g.get("path").and_then(Value::as_str).unwrap_or(""),
        ));
    }

    let mut dedup = Vec::<Value>::new();
    let mut seen_urls = HashSet::<String>::new();
    for item in candidates {
        let url = item
            .as_object()
            .and_then(|o| o.get("url"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if url.is_empty() || !seen_urls.insert(url) {
            continue;
        }
        dedup.push(item);
    }

    let items = dedup.into_iter().take(max_items).collect::<Vec<_>>();
    let bytes = items
        .iter()
        .map(|row| {
            row.as_object()
                .and_then(|o| o.get("bytes"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        })
        .sum::<u64>();
    let duration_ms = (Utc::now().timestamp_millis() - started).max(0) as u64;

    json!({
        "success": true,
        "items": items,
        "duration_ms": duration_ms,
        "requests": 0,
        "bytes": bytes
    })
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    let state_dir = resolve_state_dir(root, payload);
    match command {
        "preflight" => Ok(preflight(payload, &state_dir)),
        "collect" => Ok(collect(payload, &state_dir)),
        _ => Err("local_state_digest_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "local_state_digest_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "local_state_digest_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = payload_obj(&payload);
    match dispatch(root, &command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt("local_state_digest_kernel", out));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "local_state_digest_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn preflight_invalid_budget_fails() {
        let tmp = tempdir().expect("tmpdir");
        let payload = json!({
            "state_dir": tmp.path().display().to_string(),
            "budgets": { "max_items": 0 }
        });
        let out = preflight(payload_obj(&payload), tmp.path());
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn collect_emits_backlog_signal() {
        let tmp = tempdir().expect("tmpdir");
        let state_dir = tmp.path().join("state");
        let date = "2026-03-27";
        let proposals_dir = state_dir.join("sensory").join("proposals");
        fs::create_dir_all(&proposals_dir).expect("mkdir");
        let proposals_path = proposals_dir.join(format!("{date}.json"));
        fs::write(
            &proposals_path,
            serde_json::to_string(&json!({
                "proposals": [
                    {"status":"open"},
                    {"status":"open"},
                    {"status":"open"},
                    {"status":"open"},
                    {"status":"open"},
                    {"status":"open"},
                    {"status":"open"}
                ]
            }))
            .expect("encode"),
        )
        .expect("write");

        let payload = json!({
            "state_dir": state_dir.display().to_string(),
            "date": date,
            "budgets": { "max_items": 4 },
            "backlog_threshold": 6,
            "outcome_gap_accepted_min": 99,
            "tagging_gap_accepted_min": 99
        });
        let out = collect(payload_obj(&payload), &state_dir);
        assert_eq!(out.get("success").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty()),
            Some(true)
        );
    }
}
