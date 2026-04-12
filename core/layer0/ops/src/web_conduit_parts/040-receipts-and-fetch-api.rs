fn content_type_is_textual(content_type: &str) -> bool {
    let lowered = clean_text(content_type, 120).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    lowered.starts_with("text/")
        || lowered.contains("json")
        || lowered.contains("xml")
        || lowered.contains("javascript")
        || lowered.contains("yaml")
        || lowered.contains("csv")
}

fn fetch_with_curl_retry(
    url: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    max_attempts: usize,
) -> Value {
    let mut attempts = 0usize;
    let mut best = json!({
        "ok": false,
        "status_code": 0,
        "content_type": "",
        "body": "",
        "stderr": "fetch_not_attempted"
    });
    let target_attempts = max_attempts.clamp(1, 4);
    for idx in 0..target_attempts {
        attempts += 1;
        let ua = DEFAULT_WEB_USER_AGENTS
            .get(idx % DEFAULT_WEB_USER_AGENTS.len())
            .copied()
            .unwrap_or(DEFAULT_WEB_USER_AGENTS[0]);
        let current = fetch_with_curl(url, timeout_ms, max_response_bytes, ua);
        let current_ok = current.get("ok").and_then(Value::as_bool).unwrap_or(false);
        best = current;
        if current_ok {
            break;
        }
        if !is_retryable_fetch_result(&best) || idx + 1 >= target_attempts {
            break;
        }
        let sleep_ms = match idx {
            0 => 180_u64,
            1 => 360_u64,
            _ => 720_u64,
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
    }
    if let Some(obj) = best.as_object_mut() {
        obj.insert("retry_attempts".to_string(), json!(attempts));
        obj.insert("retry_used".to_string(), json!(attempts > 1));
    }
    best
}

fn fetch_serper_with_retry(
    api_key: &str,
    query: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    max_attempts: usize,
    top_k: usize,
) -> Value {
    let mut attempts = 0usize;
    let mut best = json!({
        "ok": false,
        "status_code": 0,
        "content_type": "",
        "body": "",
        "stderr": "serper_not_attempted"
    });
    let target_attempts = max_attempts.clamp(1, 4);
    for idx in 0..target_attempts {
        attempts += 1;
        let ua = DEFAULT_WEB_USER_AGENTS
            .get(idx % DEFAULT_WEB_USER_AGENTS.len())
            .copied()
            .unwrap_or(DEFAULT_WEB_USER_AGENTS[0]);
        let current =
            fetch_serper_with_curl(api_key, query, timeout_ms, max_response_bytes, ua, top_k);
        let current_ok = current.get("ok").and_then(Value::as_bool).unwrap_or(false);
        best = current;
        if current_ok {
            break;
        }
        if !is_retryable_fetch_result(&best) || idx + 1 >= target_attempts {
            break;
        }
        let sleep_ms = match idx {
            0 => 180_u64,
            1 => 360_u64,
            _ => 720_u64,
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
    }
    if let Some(obj) = best.as_object_mut() {
        obj.insert("retry_attempts".to_string(), json!(attempts));
        obj.insert("retry_used".to_string(), json!(attempts > 1));
    }
    best
}

fn build_receipt(
    requested_url: &str,
    policy_decision: &str,
    response_hash: Option<&str>,
    status_code: i64,
    policy_reason: &str,
    error: Option<&str>,
) -> Value {
    let timestamp = crate::now_iso();
    let mut row = json!({
        "type": "web_conduit_receipt",
        "timestamp": timestamp,
        "requested_url": clean_text(requested_url, 2200),
        "domain": extract_domain(requested_url),
        "policy_decision": clean_text(policy_decision, 40),
        "policy_reason": clean_text(policy_reason, 160),
        "status_code": status_code,
        "response_hash": response_hash.unwrap_or(""),
        "error": clean_text(error.unwrap_or(""), 320)
    });
    row["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&row));
    row
}

pub fn api_status(root: &Path) -> Value {
    let (policy, policy_path_value) = load_policy(root);
    let recent = read_recent_receipts(root, 12);
    let denied = recent
        .iter()
        .filter(|row| row.get("policy_decision").and_then(Value::as_str) == Some("deny"))
        .count();
    let last = recent.first().cloned().unwrap_or(Value::Null);
    let default_search_provider_chain = provider_chain_from_request("", &json!({}), &policy);
    let default_fetch_provider_chain = fetch_provider_chain_from_request("", &json!({}), &policy);
    let search_provider_catalog = provider_catalog_snapshot(root, &policy);
    let fetch_provider_catalog = fetch_provider_catalog_snapshot(root, &policy);
    json!({
        "ok": true,
        "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
        "policy_path": policy_path_value.to_string_lossy().to_string(),
        "policy": policy,
        "default_provider_chain": default_search_provider_chain.clone(),
        "default_search_provider_chain": default_search_provider_chain,
        "default_fetch_provider_chain": default_fetch_provider_chain,
        "provider_catalog": search_provider_catalog.clone(),
        "search_provider_catalog": search_provider_catalog,
        "fetch_provider_catalog": fetch_provider_catalog,
        "receipts_total": receipt_count(root),
        "recent_denied": denied,
        "recent_receipts": recent,
        "last_receipt": last
    })
}

pub fn api_providers(root: &Path) -> Value {
    let (policy, policy_path_value) = load_policy(root);
    let default_search_provider_chain = provider_chain_from_request("", &json!({}), &policy);
    let default_fetch_provider_chain = fetch_provider_chain_from_request("", &json!({}), &policy);
    let search_providers = provider_catalog_snapshot(root, &policy);
    let fetch_providers = fetch_provider_catalog_snapshot(root, &policy);
    json!({
        "ok": true,
        "type": "web_conduit_providers",
        "policy_path": policy_path_value.to_string_lossy().to_string(),
        "default_provider_chain": default_search_provider_chain.clone(),
        "default_search_provider_chain": default_search_provider_chain,
        "default_fetch_provider_chain": default_fetch_provider_chain,
        "providers": search_providers.clone(),
        "search_providers": search_providers,
        "fetch_providers": fetch_providers
    })
}

pub fn api_receipts(root: &Path, limit: usize) -> Value {
    json!({
        "ok": true,
        "receipts": read_recent_receipts(root, limit.clamp(1, 200))
    })
}

pub fn api_fetch(root: &Path, request: &Value) -> Value {
    let requested_url = clean_text(
        request
            .get("requested_url")
            .or_else(|| request.get("url"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    let summary_only = request
        .get("summary_only")
        .or_else(|| request.get("summary"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let human_approved = request
        .get("human_approved")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let approval_id = clean_text(
        request
            .get("approval_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let approval_state = approval_state_for_request(root, &approval_id, &requested_url);
    let token_approved = approval_state.as_deref() == Some("approved");
    let effective_human_approved = human_approved || token_approved;
    let (policy, _policy_path_value) = load_policy(root);
    let provider_hint = clean_text(
        request
            .get("provider")
            .or_else(|| request.get("source"))
            .or_else(|| request.get("fetch_provider"))
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        40,
    )
    .to_ascii_lowercase();
    if let Some(unknown_provider) = validate_explicit_fetch_provider_hint(&provider_hint) {
        let receipt = build_receipt(
            &requested_url,
            "deny",
            None,
            0,
            "unknown_fetch_provider",
            Some(&unknown_provider),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "unknown_fetch_provider",
            "requested_url": requested_url,
            "requested_provider": unknown_provider,
            "fetch_provider_catalog": fetch_provider_catalog_snapshot(root, &policy),
            "receipt": receipt
        });
    }
    let fetch_provider_chain = fetch_provider_chain_from_request(&provider_hint, request, &policy);
    let selected_provider = fetch_provider_chain
        .first()
        .cloned()
        .unwrap_or_else(|| "direct_http".to_string());
    let policy_eval = infring_layer1_security::evaluate_web_conduit_policy(
        root,
        &json!({
            "requested_url": requested_url,
            "domain": extract_domain(&requested_url),
            "human_approved": effective_human_approved,
            "requests_last_minute": requests_last_minute(root)
        }),
        &policy,
    );
    let allow = policy_eval
        .get("allow")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision = clean_text(
        policy_eval
            .get("decision")
            .and_then(Value::as_str)
            .unwrap_or("deny"),
        20,
    );
    let reason = clean_text(
        policy_eval
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("policy_denied"),
        180,
    );
    if !allow {
        let approval = if reason == "human_approval_required_for_sensitive_domain" {
            ensure_sensitive_web_approval(root, &requested_url, &policy_eval)
        } else {
            None
        };
        let receipt = build_receipt(
            &requested_url,
            "deny",
            None,
            0,
            &reason,
            Some(if approval.is_some() {
                "approval_required"
            } else {
                "policy_denied"
            }),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "web_conduit_policy_denied",
            "requested_url": requested_url,
            "policy_decision": policy_eval,
            "receipt": receipt,
            "approval_required": approval.is_some(),
            "approval": approval,
            "approval_state": approval_state,
            "retry_with": if reason == "human_approval_required_for_sensitive_domain" {
                json!({
                    "url": requested_url,
                    "approval_id": approval
                        .as_ref()
                        .and_then(|row| row.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or(approval_id.as_str()),
                    "summary_only": summary_only
                })
            } else {
                Value::Null
            }
        });
    }

    let timeout_ms = policy_eval
        .pointer("/policy/timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(9000);
    let max_response_bytes = policy_eval
        .pointer("/policy/max_response_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(350_000) as usize;
    let retry_attempts = policy_eval
        .pointer("/policy/retry_attempts")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 4) as usize;
    let fetched = fetch_with_curl_retry(
        &requested_url,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let fetched_body = fetched.get("body").and_then(Value::as_str).unwrap_or("");
    let content_is_textual = content_type_is_textual(&content_type);
    let content = if content_is_textual {
        clean_html_content(fetched_body, max_response_bytes.min(240_000))
    } else {
        String::new()
    };
    let summary = if content_is_textual {
        summarize_text(&content, 900)
    } else if requested_url.is_empty() {
        format!(
            "Fetched non-text content ({}).",
            if content_type.is_empty() {
                "binary/unknown"
            } else {
                content_type.as_str()
            }
        )
    } else {
        format!(
            "Fetched non-text content from {} ({}).",
            clean_text(&requested_url, 220),
            if content_type.is_empty() {
                "binary/unknown"
            } else {
                content_type.as_str()
            }
        )
    };
    let response_hash = if content.is_empty() {
        String::new()
    } else {
        sha256_hex(&content)
    };
    let materialize_artifact = request
        .get("materialize_artifact")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let artifact = if materialize_artifact {
        persist_artifact(root, &requested_url, &response_hash, &content)
    } else {
        None
    };
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && if content_is_textual {
            !content.is_empty()
        } else {
            status_code >= 200 && status_code < 400
        };
    let error_value = fetched
        .get("stderr")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 320))
        .unwrap_or_default();
    let receipt = build_receipt(
        &requested_url,
        &decision,
        if response_hash.is_empty() {
            None
        } else {
            Some(response_hash.as_str())
        },
        status_code,
        &reason,
        if error_value.is_empty() {
            None
        } else {
            Some(error_value.as_str())
        },
    );
    let _ = append_jsonl(&receipts_path(root), &receipt);

    json!({
        "ok": fetch_ok,
        "type": "web_conduit_fetch",
        "requested_url": requested_url,
        "provider": selected_provider,
        "provider_hint": provider_hint,
        "provider_chain": fetch_provider_chain,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String(String::new()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content.clone()) },
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "response_hash": response_hash,
        "artifact": artifact.clone().unwrap_or(Value::Null),
        "policy_decision": policy_eval,
        "receipt": receipt,
        "epistemic_object": {
            "kind": "web_document",
            "trusted": false,
            "provenance": {
                "source": "web_conduit",
                "requested_url": requested_url,
                "response_hash": response_hash,
                "artifact_id": artifact
                    .as_ref()
                    .and_then(|row| row.get("artifact_id"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "artifact_path": artifact
                    .as_ref()
                    .and_then(|row| row.get("path"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "receipt_hash": receipt.get("receipt_hash").cloned().unwrap_or(Value::Null)
            },
            "verity": {
                "validated": false,
                "checks": [
                    "policy_gate_passed",
                    "content_hash_recorded",
                    "source_marked_untrusted_until_verified"
                ]
            }
        },
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            json!("web_conduit_fetch_failed")
        } else {
            json!(error_value)
        }
    })
}
