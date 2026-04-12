const FETCH_CACHE_REL: &str = "client/runtime/local/state/web_conduit/fetch_cache.json";
const FETCH_CACHE_MAX_ENTRIES: usize = 256;

fn default_fetch_cache_state() -> Value {
    json!({
        "version": 1,
        "entries": {}
    })
}

fn fetch_cache_path(root: &Path) -> PathBuf {
    runtime_state_path(root, FETCH_CACHE_REL)
}

pub(crate) fn fetch_cache_key(
    requested_url: &str,
    resolved_url: &str,
    extract_mode: &str,
    max_chars: usize,
    summary_only: bool,
    provider_chain: &[String],
) -> String {
    crate::deterministic_receipt_hash(&json!({
        "version": 1,
        "requested_url": clean_text(requested_url, 2200),
        "resolved_url": clean_text(resolved_url, 2200),
        "extract_mode": clean_text(extract_mode, 24),
        "max_chars": max_chars,
        "summary_only": summary_only,
        "provider_chain": provider_chain
    }))
}

pub(crate) fn load_fetch_cache(root: &Path, key: &str) -> Option<Value> {
    let path = fetch_cache_path(root);
    let mut cache = read_json_or(&path, default_fetch_cache_state());
    let now_ts = Utc::now().timestamp();
    let mut mutated = false;
    let mut hit = None::<Value>;
    if let Some(entries) = cache.get_mut("entries").and_then(Value::as_object_mut) {
        let stale_keys = entries
            .iter()
            .filter_map(|(entry_key, entry)| {
                let expires_at = entry.get("expires_at").and_then(Value::as_i64).unwrap_or(0);
                if expires_at <= now_ts {
                    Some(entry_key.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for stale_key in stale_keys {
            entries.remove(&stale_key);
            mutated = true;
        }
        if let Some(entry) = entries.get_mut(key) {
            if let Some(row) = entry.get("response") {
                hit = Some(row.clone());
            }
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("last_hit_at".to_string(), json!(now_ts));
            }
            mutated = true;
        }
    }
    if mutated {
        let _ = write_json_atomic(&path, &cache);
    }
    hit
}

pub(crate) fn store_fetch_cache(
    root: &Path,
    key: &str,
    response: &Value,
    status: &str,
    ttl_minutes: u64,
) {
    let path = fetch_cache_path(root);
    let mut cache = read_json_or(&path, default_fetch_cache_state());
    let now_ts = Utc::now().timestamp();
    let mut entries = cache
        .get("entries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    entries
        .retain(|_, entry| entry.get("expires_at").and_then(Value::as_i64).unwrap_or(0) > now_ts);
    let fallback_ttl = cache_ttl_for_status(status).max(30);
    let requested_ttl = (ttl_minutes as i64).saturating_mul(60).max(0);
    let ttl = if requested_ttl > 0 {
        requested_ttl
    } else {
        fallback_ttl
    };
    entries.insert(
        key.to_string(),
        json!({
            "stored_at": now_ts,
            "last_hit_at": now_ts,
            "expires_at": now_ts + ttl,
            "status": clean_text(status, 40),
            "response": response
        }),
    );
    if entries.len() > FETCH_CACHE_MAX_ENTRIES {
        let mut order = entries
            .iter()
            .map(|(entry_key, entry)| {
                (
                    entry_key.clone(),
                    entry
                        .get("last_hit_at")
                        .and_then(Value::as_i64)
                        .or_else(|| entry.get("stored_at").and_then(Value::as_i64))
                        .unwrap_or(0),
                )
            })
            .collect::<Vec<_>>();
        order.sort_by_key(|(_, used_at)| *used_at);
        let drop_count = entries.len().saturating_sub(FETCH_CACHE_MAX_ENTRIES);
        for (entry_key, _) in order.into_iter().take(drop_count) {
            entries.remove(&entry_key);
        }
    }
    cache["version"] = json!(1);
    cache["entries"] = Value::Object(entries);
    let _ = write_json_atomic(&path, &cache);
}

#[cfg(test)]
mod fetch_cache_tests {
    use super::*;

    #[test]
    fn fetch_cache_roundtrip_returns_stored_payload() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let key = fetch_cache_key(
            "https://example.com/a",
            "https://example.com/a",
            "markdown",
            4000,
            false,
            &vec!["direct_http".to_string()],
        );
        let response = json!({
            "ok": true,
            "type": "web_conduit_fetch",
            "content": "# Example",
            "cache_status": "miss"
        });
        store_fetch_cache(tmp.path(), &key, &response, "ok", 15);
        let loaded = load_fetch_cache(tmp.path(), &key).expect("cache hit");
        assert_eq!(loaded.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            loaded.get("content").and_then(Value::as_str),
            Some("# Example")
        );
    }
}
