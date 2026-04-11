fn run_fetch(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "research_plane_contract",
            "fetch_modes": ["http","stealth","browser","auto"],
            "protection_signals": ["captcha","cloudflare","bot detected","access denied"]
        }),
    );
    let policy = load_json_or(
        root,
        POLICY_PATH,
        json!({
            "version": "v1",
            "kind": "research_plane_policy",
            "default_mode": "auto",
            "timeouts": {"fetch_ms": 12000}
        }),
    );
    let url = clean(parsed.flags.get("url").cloned().unwrap_or_default(), 2000);
    let mode = parsed
        .flags
        .get("mode")
        .map(|v| v.to_ascii_lowercase())
        .or_else(|| {
            if parse_bool(parsed.flags.get("stealth"), false) {
                Some("stealth".to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            policy
                .get("default_mode")
                .and_then(Value::as_str)
                .unwrap_or("auto")
                .to_ascii_lowercase()
        });
    let timeout_ms = parse_u64(
        parsed.flags.get("timeout-ms"),
        policy
            .get("timeouts")
            .and_then(|v| v.get("fetch_ms"))
            .and_then(Value::as_u64)
            .unwrap_or(12_000),
    )
    .clamp(1_000, 120_000);
    let max_bytes = parse_u64(parsed.flags.get("max-bytes"), 400_000).clamp(1_024, 4_000_000);
    let mut errors = Vec::<String>::new();

    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("research_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "research_plane_contract"
    {
        errors.push("research_contract_kind_invalid".to_string());
    }
    if policy
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("research_policy_version_must_be_v1".to_string());
    }
    if url.is_empty() {
        errors.push("missing_url".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "research_plane_fetch",
            "errors": errors
        });
    }

    let fetched = fetch_auto(
        root,
        &url,
        &mode,
        timeout_ms,
        max_bytes as usize,
        &policy,
        &contract,
        strict,
    );
    let ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false);
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "research_plane_fetch",
        "lane": "core/layer0/ops",
        "url": url,
        "mode_requested": mode,
        "mode_selected": fetched.get("selected_mode").cloned().unwrap_or(Value::String("unknown".to_string())),
        "status": fetched.get("status").cloned().unwrap_or(Value::Number(0_u64.into())),
        "protected": fetched.get("protected").cloned().unwrap_or(Value::Bool(false)),
        "attempts": fetched.get("attempts").cloned().unwrap_or(Value::Array(Vec::new())),
        "safety_plane_receipts": fetched.get("safety_plane_receipts").cloned().unwrap_or(Value::Array(Vec::new())),
        "body_sha256": fetched.get("body_sha256").cloned().unwrap_or(Value::Null),
        "body_preview": fetched
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .chars()
            .take(220)
            .collect::<String>(),
        "error": fetched.get("error").cloned().unwrap_or(Value::Null),
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-001.1",
                "claim": "multi_mode_fetcher_switches_http_stealth_browser_based_on_protection_signals",
                "evidence": {
                    "mode_selected": fetched.get("selected_mode").cloned().unwrap_or(Value::Null),
                    "attempt_count": fetched.get("attempts").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)
                }
            },
            {
                "id": "V6-RESEARCH-001.5",
                "claim": "stealth_and_browser_paths_are_safety_plane_routed_with_deterministic_receipts",
                "evidence": {
                    "safety_receipt_count": fetched.get("safety_plane_receipts").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)
                }
            }
        ]
    })
}

fn decode_html_payload(parsed: &ParsedArgs, root: &Path) -> Result<String, String> {
    if let Some(raw) = parsed.flags.get("html") {
        return Ok(raw.to_string());
    }
    if let Some(raw_b64) = parsed.flags.get("html-base64") {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("decode_html_base64_failed:{err}"))?;
        return Ok(String::from_utf8_lossy(&bytes).to_string());
    }
    if let Some(rel) = parsed.flags.get("html-path") {
        let path = if Path::new(rel).is_absolute() {
            PathBuf::from(rel)
        } else {
            root.join(rel)
        };
        return fs::read_to_string(&path)
            .map_err(|err| format!("read_html_path_failed:{}:{err}", path.display()));
    }
    Err("missing_html_input".to_string())
}

fn normalize_selector_for_match(selector: &str) -> String {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Some(xpath) = trimmed.strip_prefix("xpath=") {
        return xpath.trim().to_ascii_lowercase();
    }
    trimmed.to_ascii_lowercase()
}

fn html_attribute_values<'a>(html_lc: &'a str, attribute: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    for quote in ['"', '\''] {
        let needle = format!("{attribute}={quote}");
        let mut start = 0usize;
        while let Some(offset) = html_lc[start..].find(&needle) {
            let value_start = start + offset + needle.len();
            let Some(value_end_rel) = html_lc[value_start..].find(quote) else {
                break;
            };
            let value_end = value_start + value_end_rel;
            out.push(&html_lc[value_start..value_end]);
            start = value_end.saturating_add(1);
        }
    }
    out
}

fn html_class_contains(html_lc: &str, class_name: &str) -> bool {
    html_attribute_values(html_lc, "class")
        .into_iter()
        .any(|value| value.split_whitespace().any(|token| token == class_name))
}

fn selector_exists(html_lc: &str, selector: &str) -> bool {
    let sel = normalize_selector_for_match(selector);
    if sel.is_empty() {
        return false;
    }
    if let Some(id) = sel.strip_prefix('#') {
        return html_lc.contains(&format!("id=\"{}\"", id))
            || html_lc.contains(&format!("id='{}'", id));
    }
    if let Some(class) = sel.strip_prefix('.') {
        return html_class_contains(html_lc, class);
    }
    if let Some(xpath) = sel.strip_prefix("//") {
        let tag = xpath
            .split(['[', '/', ' '])
            .next()
            .unwrap_or_default()
            .trim();
        if !tag.is_empty() {
            return html_lc.contains(&format!("<{}", tag));
        }
    }
    html_lc.contains(&format!("<{}", sel))
}

fn token_similarity(left: &str, right: &str) -> f64 {
    let mut left_counts = BTreeMap::<char, u64>::new();
    for ch in left.chars() {
        if ch.is_ascii_alphanumeric() {
            *left_counts.entry(ch).or_insert(0) += 1;
        }
    }
    let mut right_counts = BTreeMap::<char, u64>::new();
    for ch in right.chars() {
        if ch.is_ascii_alphanumeric() {
            *right_counts.entry(ch).or_insert(0) += 1;
        }
    }
    let mut intersection = 0_u64;
    let mut union = 0_u64;
    let mut keys = left_counts
        .keys()
        .chain(right_counts.keys())
        .copied()
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    for key in keys {
        let l = left_counts.get(&key).copied().unwrap_or(0);
        let r = right_counts.get(&key).copied().unwrap_or(0);
        intersection += l.min(r);
        union += l.max(r);
    }
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn run_recover_selectors(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "research_plane_contract",
            "selector_recovery_order": ["css", "xpath", "text", "similarity"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("research_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "research_plane_contract"
    {
        errors.push("research_contract_kind_invalid".to_string());
    }
    let html = match decode_html_payload(parsed, root) {
        Ok(v) => v,
        Err(err) => {
            errors.push(err);
            return json!({
                "ok": false,
                "strict": strict,
                "type": "research_plane_selector_recovery",
                "errors": errors
            });
        }
    };
    let html_lc = html.to_ascii_lowercase();
    let mut selectors = parsed
        .flags
        .get("selectors")
        .map(|v| {
            v.split(',')
                .map(|part| clean(part, 160))
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if selectors.is_empty() {
        if let Some(single) = parsed.flags.get("selector").map(|v| clean(v, 160)) {
            if !single.is_empty() {
                selectors.push(single);
            }
        }
    }
    let target_text = clean(
        parsed.flags.get("target-text").cloned().unwrap_or_default(),
        240,
    );
    let mut recovery = Vec::<Value>::new();
    let mut recovered_selector = Value::Null;
    let mut recovered_strategy = "none".to_string();

    for selector in &selectors {
        let ok = selector_exists(&html_lc, selector);
        recovery.push(json!({
            "strategy": "css_or_xpath",
            "selector": selector,
            "ok": ok
        }));
        if ok && recovered_selector.is_null() {
            recovered_selector = Value::String(selector.clone());
            recovered_strategy = "css_or_xpath".to_string();
        }
    }

    if recovered_selector.is_null() && !target_text.is_empty() {
        let text_ok = html_lc.contains(&target_text.to_ascii_lowercase());
        recovery.push(json!({
            "strategy": "text",
            "selector": target_text,
            "ok": text_ok
        }));
        if text_ok {
            recovered_selector = Value::String(target_text.clone());
            recovered_strategy = "text".to_string();
        }
    }

    if recovered_selector.is_null() && !selectors.is_empty() {
        let mut best_score = 0.0_f64;
        let mut best = String::new();
        for selector in &selectors {
            let score = token_similarity(selector, &html_lc);
            if score > best_score {
                best_score = score;
                best = selector.clone();
            }
        }
        recovery.push(json!({
            "strategy": "similarity",
            "selector": best,
            "score": (best_score * 1000.0).round() / 1000.0
        }));
        if best_score >= 0.15 {
            recovered_selector = Value::String(best);
            recovered_strategy = "similarity".to_string();
        }
    }

    if recovered_selector.is_null() {
        errors.push("selector_recovery_failed".to_string());
    }
    let ok = !recovered_selector.is_null() && errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "research_plane_selector_recovery",
        "lane": "core/layer0/ops",
        "selector_count": selectors.len(),
        "target_text": target_text,
        "recovered_selector": recovered_selector,
        "recovered_strategy": recovered_strategy,
        "steps": recovery,
        "errors": errors,
        "contract_path": CONTRACT_PATH,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-001.2",
                "claim": "selector_recovery_falls_back_css_xpath_text_similarity",
                "evidence": {
                    "recovered_strategy": recovered_strategy
                }
            }
        ]
    })
}

fn url_domain(url: &str) -> String {
    if url.starts_with("file://") {
        return "file".to_string();
    }
    let cleaned = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("unknown");
    clean(cleaned, 120).to_ascii_lowercase()
}

fn parse_seed_urls(parsed: &ParsedArgs) -> Vec<String> {
    let mut out = parsed
        .flags
        .get("seed-urls")
        .map(|v| {
            v.split(',')
                .map(|part| clean(part, 2000))
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if out.is_empty() {
        if let Some(url) = parsed.flags.get("seed-url").map(|v| clean(v, 2000)) {
            if !url.is_empty() {
                out.push(url);
            }
        }
    }
    let mut seen = std::collections::BTreeSet::<String>::new();
    out.retain(|url| seen.insert(url.clone()));
    out
}
