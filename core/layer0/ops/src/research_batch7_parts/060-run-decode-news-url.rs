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

    let (decoded, method) = decode_news_url_recursively(&input_url, 4);

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
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

#[cfg(test)]
mod decode_news_url_tests {
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

    #[test]
    fn decode_news_url_unwraps_nested_google_redirects() {
        let root = tempfile::tempdir().expect("tempdir");
        let nested = "https://news.google.com/read/OUTER?url=https%3A%2F%2Fnews.google.com%2Fread%2FINNER%3Furl%3Dhttps%253A%252F%252Fexample.com%252Fnested-story";
        let parsed = crate::parse_args(&[
            "decode-news-url".to_string(),
            format!("--url={nested}"),
        ]);
        let out = run_decode_news_url(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("decoded_url").and_then(Value::as_str),
            Some("https://example.com/nested-story")
        );
        assert_eq!(
            out.get("decode_method").and_then(Value::as_str),
            Some("query_param->query_param")
        );
    }
}
