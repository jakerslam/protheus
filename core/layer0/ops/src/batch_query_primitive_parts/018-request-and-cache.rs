const CACHE_REL: &str = "client/runtime/local/state/batch_query/cache.json";
const CACHE_MAX_ENTRIES: usize = 240;
const CACHE_TTL_SUCCESS_SECS: i64 = 30 * 60;
const CACHE_TTL_NO_RESULTS_SECS: i64 = 2 * 60;
const CACHE_MODE_ENV: &str = "INFRING_BATCH_QUERY_CACHE_MODE";
const CACHE_TTL_SUCCESS_ENV: &str = "INFRING_BATCH_QUERY_CACHE_TTL_SUCCESS_SECONDS";
const CACHE_TTL_NO_RESULTS_ENV: &str = "INFRING_BATCH_QUERY_CACHE_TTL_NO_RESULTS_SECONDS";
const CACHE_MAX_ENTRIES_ENV: &str = "INFRING_BATCH_QUERY_CACHE_MAX_ENTRIES";

#[derive(Clone, Debug)]
struct QueryPlanSelection {
    queries: Vec<String>,
    rewrite_set: Vec<String>,
    rewrite_applied: bool,
    rerank_query: String,
    query_plan_source: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BatchQuerySearchScope {
    allowed_domains: Vec<String>,
    exclude_subdomains: bool,
}

impl BatchQuerySearchScope {
    fn is_empty(&self) -> bool {
        self.allowed_domains.is_empty()
    }

    fn to_value(&self) -> Value {
        json!({
            "allowed_domains": self.allowed_domains.clone(),
            "exclude_subdomains": self.exclude_subdomains
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BatchQueryCacheControl {
    mode: String,
    ttl_success_secs: i64,
    ttl_no_results_secs: i64,
    max_entries: usize,
}

impl BatchQueryCacheControl {
    fn read_enabled(&self) -> bool {
        self.mode == "enabled"
    }

    fn write_enabled(&self) -> bool {
        self.mode == "enabled" || self.mode == "refresh"
    }

    fn fresh_status(&self) -> &'static str {
        match self.mode.as_str() {
            "enabled" => "miss",
            "refresh" => "refresh",
            _ => "disabled",
        }
    }
}

fn contains_antibot_marker(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    [
        "unfortunately, bots use duckduckgo too",
        "please complete the following challenge",
        "select all squares containing",
        "error-lite@duckduckgo.com",
        "anomaly-modal",
        "captcha",
        "verify you are human",
        "checking your browser before accessing",
        "cf-challenge",
        "cloudflare ray id",
        "just a moment...",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn contains_web_junk_marker(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if contains_antibot_marker(&lowered) {
        return true;
    }
    [
        "please enable javascript",
        "enable javascript and cookies",
        "access denied",
        "403 forbidden",
        "login required",
        "subscribe to continue",
        "please log in to continue",
        "this content is not available in your region",
        "we use cookies to improve your experience",
        "manage your cookie preferences",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_internal_route_query(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    lowered.contains("tool::")
        || lowered.contains("map `tool::")
        || lowered.contains("supported route")
        || lowered.contains("command-to-route")
}

fn looks_like_domain_list_noise(text: &str) -> bool {
    let cleaned = clean_text(text, 1_600);
    if cleaned.is_empty() {
        return false;
    }
    let domains = extract_domains_from_text(&cleaned, 16);
    if domains.len() < 3 {
        return false;
    }
    let words = cleaned.split_whitespace().count();
    words <= (domains.len() * 3 + 10)
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(CACHE_REL)
}

fn cache_identity_policy(policy: &Value) -> Value {
    let mut batch_query = policy
        .get("batch_query")
        .cloned()
        .unwrap_or(Value::Null);
    if let Some(obj) = batch_query.as_object_mut() {
        obj.remove("cache");
    }
    batch_query
}

fn cache_key(source: &str, query: &str, aperture: &str, policy: &Value) -> String {
    crate::deterministic_receipt_hash(&json!({
        "version": 1,
        "source": source,
        "query": query,
        "aperture": aperture,
        "policy": cache_identity_policy(policy),
    }))
}

fn cache_key_with_query_plan(
    source: &str,
    query: &str,
    aperture: &str,
    policy: &Value,
    query_plan: &[String],
) -> String {
    let normalized_plan = cache_identity_query_plan(query, query_plan);
    if normalized_plan.len() <= 1 {
        return cache_key(source, query, aperture, policy);
    }
    crate::deterministic_receipt_hash(&json!({
        "version": 4,
        "source": source,
        "query": query,
        "aperture": aperture,
        "query_plan": normalized_plan,
        "policy": cache_identity_policy(policy),
    }))
}

fn cache_key_with_query_plan_and_scope(
    source: &str,
    query: &str,
    aperture: &str,
    policy: &Value,
    query_plan: &[String],
    search_scope: &BatchQuerySearchScope,
) -> String {
    let normalized_plan = cache_identity_query_plan(query, query_plan);
    if search_scope.is_empty() && normalized_plan.len() <= 1 {
        return cache_key(source, query, aperture, policy);
    }
    if search_scope.is_empty() {
        return cache_key_with_query_plan(source, query, aperture, policy, query_plan);
    }
    crate::deterministic_receipt_hash(&json!({
        "version": 5,
        "source": source,
        "query": query,
        "aperture": aperture,
        "query_plan": normalized_plan,
        "search_scope": search_scope.to_value(),
        "policy": cache_identity_policy(policy),
    }))
}

fn cache_identity_query_plan(query: &str, query_plan: &[String]) -> Vec<String> {
    let mut dedup = HashSet::<String>::new();
    let mut normalized = Vec::<String>::new();
    for value in query_plan {
        let cleaned = clean_text(value, 600);
        if cleaned.is_empty() {
            continue;
        }
        let key = cleaned.to_ascii_lowercase();
        if dedup.insert(key) {
            normalized.push(cleaned);
        }
    }
    if normalized.is_empty() {
        let cleaned_query = clean_text(query, 600);
        if !cleaned_query.is_empty() {
            normalized.push(cleaned_query);
        }
    }
    normalized
}

fn provider_result_identity(value: &Value) -> String {
    let provider = clean_text(
        value.get("provider").and_then(Value::as_str).unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    let status = clean_text(value.get("status").and_then(Value::as_str).unwrap_or(""), 80)
        .to_ascii_lowercase();
    let error = clean_text(value.get("error").and_then(Value::as_str).unwrap_or(""), 160)
        .to_ascii_lowercase();
    if !provider.is_empty() && !error.is_empty() {
        return crate::deterministic_receipt_hash(&json!({
            "kind": "provider_error",
            "provider": provider,
            "status": status,
            "error": error
        }));
    }

    let content = value
        .get("content_preview")
        .or_else(|| value.get("summary"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 1_200).to_ascii_lowercase())
        .unwrap_or_default();
    if content.split_whitespace().count() >= 8 {
        return crate::deterministic_receipt_hash(&json!({
            "kind": "provider_content",
            "provider": provider,
            "status": status,
            "content": content
        }));
    }

    let stage = clean_text(value.get("stage").and_then(Value::as_str).unwrap_or(""), 120)
        .to_ascii_lowercase();
    let locator = clean_text(
        value.get("locator").and_then(Value::as_str).unwrap_or(""),
        600,
    )
    .to_ascii_lowercase();
    crate::deterministic_receipt_hash(&json!({
        "kind": "provider_locator",
        "provider": provider,
        "stage": stage,
        "status": status,
        "locator": locator
    }))
}

fn dedup_provider_results(provider_results: Vec<Value>) -> (Vec<Value>, usize) {
    let before = provider_results.len();
    let mut seen = HashSet::<String>::new();
    let mut deduped = Vec::<Value>::new();
    for value in provider_results {
        let key = provider_result_identity(&value);
        if seen.insert(key) {
            deduped.push(value);
        }
    }
    let removed = before.saturating_sub(deduped.len());
    (deduped, removed)
}

fn cache_policy_i64(policy: &Value, pointer: &str, default_value: i64, min: i64, max: i64) -> i64 {
    policy
        .pointer(pointer)
        .and_then(Value::as_i64)
        .unwrap_or(default_value)
        .clamp(min, max)
}

fn cache_policy_usize(
    policy: &Value,
    pointer: &str,
    default_value: usize,
    min: usize,
    max: usize,
) -> usize {
    policy
        .pointer(pointer)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(default_value)
        .clamp(min, max)
}

fn env_i64(key: &str, fallback: i64, min: i64, max: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn env_usize(key: &str, fallback: usize, min: usize, max: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn normalize_cache_mode(raw: &str) -> String {
    match clean_text(raw, 40).to_ascii_lowercase().as_str() {
        "refresh" | "fresh" | "write_only" | "write-only" => "refresh".to_string(),
        "disabled" | "disable" | "off" | "none" | "bypass" | "no_cache" | "no-cache" => {
            "disabled".to_string()
        }
        _ => "enabled".to_string(),
    }
}

fn requested_cache_mode(request: &Value) -> Option<String> {
    request
        .get("cache_mode")
        .or_else(|| request.pointer("/cache/mode"))
        .or_else(|| request.pointer("/cache_policy/mode"))
        .and_then(Value::as_str)
        .map(normalize_cache_mode)
}

fn batch_query_cache_control(policy: &Value, request: &Value) -> BatchQueryCacheControl {
    let ttl_success = cache_policy_i64(
        policy,
        "/batch_query/cache/ttl_success_seconds",
        CACHE_TTL_SUCCESS_SECS,
        30,
        7 * 24 * 60 * 60,
    );
    let ttl_no_results = cache_policy_i64(
        policy,
        "/batch_query/cache/ttl_no_results_seconds",
        CACHE_TTL_NO_RESULTS_SECS,
        30,
        24 * 60 * 60,
    );
    let max_entries = cache_policy_usize(
        policy,
        "/batch_query/cache/max_entries",
        CACHE_MAX_ENTRIES,
        1,
        10_000,
    );
    let mode = std::env::var(CACHE_MODE_ENV)
        .ok()
        .map(|raw| normalize_cache_mode(&raw))
        .or_else(|| requested_cache_mode(request))
        .or_else(|| {
            policy
                .pointer("/batch_query/cache/mode")
                .and_then(Value::as_str)
                .map(normalize_cache_mode)
        })
        .unwrap_or_else(|| "enabled".to_string());
    BatchQueryCacheControl {
        mode,
        ttl_success_secs: env_i64(CACHE_TTL_SUCCESS_ENV, ttl_success, 30, 7 * 24 * 60 * 60),
        ttl_no_results_secs: env_i64(CACHE_TTL_NO_RESULTS_ENV, ttl_no_results, 30, 24 * 60 * 60),
        max_entries: env_usize(CACHE_MAX_ENTRIES_ENV, max_entries, 1, 10_000),
    }
}

fn cache_ttl_for_status(status: &str, control: &BatchQueryCacheControl) -> i64 {
    if status == "ok" || status == "partial" {
        control.ttl_success_secs
    } else {
        control.ttl_no_results_secs
    }
}

fn prune_cache_entries(
    entries: &mut Map<String, Value>,
    now_ts: i64,
    max_entries: usize,
) -> usize {
    let before = entries.len();
    entries.retain(|_, entry| entry.get("expires_at").and_then(Value::as_i64).unwrap_or(0) > now_ts);
    if entries.len() > max_entries {
        let mut order = entries
            .iter()
            .map(|(entry_key, entry)| {
                (
                    entry_key.clone(),
                    entry.get("stored_at").and_then(Value::as_i64).unwrap_or(0),
                )
            })
            .collect::<Vec<_>>();
        order.sort_by_key(|(_, stored_at)| *stored_at);
        let drop_count = entries.len().saturating_sub(max_entries);
        for (entry_key, _) in order.into_iter().take(drop_count) {
            entries.remove(&entry_key);
        }
    }
    before.saturating_sub(entries.len())
}

fn load_cached_response(root: &Path, key: &str, control: &BatchQueryCacheControl) -> Option<Value> {
    if !control.read_enabled() {
        return None;
    }
    let path = cache_path(root);
    let mut cache = read_json_or(&path, json!({"version": 1, "entries": {}}));
    let now_ts = chrono::Utc::now().timestamp();
    let mut mutated = false;
    let mut hit = None::<Value>;
    if let Some(entries) = cache.get_mut("entries").and_then(Value::as_object_mut) {
        mutated = prune_cache_entries(entries, now_ts, control.max_entries) > 0;
        if let Some(entry) = entries.get(key) {
            if let Some(response) = entry.get("response") {
                hit = Some(response.clone());
            }
        }
    }
    if mutated {
        let _ = write_json_atomic(&path, &cache);
    }
    hit
}

fn store_cached_response(
    root: &Path,
    key: &str,
    response: &Value,
    status: &str,
    control: &BatchQueryCacheControl,
) {
    if !control.write_enabled() {
        return;
    }
    let path = cache_path(root);
    let mut cache = read_json_or(&path, json!({"version": 1, "entries": {}}));
    let now_ts = chrono::Utc::now().timestamp();
    let mut entries = cache
        .get("entries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    prune_cache_entries(&mut entries, now_ts, control.max_entries);
    let ttl = cache_ttl_for_status(status, control).max(30);
    entries.insert(
        key.to_string(),
        json!({
            "stored_at": now_ts,
            "expires_at": now_ts + ttl,
            "status": status,
            "response": response
        }),
    );
    prune_cache_entries(&mut entries, now_ts, control.max_entries);
    cache["version"] = json!(1);
    cache["entries"] = Value::Object(entries);
    let _ = write_json_atomic(&path, &cache);
}

fn prune_batch_query_cache(root: &Path, control: &BatchQueryCacheControl) -> Value {
    let path = cache_path(root);
    if !path.exists() {
        return json!({
            "ok": true,
            "type": "batch_query_cache_cleanup",
            "cache_path": path.to_string_lossy().to_string(),
            "cache_written": false,
            "before_entries": 0,
            "after_entries": 0,
            "removed_entries": 0,
            "max_entries": control.max_entries,
            "ttl_success_seconds": control.ttl_success_secs,
            "ttl_no_results_seconds": control.ttl_no_results_secs
        });
    }
    let mut cache = read_json_or(&path, json!({"version": 1, "entries": {}}));
    let now_ts = chrono::Utc::now().timestamp();
    let before = cache
        .get("entries")
        .and_then(Value::as_object)
        .map(|entries| entries.len())
        .unwrap_or(0);
    let mut entries = cache
        .get("entries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let removed = prune_cache_entries(&mut entries, now_ts, control.max_entries);
    let after = entries.len();
    cache["version"] = json!(1);
    cache["entries"] = Value::Object(entries);
    let cache_written = removed > 0;
    let write_ok = if cache_written {
        write_json_atomic(&path, &cache).is_ok()
    } else {
        true
    };
    json!({
        "ok": write_ok,
        "type": "batch_query_cache_cleanup",
        "cache_path": path.to_string_lossy().to_string(),
        "cache_written": cache_written,
        "before_entries": before,
        "after_entries": after,
        "removed_entries": removed,
        "max_entries": control.max_entries,
        "ttl_success_seconds": control.ttl_success_secs,
        "ttl_no_results_seconds": control.ttl_no_results_secs
    })
}

pub(crate) fn cleanup_cache(root: &Path) -> Value {
    let policy = load_policy(root);
    let control = batch_query_cache_control(&policy, &json!({}));
    prune_batch_query_cache(root, &control)
}

fn request_query_text(request: &Value, max_len: usize) -> String {
    let direct = clean_text(
        request
            .get("query")
            .or_else(|| request.get("q"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        max_len,
    );
    if !direct.is_empty() {
        return direct;
    }
    request
        .get("queries")
        .and_then(Value::as_array)
        .and_then(|rows| rows.iter().find_map(|row| extract_request_query_row(row, max_len)))
        .unwrap_or_default()
}

fn request_bool_alias(request: &Value, names: &[&str]) -> bool {
    for name in names {
        if let Some(value) = request.get(*name) {
            if let Some(flag) = value.as_bool() {
                return flag;
            }
            if let Some(raw) = value.as_str() {
                let lowered = clean_text(raw, 20).to_ascii_lowercase();
                if matches!(lowered.as_str(), "1" | "true" | "yes" | "on") {
                    return true;
                }
                if matches!(lowered.as_str(), "0" | "false" | "no" | "off") {
                    return false;
                }
            }
        }
    }
    false
}

fn normalize_batch_query_scope_domain(raw: &str) -> Option<String> {
    let mut value = clean_text(raw, 260).to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    for prefix in ["https://", "http://"] {
        if let Some(stripped) = value.strip_prefix(prefix) {
            value = stripped.to_string();
            break;
        }
    }
    if let Some(stripped) = value.strip_prefix("*.") {
        value = stripped.to_string();
    }
    if let Some(stripped) = value.strip_prefix("www.") {
        value = stripped.to_string();
    }
    value = value
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim_matches('.')
        .to_string();
    if value.is_empty() || !value.contains('.') {
        return None;
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
    {
        return None;
    }
    Some(value)
}

fn push_normalized_scope_domain(raw: &str, seen: &mut HashSet<String>, out: &mut Vec<String>) {
    if let Some(domain) = normalize_batch_query_scope_domain(raw) {
        if seen.insert(domain.clone()) {
            out.push(domain);
        }
    }
}

fn collect_scope_domains_from_value(
    value: &Value,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) {
    match value {
        Value::Array(rows) => {
            for row in rows {
                collect_scope_domains_from_value(row, seen, out);
            }
        }
        Value::String(raw) => {
            for part in raw.split([',', ';', '\n', '\t', ' ']) {
                push_normalized_scope_domain(part, seen, out);
            }
        }
        Value::Object(map) => {
            for key in ["domain", "host", "url", "origin"] {
                if let Some(raw) = map.get(key).and_then(Value::as_str) {
                    push_normalized_scope_domain(raw, seen, out);
                }
            }
        }
        _ => {}
    }
}

fn batch_query_search_scope(request: &Value) -> BatchQuerySearchScope {
    let mut seen = HashSet::<String>::new();
    let mut allowed_domains = Vec::<String>::new();
    for key in [
        "allowed_domains",
        "include_domains",
        "allowedDomains",
        "includeDomains",
    ] {
        if let Some(value) = request.get(key) {
            collect_scope_domains_from_value(value, &mut seen, &mut allowed_domains);
            if !allowed_domains.is_empty() {
                break;
            }
        }
    }
    let exclude_subdomains = !allowed_domains.is_empty()
        && request_bool_alias(
            request,
            &[
                "exclude_subdomains",
                "exact_domain_only",
                "excludeSubdomains",
                "exactDomainOnly",
            ],
        );
    BatchQuerySearchScope {
        allowed_domains,
        exclude_subdomains,
    }
}

fn extract_request_query_row(row: &Value, max_len: usize) -> Option<String> {
    let raw = if let Some(value) = row.as_str() {
        Some(value)
    } else {
        row.get("query")
            .or_else(|| row.get("q"))
            .and_then(Value::as_str)
    }?;
    let cleaned = clean_text(raw, max_len);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn max_explicit_queries_for_budget(primary_query: &str, budget: ApertureBudget) -> usize {
    let _ = primary_query;
    budget.max_candidates.clamp(2, 12)
}

fn normalize_requested_queries(
    request: &Value,
    primary_query: &str,
    budget: ApertureBudget,
) -> Vec<String> {
    let mut dedup = HashSet::<String>::new();
    let mut queries = Vec::<String>::new();
    let push_query =
        |value: String, dedup: &mut HashSet<String>, queries: &mut Vec<String>| {
            if value.is_empty() {
                return;
            }
            let key = value.to_ascii_lowercase();
            if dedup.insert(key) {
                queries.push(value);
            }
        };
    let normalized_primary = clean_text(primary_query, 600);
    if !normalized_primary.is_empty() {
        push_query(normalized_primary, &mut dedup, &mut queries);
    }
    let max_queries = max_explicit_queries_for_budget(primary_query, budget);
    if let Some(rows) = request.get("queries").and_then(Value::as_array) {
        for row in rows {
            if queries.len() >= max_queries {
                break;
            }
            if let Some(value) = extract_request_query_row(row, 600) {
                push_query(value, &mut dedup, &mut queries);
            }
        }
    }
    queries
}

fn broad_current_research_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/query_recovery/broad_current_research/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn broad_current_research_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/query_recovery/broad_current_research/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or(4)
        .clamp(1, max_explicit_queries_for_budget("", budget) as u64) as usize
}

fn query_recovery_policy_strings(policy: &Value, pointer: &str, max_len: usize) -> Vec<String> {
    policy
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, max_len))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn query_recovery_policy_strings_with_default(
    policy: &Value,
    pointer: &str,
    max_len: usize,
) -> Vec<String> {
    let configured = query_recovery_policy_strings(policy, pointer, max_len);
    if configured.is_empty() {
        query_recovery_policy_strings(&default_policy(), pointer, max_len)
    } else {
        configured
    }
}

fn broad_current_research_recovery_templates(policy: &Value) -> Vec<String> {
    query_recovery_policy_strings_with_default(
        policy,
        "/batch_query/query_recovery/broad_current_research/templates",
        600,
    )
}

fn broad_current_research_recovery_markers(policy: &Value) -> Vec<String> {
    query_recovery_policy_strings_with_default(
        policy,
        "/batch_query/query_recovery/broad_current_research/intent_markers",
        80,
    )
    .into_iter()
    .map(|row| row.to_ascii_lowercase())
    .collect()
}

fn query_looks_like_broad_current_research(policy: &Value, query: &str) -> bool {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() || !current_web_intent(&cleaned) {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains('"')
        || lowered.contains('`')
    {
        return false;
    }
    let word_count = cleaned.split_whitespace().count();
    let broad_marker = broad_current_research_recovery_markers(policy)
        .iter()
        .any(|marker| lowered.contains(marker));
    broad_marker || word_count <= 8
}

fn expand_query_recovery_template(template: &str, query: &str) -> Option<String> {
    let year = current_year();
    if template.contains("{current_year}") && query.to_ascii_lowercase().contains(&year) {
        return None;
    }
    let expanded = template
        .replace("{query}", query)
        .replace("{current_year}", &year);
    let cleaned = clean_text(&expanded, 600);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn broad_current_research_recovery_queries(
    policy: &Value,
    query: &str,
    budget: ApertureBudget,
) -> Vec<String> {
    if !broad_current_research_recovery_enabled(policy)
        || !query_looks_like_broad_current_research(policy, query)
    {
        return Vec::new();
    }
    let max_queries = broad_current_research_recovery_max_queries(policy, budget);
    let mut dedup = HashSet::<String>::new();
    let mut queries = Vec::<String>::new();
    for template in broad_current_research_recovery_templates(policy) {
        if queries.len() >= max_queries {
            break;
        }
        if let Some(value) = expand_query_recovery_template(&template, query) {
            let key = value.to_ascii_lowercase();
            if dedup.insert(key) {
                queries.push(value);
            }
        }
    }
    if queries.len() <= 1 {
        Vec::new()
    } else {
        queries
    }
}

fn general_research_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/query_recovery/general_research/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn general_research_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/query_recovery/general_research/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or(6)
        .clamp(1, max_explicit_queries_for_budget("", budget) as u64) as usize
}

fn general_research_recovery_templates(policy: &Value) -> Vec<String> {
    query_recovery_policy_strings_with_default(
        policy,
        "/batch_query/query_recovery/general_research/templates",
        600,
    )
}

fn general_research_recovery_markers(policy: &Value) -> Vec<String> {
    query_recovery_policy_strings_with_default(
        policy,
        "/batch_query/query_recovery/general_research/intent_markers",
        80,
    )
    .into_iter()
    .map(|row| row.to_ascii_lowercase())
    .collect()
}

fn query_looks_like_general_research(policy: &Value, query: &str) -> bool {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains('"')
        || lowered.contains('`')
        || looks_like_internal_route_query(&cleaned)
    {
        return false;
    }
    if is_framework_catalog_intent(&cleaned) {
        return false;
    }
    general_research_recovery_markers(policy)
        .iter()
        .any(|marker| lowered.contains(marker))
}

fn general_research_recovery_queries(
    policy: &Value,
    query: &str,
    budget: ApertureBudget,
) -> Vec<String> {
    if !general_research_recovery_enabled(policy)
        || !query_looks_like_general_research(policy, query)
    {
        return Vec::new();
    }
    let max_queries = general_research_recovery_max_queries(policy, budget);
    let mut dedup = HashSet::<String>::new();
    let mut queries = Vec::<String>::new();
    for template in general_research_recovery_templates(policy) {
        if queries.len() >= max_queries {
            break;
        }
        if let Some(value) = expand_query_recovery_template(&template, query) {
            let key = value.to_ascii_lowercase();
            if dedup.insert(key) {
                queries.push(value);
            }
        }
    }
    if queries.len() <= 1 {
        Vec::new()
    } else {
        queries
    }
}

fn quality_gate_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/quality_gate/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn provider_recovery_enabled(policy: &Value) -> bool {
    quality_gate_enabled(policy)
        && policy
            .pointer("/batch_query/quality_gate/provider_recovery/enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn provider_recovery_max_providers(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/quality_gate/provider_recovery/max_providers")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .clamp(1, 6) as usize
}

fn provider_recovery_list(policy: &Value, pointer: &str) -> Vec<String> {
    policy
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 80).to_ascii_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn provider_recovery_providers(policy: &Value, query: &str) -> Vec<String> {
    if !provider_recovery_enabled(policy) {
        return Vec::new();
    }
    let mut dedup = HashSet::<String>::new();
    let mut providers = Vec::<String>::new();
    let mut push_provider = |provider: String| {
        if provider.is_empty() {
            return;
        }
        if dedup.insert(provider.clone()) {
            providers.push(provider);
        }
    };
    if current_web_intent(query) {
        for provider in provider_recovery_list(
            policy,
            "/batch_query/quality_gate/provider_recovery/current_intent_providers",
        ) {
            push_provider(provider);
        }
    }
    for provider in provider_recovery_list(
        policy,
        "/batch_query/quality_gate/provider_recovery/providers",
    ) {
        push_provider(provider);
    }
    providers.truncate(provider_recovery_max_providers(policy));
    providers
}

fn low_confidence_retention_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/result_retention/retain_low_confidence_raw_results")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn low_confidence_retention_max_items(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/result_retention/max_low_confidence_items")
        .and_then(Value::as_u64)
        .unwrap_or(budget.max_evidence as u64)
        .clamp(1, budget.max_candidates.max(1) as u64) as usize
}

fn facet_aware_evidence_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/coverage_aware_evidence/enabled")
        .and_then(Value::as_bool)
        .or_else(|| {
            policy
                .pointer("/batch_query/coverage_aware_query_planning/coverage_buckets/enabled")
                .and_then(Value::as_bool)
        })
        .unwrap_or(false)
}

fn facet_aware_max_facets(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/coverage_aware_evidence/max_facets")
        .or_else(|| policy.pointer("/batch_query/coverage_aware_query_planning/budget/default_max_lanes"))
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, budget.max_candidates.clamp(1, 16) as u64) as usize
}

fn facet_aware_min_terms(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/coverage_aware_evidence/min_facet_terms")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 6) as usize
}

fn second_pass_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/second_pass_recovery/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn second_pass_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/second_pass_recovery/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, budget.max_candidates.clamp(1, 8) as u64) as usize
}

fn second_pass_recovery_templates(policy: &Value) -> Vec<String> {
    policy
        .pointer("/batch_query/second_pass_recovery/templates")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 240))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![
                "{query} source-backed evidence".to_string(),
                "{query} primary or official source".to_string(),
                "{query} independent evaluation or source-backed analysis".to_string(),
            ]
        })
}

fn coverage_gap_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/coverage_gap_recovery/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn coverage_gap_recovery_min_usable_evidence(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/coverage_gap_recovery/min_usable_evidence")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, budget.max_evidence.max(1) as u64) as usize
}

fn coverage_gap_recovery_min_covered_facets(policy: &Value, facet_count: usize, budget: ApertureBudget) -> usize {
    if facet_count == 0 {
        return 0;
    }
    let configured_ratio = policy
        .pointer("/batch_query/coverage_gap_recovery/min_covered_facet_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let ratio_target = ((facet_count as f64) * configured_ratio).ceil() as usize;
    let configured_min = policy
        .pointer("/batch_query/coverage_gap_recovery/min_covered_facets")
        .and_then(Value::as_u64)
        .unwrap_or(2) as usize;
    ratio_target
        .max(configured_min)
        .min(facet_count)
        .min(budget.max_evidence.max(1))
}

fn coverage_gap_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/coverage_gap_recovery/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| second_pass_recovery_max_queries(policy, budget) as u64)
        .clamp(1, budget.max_candidates.clamp(1, 8) as u64) as usize
}

fn coverage_gap_recovery_templates(policy: &Value) -> Vec<String> {
    policy
        .pointer("/batch_query/coverage_gap_recovery/templates")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 320))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "{facet} source-backed evidence".to_string(),
                "{facet} primary or official source".to_string(),
                "{facet} independent analysis evidence".to_string(),
                "{facet} examples reports data".to_string(),
            ]
        })
}

fn second_pass_recovery_queries(
    policy: &Value,
    query: &str,
    existing_queries: &[String],
    budget: ApertureBudget,
) -> Vec<String> {
    if !second_pass_recovery_enabled(policy) {
        return Vec::new();
    }
    let base = clean_text(query, 600);
    if base.is_empty() {
        return Vec::new();
    }
    let mut seen = existing_queries
        .iter()
        .map(|row| clean_text(row, 600).to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut out = Vec::<String>::new();
    for template in second_pass_recovery_templates(policy) {
        let candidate = clean_text(&template.replace("{query}", &base), 600);
        if candidate.is_empty() {
            continue;
        }
        if seen.insert(candidate.to_ascii_lowercase()) {
            out.push(candidate);
        }
        if out.len() >= second_pass_recovery_max_queries(policy, budget) {
            break;
        }
    }
    out
}

fn retrieval_telemetry_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/retrieval_telemetry/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn provider_artifact_count_field(artifact: &Value, key: &str) -> usize {
    artifact.get(key).and_then(Value::as_u64).unwrap_or(0) as usize
}

fn retrieval_telemetry_row(
    query: &str,
    phase: &str,
    rows: &[Candidate],
    issues: &[String],
    artifacts: &[Value],
) -> Value {
    let provider_raw_rows = artifacts
        .iter()
        .map(|artifact| {
            provider_artifact_count_field(artifact, "provider_raw_count").max(
                provider_artifact_count_field(artifact, "provider_candidate_count")
                    .max(provider_artifact_count_field(artifact, "synthesis_candidate_count")),
            )
        })
        .sum::<usize>();
    let synthesis_rows = rows
        .iter()
        .filter(|row| !candidate_is_low_confidence_retained(row))
        .count();
    let low_confidence_rows = rows.len().saturating_sub(synthesis_rows);
    let failure_reasons = issues
        .iter()
        .map(|issue| clean_text(issue, 180))
        .filter(|issue| !issue.is_empty())
        .take(12)
        .collect::<Vec<_>>();
    json!({
        "query": clean_text(query, 600),
        "phase": clean_text(phase, 80),
        "provider_count": artifacts.len(),
        "provider_raw_rows": provider_raw_rows,
        "candidate_rows": rows.len(),
        "synthesis_candidate_rows": synthesis_rows,
        "low_confidence_raw_rows": low_confidence_rows,
        "filtered_or_rejected_rows": issues.len(),
        "failure_reasons": failure_reasons
    })
}

fn resolve_query_plan(
    policy: &Value,
    request: &Value,
    query: &str,
    budget: ApertureBudget,
) -> QueryPlanSelection {
    let explicit_queries = normalize_requested_queries(request, query, budget);
    let explicit_query_pack_used = !explicit_queries.is_empty()
        && (query.is_empty()
            || explicit_queries.len() > 1
            || explicit_queries
                .first()
                .map(|value| !value.eq_ignore_ascii_case(query))
                .unwrap_or(false));
    if explicit_query_pack_used {
        let rerank_query = clean_text(query, 600);
        let rerank_query = if rerank_query.is_empty() {
            explicit_queries
                .first()
                .cloned()
                .unwrap_or_else(|| clean_text(query, 600))
        } else {
            rerank_query
        };
        let rewrite_set = explicit_queries.iter().skip(1).cloned().collect::<Vec<_>>();
        return QueryPlanSelection {
            rewrite_applied: explicit_queries.len() > 1,
            queries: explicit_queries,
            rewrite_set,
            rerank_query,
            query_plan_source: "explicit_request_pack",
        };
    }
    let recovery_queries = general_research_recovery_queries(policy, query, budget);
    if !recovery_queries.is_empty() {
        let rerank_query = recovery_queries
            .first()
            .cloned()
            .unwrap_or_else(|| clean_text(query, 600));
        let rewrite_set = recovery_queries.iter().skip(1).cloned().collect::<Vec<_>>();
        return QueryPlanSelection {
            rewrite_applied: recovery_queries.len() > 1,
            queries: recovery_queries,
            rewrite_set,
            rerank_query,
            query_plan_source: "policy_general_research_recovery",
        };
    }
    let recovery_queries = broad_current_research_recovery_queries(policy, query, budget);
    if !recovery_queries.is_empty() {
        let rerank_query = recovery_queries
            .first()
            .cloned()
            .unwrap_or_else(|| clean_text(query, 600));
        let rewrite_set = recovery_queries.iter().skip(1).cloned().collect::<Vec<_>>();
        return QueryPlanSelection {
            rewrite_applied: recovery_queries.len() > 1,
            queries: recovery_queries,
            rewrite_set,
            rerank_query,
            query_plan_source: "policy_broad_current_research_recovery",
        };
    }
    let queries = cache_identity_query_plan(query, &explicit_queries);
    let rerank_query = queries
        .first()
        .cloned()
        .unwrap_or_else(|| clean_text(query, 600));
    QueryPlanSelection {
        queries,
        rewrite_set: Vec::new(),
        rewrite_applied: false,
        rerank_query,
        query_plan_source: "agent_submitted_single_query",
    }
}
