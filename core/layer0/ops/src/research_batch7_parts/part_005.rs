pub fn run_decode_news_url(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let conduit = conduit_enforcement(root, parsed, strict, "decode_news_url");
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return fail_payload(
            "research_plane_decode_news_url",
            strict,
            vec!["conduit_bypass_rejected".to_string()],
            Some(conduit),
        );
    }

    let contract = read_json_or(
        root,
        NEWS_DECODE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "google_news_decode_contract",
            "decoder_version": "v1"
        }),
    );
    let input_url = clean(
        parsed
            .flags
            .get("url")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        2400,
    );

    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("news_decode_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "google_news_decode_contract"
    {
        errors.push("news_decode_contract_kind_invalid".to_string());
    }
    if input_url.is_empty() {
        errors.push("missing_url".to_string());
    }
    if !errors.is_empty() {
        return fail_payload(
            "research_plane_decode_news_url",
            strict,
            errors,
            Some(conduit),
        );
    }

    let mut decoded = String::new();
    let mut method = "none".to_string();

    if let Some((_, query)) = input_url.split_once('?') {
        for part in query.split('&') {
            let mut chunks = part.splitn(2, '=');
            let key = chunks.next().unwrap_or_default();
            let value = chunks.next().unwrap_or_default();
            if ["url", "u", "q"].contains(&key) {
                let candidate = percent_decode(value);
                if candidate.starts_with("http://") || candidate.starts_with("https://") {
                    decoded = candidate;
                    method = "query_param".to_string();
                    break;
                }
            }
        }
    }

    if decoded.is_empty() {
        let path = input_url.split('?').next().unwrap_or_default().to_string();
        let segments = path
            .split('/')
            .map(|v| clean(v, 1200))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if let Some(token) = segments.last() {
            if let Some(candidate) = decode_b64_candidate(token) {
                decoded = candidate;
                method = "base64_segment".to_string();
            }
        }
    }

    if decoded.is_empty() {
        decoded = input_url.clone();
        method = "fallback_identity".to_string();
    }

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "research_plane_decode_news_url",
        "lane": "core/layer0/ops",
        "input_url": input_url,
        "decoded_url": decoded,
        "decode_method": method,
        "provenance": {
            "decoder_version": contract
                .get("decoder_version")
                .and_then(Value::as_str)
                .unwrap_or("v1"),
            "input_sha256": sha256_hex_str(
                parsed
                    .flags
                    .get("url")
                    .cloned()
                    .or_else(|| parsed.positional.get(1).cloned())
                    .unwrap_or_default()
                    .as_str()
            )
        },
        "conduit_enforcement": conduit,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-006.1",
                "claim": "google_news_obfuscated_urls_decode_to_structured_outputs_with_provenance_receipts",
                "evidence": {
                    "method": method
                }
            },
            {
                "id": "V6-RESEARCH-004.6",
                "claim": "decode_path_is_enforced_through_conduit_only",
                "evidence": {
                    "conduit": true
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conduit_rejects_bypass_in_strict_mode() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "goal-crawl".to_string(),
            "--goal=map memory graph".to_string(),
            "--bypass=1".to_string(),
        ]);
        let out = run_goal_crawl(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert!(out
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|r| r.as_str() == Some("conduit_bypass_rejected")))
            .unwrap_or(false));
    }

    #[test]
    fn decode_news_url_prefers_query_param() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "decode-news-url".to_string(),
            "--url=https://news.google.com/read/ABC?url=https%3A%2F%2Fexample.com%2Fstory"
                .to_string(),
        ]);
        let out = run_decode_news_url(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("decoded_url").and_then(Value::as_str),
            Some("https://example.com/story")
        );
    }
}

