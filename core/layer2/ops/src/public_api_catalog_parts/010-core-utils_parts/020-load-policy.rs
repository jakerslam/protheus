
fn load_policy(root: &Path, argv: &[String]) -> Policy {
    let policy_path = resolve_path(root, parse_flag(argv, "policy"), DEFAULT_POLICY_REL);
    let raw = read_json(&policy_path).unwrap_or_else(|| json!({}));
    let strict = parse_bool(
        parse_flag(argv, "strict"),
        raw.get("strict_fail_closed")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    );
    let max_age_days = parse_f64(parse_flag(argv, "max-age-days"))
        .or_else(|| {
            raw.pointer("/freshness/max_age_days")
                .and_then(Value::as_f64)
        })
        .unwrap_or(DEFAULT_FRESHNESS_DAYS)
        .clamp(1.0, 3650.0);
    let min_sync_actions = parse_usize(
        parse_flag(argv, "min-sync-actions").or_else(|| {
            raw.pointer("/sync/min_actions")
                .and_then(Value::as_u64)
                .map(|v| v.to_string())
        }),
        DEFAULT_MIN_SYNC_ACTIONS,
        1,
        100_000,
    );
    let state_path = resolve_path(
        root,
        parse_flag(argv, "state-path").or_else(|| {
            raw.pointer("/outputs/state_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        }),
        DEFAULT_STATE_REL,
    );
    let history_path = resolve_path(
        root,
        parse_flag(argv, "history-path").or_else(|| {
            raw.pointer("/outputs/history_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        }),
        DEFAULT_HISTORY_REL,
    );
    let source_catalog_path = parse_flag(argv, "catalog-path")
        .or_else(|| {
            raw.pointer("/source/catalog_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(PathBuf::from);
    Policy {
        strict,
        max_age_days,
        min_sync_actions,
        state_path,
        history_path,
        source_catalog_path,
    }
}

fn default_state() -> Value {
    json!({
        "version": "1.0",
        "synced_epoch_ms": 0,
        "last_verified_epoch_ms": 0,
        "source_ref": "",
        "actions": [],
        "connections": [],
        "workflows": [],
        "recent_events": []
    })
}

fn load_state(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(default_state());
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("state_read_failed:{e}"))?;
    let mut parsed =
        serde_json::from_str::<Value>(&raw).map_err(|e| format!("state_parse_failed:{e}"))?;
    if !parsed.is_object() {
        parsed = default_state();
    }
    for key in ["actions", "connections", "workflows", "recent_events"] {
        if parsed.get(key).and_then(Value::as_array).is_none() {
            parsed[key] = Value::Array(Vec::new());
        }
    }
    Ok(parsed)
}

fn save_state(path: &Path, state: &Value) -> Result<(), String> {
    write_json_atomic(path, state)
}

fn event(kind: &str, detail: Value) -> Value {
    json!({
        "ts_epoch_ms": now_epoch_ms(),
        "ts": now_iso(),
        "kind": kind,
        "detail": detail
    })
}

fn push_event(state: &mut Value, kind: &str, detail: Value) {
    let rows = state
        .get_mut("recent_events")
        .and_then(Value::as_array_mut)
        .expect("recent_events array ensured");
    rows.push(event(kind, detail));
    if rows.len() > 100 {
        let excess = rows.len() - 100;
        rows.drain(0..excess);
    }
}

fn builtin_actions(now_ms: u64) -> Vec<Value> {
    vec![
        json!({
            "id": "github.issues.create",
            "platform": "github",
            "title": "Create GitHub Issue",
            "description": "Create an issue in a repository.",
            "method": "POST",
            "url": "https://api.github.com/repos/{owner}/{repo}/issues",
            "parameters": {"required":["owner","repo","title"],"optional":["body","labels","assignees"]},
            "auth": {"type":"oauth","scope":["repo"]},
            "enforcement_rules": {"rate_limit_per_minute":60},
            "response_schema": {"type":"object","required":["id","html_url","number"]},
            "examples": [{"owner":"protheuslabs","repo":"InfRing","title":"Bug report"}],
            "tags": ["github","issues","tracker"],
            "updated_epoch_ms": now_ms,
            "source": "builtin_seed",
            "verified": true
        }),
        json!({
            "id": "slack.chat.post_message",
            "platform": "slack",
            "title": "Post Slack Message",
            "description": "Send a message to a Slack channel.",
            "method": "POST",
            "url": "https://slack.com/api/chat.postMessage",
            "parameters": {"required":["channel","text"],"optional":["thread_ts","blocks"]},
            "auth": {"type":"oauth","scope":["chat:write"]},
            "enforcement_rules": {"rate_limit_per_minute":50},
            "response_schema": {"type":"object","required":["ok","ts"]},
            "examples": [{"channel":"#alerts","text":"deploy complete"}],
            "tags": ["slack","chat"],
            "updated_epoch_ms": now_ms,
            "source": "builtin_seed",
            "verified": true
        }),
        json!({
            "id": "gmail.messages.send",
            "platform": "gmail",
            "title": "Send Gmail Message",
            "description": "Send an email using Gmail API.",
            "method": "POST",
            "url": "https://gmail.googleapis.com/gmail/v1/users/me/messages/send",
            "parameters": {"required":["to","subject","body"],"optional":["cc","bcc","attachments"]},
            "auth": {"type":"oauth","scope":["gmail.send"]},
            "enforcement_rules": {"rate_limit_per_minute":30},
            "response_schema": {"type":"object","required":["id","threadId"]},
            "examples": [{"to":"ops@example.com","subject":"Status","body":"All green"}],
            "tags": ["gmail","email"],
            "updated_epoch_ms": now_ms,
            "source": "builtin_seed",
            "verified": true
        }),
    ]
}

fn normalize_action(raw: &Value, source: &str, now_ms: u64) -> Option<Value> {
    let id = raw
        .get("id")
        .and_then(Value::as_str)
        .map(clean_id)
        .unwrap_or_default();
    let platform = raw
        .get("platform")
        .and_then(Value::as_str)
        .map(clean_id)
        .unwrap_or_default();
    let url = raw
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if id.is_empty() || platform.is_empty() || url.is_empty() {
        return None;
    }
    let tags = raw
        .get("tags")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(clean_id)
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(json!({
        "id": id,
        "platform": platform,
        "title": raw.get("title").and_then(Value::as_str).unwrap_or("").trim(),
        "description": raw.get("description").and_then(Value::as_str).unwrap_or("").trim(),
        "method": normalize_method(raw.get("method").and_then(Value::as_str).unwrap_or("POST")),
        "url": url,
        "parameters": raw.get("parameters").cloned().unwrap_or_else(|| json!({})),
        "auth": raw.get("auth").cloned().unwrap_or_else(|| json!({})),
        "enforcement_rules": raw.get("enforcement_rules").cloned().or_else(|| raw.get("enforcement").cloned()).unwrap_or_else(|| json!({})),
        "response_schema": raw.get("response_schema").cloned().or_else(|| raw.get("response").cloned()).unwrap_or_else(|| json!({})),
        "examples": raw.get("examples").and_then(Value::as_array).cloned().unwrap_or_default(),
        "tags": tags,
        "updated_epoch_ms": raw.get("updated_epoch_ms").and_then(Value::as_u64).unwrap_or(now_ms),
        "source": raw.get("source").and_then(Value::as_str).unwrap_or(source),
        "verified": raw.get("verified").and_then(Value::as_bool).unwrap_or(true)
    }))
}
