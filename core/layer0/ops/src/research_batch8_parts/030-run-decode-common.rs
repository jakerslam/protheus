fn run_decode_common(
    root: &Path,
    parsed: &ParsedArgs,
    strict: bool,
    batch: bool,
    command_type: &str,
) -> Value {
    let conduit = conduit_enforcement(root, parsed, strict, command_type);
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return fail_payload(
            command_type,
            strict,
            vec!["conduit_bypass_rejected".to_string()],
            Some(conduit),
        );
    }

    let (policy, mut errors) = load_decode_policy(root, parsed);
    if strict
        && !policy
            .allowed_proxy_modes
            .iter()
            .any(|v| v == &policy.proxy_mode)
    {
        errors.push("proxy_mode_not_allowed".to_string());
    }
    if policy.max_attempts == 0 {
        errors.push("max_attempts_must_be_positive".to_string());
    }

    let urls = if batch {
        let mut rows = parse_csv_or_file_unique(root, &parsed.flags, "urls", "urls-file", 2400);
        if rows.is_empty() {
            rows.extend(
                parsed
                    .positional
                    .iter()
                    .skip(1)
                    .map(|v| clean(v, 2400))
                    .filter(|v| !v.is_empty()),
            );
            rows.sort();
            rows.dedup();
        }
        rows
    } else {
        vec![clean(
            parsed
                .flags
                .get("url")
                .cloned()
                .or_else(|| parsed.positional.get(1).cloned())
                .unwrap_or_default(),
            2400,
        )]
    };

    if urls.is_empty() || urls.first().map(|v| v.is_empty()).unwrap_or(true) {
        errors.push("missing_url".to_string());
    }
    if !errors.is_empty() {
        return fail_payload(command_type, strict, errors, Some(conduit));
    }

    if batch {
        let continue_on_error = parse_bool(parsed.flags.get("continue-on-error"), true);
        let mut per_item = Vec::<Value>::new();
        let mut succeeded = 0usize;
        let mut failed = 0usize;
        let mut all_policy_attempts = Vec::<Value>::new();

        for (idx, url) in urls.iter().enumerate() {
            let (result, resolver_attempts, mut policy_attempts) =
                decode_with_dual_path(url, &policy);
            for row in &mut policy_attempts {
                row["item_index"] = Value::from(idx as u64);
            }
            all_policy_attempts.extend(policy_attempts.clone());
            let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
            if ok {
                succeeded += 1;
            } else {
                failed += 1;
            }
            per_item.push(json!({
                "index": idx,
                "input_url": url,
                "status": result.get("status").cloned().unwrap_or_else(|| Value::String("unresolved".to_string())),
                "decoded_url": result.get("decoded_url").cloned().unwrap_or(Value::Null),
                "message": result.get("message").cloned().unwrap_or_else(|| Value::String("decode failed".to_string())),
                "decode_method": result.get("decode_method").cloned().unwrap_or(Value::Null),
                "error_taxonomy": result.get("error_taxonomy").cloned().unwrap_or_else(|| json!([])),
                "resolver_attempts": resolver_attempts,
                "policy_attempts": policy_attempts
            }));
            if !ok && !continue_on_error {
                break;
            }
        }

        let mut out = json!({
            "ok": if strict { failed == 0 } else { true },
            "strict": strict,
            "type": command_type,
            "lane": "core/layer0/ops",
            "proxy_mode": policy.proxy_mode,
            "policy_attempts": all_policy_attempts,
            "summary": {
                "requested": urls.len(),
                "processed": per_item.len(),
                "succeeded": succeeded,
                "failed": failed,
                "continue_on_error": continue_on_error
            },
            "items": per_item,
            "conduit_enforcement": conduit,
            "claim_evidence": [
                {
                    "id": "V6-RESEARCH-006.2",
                    "claim": "proxy_aware_decode_rate_limit_governance_emits_deterministic_attempt_receipts",
                    "evidence": {
                        "proxy_mode": policy.proxy_mode,
                        "attempt_receipt_count": all_policy_attempts.len()
                    }
                },
                {
                    "id": "V6-RESEARCH-006.4",
                    "claim": "batch_decode_isolates_failures_and_emits_summary_receipts",
                    "evidence": {
                        "processed": per_item.len(),
                        "failed": failed
                    }
                },
                {
                    "id": "V6-RESEARCH-006.6",
                    "claim": "decode_batch_actions_are_conduit_only_fail_closed",
                    "evidence": {
                        "conduit": true
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        return out;
    }

    let input_url = urls.first().cloned().unwrap_or_default();
    let (result, resolver_attempts, policy_attempts) = decode_with_dual_path(&input_url, &policy);
    let mut out = json!({
        "ok": result.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "strict": strict,
        "type": command_type,
        "lane": "core/layer0/ops",
        "status": result.get("status").cloned().unwrap_or_else(|| Value::String("unresolved".to_string())),
        "input_url": input_url,
        "decoded_url": result.get("decoded_url").cloned().unwrap_or(Value::Null),
        "message": result.get("message").cloned().unwrap_or_else(|| Value::String("decode failed".to_string())),
        "decode_method": result.get("decode_method").cloned().unwrap_or(Value::Null),
        "resolver_attempts": resolver_attempts,
        "error_taxonomy": result.get("error_taxonomy").cloned().unwrap_or_else(|| json!([])),
        "proxy_mode": policy.proxy_mode,
        "policy_attempts": policy_attempts,
        "provenance": {
            "input_sha256": sha256_hex_str(&input_url),
            "decoder_contract": NEWS_DECODE_CONTRACT_PATH
        },
        "conduit_enforcement": conduit,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-006.1",
                "claim": "google_news_obfuscated_urls_decode_to_structured_outputs_with_provenance_receipts",
                "evidence": {
                    "status": result.get("status").cloned().unwrap_or(Value::String("unresolved".to_string()))
                }
            },
            {
                "id": "V6-RESEARCH-006.2",
                "claim": "proxy_aware_decode_rate_limit_governance_emits_deterministic_attempt_receipts",
                "evidence": {
                    "proxy_mode": policy.proxy_mode,
                    "attempt_receipt_count": policy_attempts.len()
                }
            },
            {
                "id": "V6-RESEARCH-006.3",
                "claim": "decode_uses_deterministic_articles_primary_then_rss_fallback_with_error_taxonomy",
                "evidence": {
                    "resolver_attempt_count": resolver_attempts.len(),
                    "resolver_order": resolver_attempts.iter().map(|row| row.get("resolver_path").cloned().unwrap_or(Value::Null)).collect::<Vec<_>>()
                }
            },
            {
                "id": "V6-RESEARCH-006.6",
                "claim": "decode_actions_are_conduit_only_fail_closed",
                "evidence": {
                    "conduit": true
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

pub fn run_decode_news_url(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    run_decode_common(
        root,
        parsed,
        strict,
        false,
        "research_plane_decode_news_url",
    )
}

pub fn run_decode_news_urls(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    run_decode_common(
        root,
        parsed,
        strict,
        true,
        "research_plane_decode_news_urls",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_fallback_uses_continue_on_rss_path() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "decode-news-url".to_string(),
            "--url=https://news.google.com/read/ABC?continue=https%3A%2F%2Fexample.com%2Ffallback"
                .to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_decode_news_url(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("decoded_url").and_then(Value::as_str),
            Some("https://example.com/fallback")
        );
        let attempts = out
            .get("resolver_attempts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let order = attempts
            .iter()
            .filter_map(|row| {
                row.get("resolver_path")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();
        assert!(order.first().map(|v| v.as_str()) == Some("/articles"));
        assert!(order.len() >= 2);
    }

    #[test]
    fn conduit_rejects_bypass_for_batch_decode() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "decode-news-urls".to_string(),
            "--urls=https://news.google.com/read/a".to_string(),
            "--bypass=1".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_decode_news_urls(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert!(out
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("conduit_bypass_rejected")))
            .unwrap_or(false));
    }
}

