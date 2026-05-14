fn browser_materialization_config_from_policy(policy: &Value) -> Value {
    policy
        .pointer("/web_conduit/browser_materialization")
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn browser_materialization_request_field_list(
    config: &Value,
    pointer: &str,
    fallback: &[&str],
) -> Vec<String> {
    config
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 120))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| fallback.iter().map(|row| row.to_string()).collect())
}

fn browser_materialization_first_denied_request_field(
    request: &Value,
    denied_fields: &[String],
) -> Option<String> {
    let obj = request.as_object()?;
    denied_fields
        .iter()
        .find(|field| obj.contains_key(field.as_str()))
        .cloned()
}

fn browser_materialization_safety_projection(url: &str, ssrf_guard: &Value) -> Value {
    json!({
        "version": "browser_materialization_url_safety_v1",
        "url": clean_text(url, 2200),
        "ok": ssrf_guard.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "status": ssrf_guard
            .get("url_safety_status")
            .and_then(Value::as_str)
            .unwrap_or("invalid_url"),
        "host": ssrf_guard.get("host").cloned().unwrap_or(Value::Null),
        "error": ssrf_guard.get("error").cloned().unwrap_or(Value::Null),
        "resolved_ip_addrs": ssrf_guard
            .get("resolved_ip_addrs")
            .cloned()
            .unwrap_or_else(|| json!([]))
    })
}

fn browser_materialization_final_url_safety_projection() -> Value {
    json!({
        "version": "browser_materialization_final_url_safety_v1",
        "ok": false,
        "status": "not_observed",
        "final_url": Value::Null,
        "revalidate_after_navigation_required": true,
        "revalidate_before_artifact_creation": true,
        "reason": "Adapter did not execute, so no browser final URL was observed."
    })
}

fn browser_materialization_redact_url_credentials(raw_url: &str) -> String {
    let cleaned = clean_text(raw_url, 2200);
    let scheme = fetch_url_scheme(&cleaned);
    if scheme.is_empty() || !fetch_url_has_credentials(&cleaned) {
        return cleaned;
    }
    let prefix = format!("{scheme}://");
    let Some(rest) = cleaned.strip_prefix(&prefix) else {
        return "[credentialed-url-redacted]".to_string();
    };
    let Some((_, after_credentials)) = rest.split_once('@') else {
        return "[credentialed-url-redacted]".to_string();
    };
    format!("{prefix}[redacted]@{after_credentials}")
}

fn browser_materialization_observed_final_url_safety_projection(
    final_url: &str,
    ssrf_guard: &Value,
) -> Value {
    json!({
        "version": "browser_materialization_final_url_safety_v1",
        "ok": ssrf_guard.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "status": ssrf_guard
            .get("url_safety_status")
            .and_then(Value::as_str)
            .unwrap_or("invalid_url"),
        "final_url": browser_materialization_redact_url_credentials(final_url),
        "host": ssrf_guard.get("host").cloned().unwrap_or(Value::Null),
        "resolved_ip_addrs": ssrf_guard
            .get("resolved_ip_addrs")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "revalidate_after_navigation_required": true,
        "revalidate_before_artifact_creation": true,
        "reason": "Final URL was revalidated by the materialization provider before creating artifact refs."
    })
}

fn browser_materialization_navigation_contract_projection(config: &Value) -> Value {
    let security = config.get("security").cloned().unwrap_or_else(|| json!({}));
    json!({
        "version": "browser_materialization_navigation_contract_v1",
        "source_pattern": "cloakbrowser_one_shot_navigate_wait_capture_close",
        "navigate_once_before_capture": true,
        "wait_until_default": "domcontentloaded",
        "default_timeout_ms": config
            .get("default_timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(30000),
        "max_response_bytes": config
            .get("max_response_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(350000),
        "max_redirects": security
            .get("max_redirects")
            .and_then(Value::as_u64)
            .unwrap_or(8),
        "pre_navigation_url_safety_required": true,
        "final_url_revalidation_required": security
            .get("revalidate_final_url_after_navigation")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "capture_after_final_url_safety_only": true,
        "raw_payload_chat_visible": false
    })
}

fn browser_materialization_readiness_strategy_projection(config: &Value) -> Value {
    let smart_wait = config.get("smart_wait").cloned().unwrap_or_else(|| json!({}));
    json!({
        "version": "browser_materialization_readiness_strategy_v1",
        "source_pattern": "cloakbrowser_smart_dom_settle_wait",
        "strategy": if smart_wait
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true)
        {
            "smart_dom_settle_default"
        } else {
            "bounded_navigation_wait_only"
        },
        "dom_stable_ms": smart_wait
            .get("dom_stable_ms")
            .and_then(Value::as_u64)
            .unwrap_or(1500),
        "max_settle_ms": smart_wait
            .get("max_settle_ms")
            .and_then(Value::as_u64)
            .unwrap_or(15000),
        "polls_dom_growth_not_network_idle_only": true,
        "caller_raw_wait_script_allowed": false,
        "bounded_selector_wait_allowed": true,
        "fallback_on_settle_timeout": "return_low_confidence_materialization_if_content_exists"
    })
}

fn browser_materialization_cleanup_status_projection() -> Value {
    json!({
        "version": "browser_materialization_cleanup_status_v1",
        "status": "not_started",
        "browser_launch_attempted": false,
        "context_created": false,
        "context_close_attempted": false,
        "cleanup_required": false,
        "cleanup_error_chat_visible": false
    })
}

fn browser_materialization_fake_cleanup_status_projection() -> Value {
    json!({
        "version": "browser_materialization_cleanup_status_v1",
        "status": "completed_noop",
        "browser_launch_attempted": false,
        "context_created": false,
        "context_close_attempted": false,
        "cleanup_required": false,
        "cleanup_error_chat_visible": false,
        "fake_provider_no_browser_process": true
    })
}

fn browser_materialization_local_fixture_cleanup_status_projection(status: &str) -> Value {
    json!({
        "version": "browser_materialization_cleanup_status_v1",
        "status": clean_text(status, 80),
        "browser_launch_attempted": false,
        "context_created": false,
        "context_close_attempted": false,
        "cleanup_required": true,
        "cleanup_attempted": true,
        "cleanup_error_chat_visible": false,
        "local_fixture_handle_opened": true,
        "local_fixture_handle_close_attempted": true,
        "raw_fixture_path_chat_visible": false
    })
}

fn browser_materialization_context_contract_projection() -> Value {
    json!({
        "version": "browser_materialization_context_contract_v1",
        "source_pattern": "cloakbrowser_playwright_context_boundary",
        "context_created_by_adapter_only": true,
        "caller_context_options_allowed": false,
        "caller_launch_options_allowed": false,
        "policy_profile_overrides_only": true,
        "normalize_or_reject_context_conflicts": true,
        "context_conflict_fields": [
            "locale",
            "timezone",
            "timezoneId",
            "timezone_id",
            "viewport",
            "userAgent",
            "user_agent",
            "colorScheme"
        ],
        "close_browser_on_context_creation_failure": true,
        "context_close_closes_browser": true,
        "default_viewport_policy_owned": true,
        "caller_viewport_allowed": false,
        "caller_user_agent_allowed": false,
        "caller_color_scheme_allowed": false,
        "locale_timezone_cdp_emulation_allowed": false,
        "locale_timezone_binary_profile_fields_policy_owned": true,
        "geoip_filled_profile_fields_require_proxy_capability": true,
        "generic_context_kwargs_allowed_from_workflow": false,
        "storage_state_requires_session_capability": true,
        "persistent_context_allowed": false,
        "humanized_interaction_allowed": false,
        "proxy_or_geo_profile_allowed_without_separate_admission": false,
        "raw_context_options_chat_visible": false
    })
}

fn browser_materialization_retry_diagnostics_projection(error: &str) -> Value {
    let recommendation = match error {
        "adapter_not_ready" => "satisfy_adapter_readiness_before_retry",
        "browser_adapter_stub_only" => "implement_or_admit_browser_adapter_before_retry",
        "capability_not_enabled" => "admit_capability_before_retry",
        "local_static_fixture_unavailable"
        | "local_static_fixture_url_mismatch"
        | "local_js_rendered_fixture_unavailable"
        | "local_js_rendered_fixture_url_mismatch" => "fix_policy_owned_fixture_before_retry",
        "url_safety_blocked" | "unsafe_caller_control_rejected" => "do_not_retry_without_request_change",
        _ => "no_retry_recommendation",
    };
    json!({
        "version": "browser_materialization_retry_diagnostics_v1",
        "source_pattern": "cloakbrowser_classified_strategy_retry",
        "hidden_retry_executed": false,
        "retry_history": [],
        "retry_recommendation": recommendation,
        "retry_budget": {
            "source": "tool_cd_or_workflow_budget",
            "attempts_consumed": 0,
            "raw_retry_trace_visible": false
        },
        "certificate_bypass_default_allowed": false,
        "caller_strategy_args_allowed": false,
        "retry_trace_chat_visible": false
    })
}

fn browser_materialization_blocker_classification(
    blocker_class: &str,
    retryable: bool,
    evidence_impact: &str,
    recommended_next_capability: &str,
    telemetry_summary: &str,
) -> Value {
    json!({
        "version": "browser_materialization_blocker_classification_v1",
        "source_pattern": "cloakbrowser_blocker_classification",
        "blocker_class": clean_text(blocker_class, 80),
        "retryable": retryable,
        "recommended_next_capability": clean_text(recommended_next_capability, 120),
        "evidence_impact": clean_text(evidence_impact, 120),
        "telemetry_summary": clean_text(telemetry_summary, 240),
        "chat_visibility": "telemetry_only_until_synthesized",
        "raw_browser_trace_chat_visible": false
    })
}

fn browser_materialization_error_blocker_classification(error: &str) -> Value {
    match error {
        "url_safety_blocked" => browser_materialization_blocker_classification(
            "unsafe_url",
            false,
            "rejected",
            "none",
            "URL safety guard blocked materialization before evidence creation.",
        ),
        "adapter_not_ready" | "browser_adapter_stub_only" => {
            browser_materialization_blocker_classification(
                "adapter_not_ready",
                true,
                "no_evidence_created",
                "browser_materialization_adapter",
                "Browser materialization capability exists but no admitted adapter is ready.",
            )
        }
        "local_static_fixture_unavailable" | "local_js_rendered_fixture_unavailable" => {
            browser_materialization_blocker_classification(
                "extraction_failed",
                true,
                "rejected",
                "policy_fixture_repair",
                "Policy-owned fixture materialization failed before extraction.",
            )
        }
        "local_static_fixture_url_mismatch" | "local_js_rendered_fixture_url_mismatch" => {
            browser_materialization_blocker_classification(
                "access_denied",
                false,
                "rejected",
                "none",
                "Policy-owned fixture does not admit the requested URL.",
            )
        }
        "unsafe_caller_control_rejected" => browser_materialization_blocker_classification(
            "unsafe_url",
            false,
            "rejected",
            "none",
            "Caller-supplied browser control was rejected before materialization.",
        ),
        "capability_not_enabled" => browser_materialization_blocker_classification(
            "adapter_not_ready",
            true,
            "no_evidence_created",
            "browser_materialization_admission",
            "Browser materialization is default-off until policy admits it.",
        ),
        _ => browser_materialization_blocker_classification(
            "extraction_failed",
            true,
            "rejected",
            "alternate_retrieval_provider",
            "Materialization failed before usable evidence was created.",
        ),
    }
}

fn browser_materialization_main_text_substantive(main_text: &str) -> bool {
    main_text.split_whitespace().count() >= 12 && main_text.chars().count() >= 80
}

fn browser_materialization_content_blocker_classification(main_text: &str) -> Value {
    if browser_materialization_main_text_substantive(main_text) {
        browser_materialization_blocker_classification(
            "none",
            false,
            "usable",
            "none",
            "Materialized page produced substantive extracted text.",
        )
    } else {
        browser_materialization_blocker_classification(
            "content_too_thin",
            false,
            "low_confidence_raw",
            "alternate_retrieval_or_more_context",
            "Materialized page was safe but did not produce enough extracted text for confident evidence promotion.",
        )
    }
}

fn browser_materialization_output_contract_projection(config: &Value) -> Value {
    let output_contract = config
        .get("output_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "version": "browser_materialized_page_contract_v1",
        "schema_ref": "web_research.browser_materialized_page.v1",
        "fields": output_contract
            .get("fields")
            .cloned()
            .unwrap_or_else(|| json!([
                "source_url",
                "pre_navigation_url_safety",
                "final_url",
                "final_url_safety",
                "status_code",
                "title",
                "main_text_or_markdown",
                "links_summary",
                "blocker_classification",
                "extraction_confidence",
                "artifact_ref",
                "readiness_strategy",
                "cleanup_status",
                "retry_diagnostics"
            ])),
        "chat_visible": output_contract
            .get("chat_visible")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "raw_payload_chat_visible": false
    })
}

fn browser_materialization_evidence_handoff_projection(config: &Value) -> Value {
    browser_materialization_evidence_handoff_projection_with_state(config, "not_created")
}

fn browser_materialization_evidence_handoff_projection_with_state(
    config: &Value,
    evidence_candidate_state: &str,
) -> Value {
    let handoff = config
        .get("evidence_handoff")
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "version": "browser_materialized_page_evidence_handoff_v1",
        "target_lane": handoff
            .get("target_lane")
            .and_then(Value::as_str)
            .unwrap_or("candidate_enrichment"),
        "promotion_requires": handoff
            .get("promotion_requires")
            .cloned()
            .unwrap_or_else(|| json!([
                "safe_final_url",
                "substantive_main_text",
                "query_relevance",
                "not_blocker_shell"
            ])),
        "confidence_values": handoff
            .get("confidence_values")
            .cloned()
            .unwrap_or_else(|| json!(["usable", "low_confidence_raw", "rejected"])),
        "evidence_candidate_state": clean_text(evidence_candidate_state, 80),
        "raw_payload_chat_visible": handoff
            .get("raw_payload_chat_visible")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "browser_success_is_not_source_truth_without_packaging": true
    })
}

fn browser_materialization_artifact_quarantine_projection() -> Value {
    json!({
        "version": "browser_materialization_artifact_quarantine_v1",
        "state": "not_created",
        "artifact_ref": Value::Null,
        "raw_payload_chat_visible": false,
        "browser_trace_chat_visible": false
    })
}

fn browser_materialization_artifact_manifest_projection(
    artifact_ref: &str,
    artifact_hash: &str,
) -> Value {
    let manifest_ref = format!("{artifact_ref}/manifest/{}", clean_text(artifact_hash, 64));
    json!({
        "version": "browser_materialization_artifact_manifest_v1",
        "state": "ref_only_fixture",
        "manifest_ref": clean_text(&manifest_ref, 340),
        "base_artifact_ref": clean_text(artifact_ref, 260),
        "projection_contains_raw_artifacts": false,
        "evidence_receives_extracted_text_only": true,
        "artifacts": [
            {
                "kind": "raw_html",
                "artifact_ref": format!("{artifact_ref}/raw-html"),
                "mime": "text/html",
                "raw_bytes_chat_visible": false,
                "workflow_trace_visible": false,
                "role": "quarantined_reprocess_only"
            },
            {
                "kind": "extracted_text",
                "artifact_ref": format!("{artifact_ref}/extracted-text"),
                "mime": "text/plain",
                "raw_bytes_chat_visible": false,
                "workflow_trace_visible": false,
                "role": "evidence_candidate_text"
            },
            {
                "kind": "browser_trace",
                "artifact_ref": format!("{artifact_ref}/browser-trace"),
                "mime": "application/json",
                "raw_bytes_chat_visible": false,
                "workflow_trace_visible": false,
                "role": "telemetry_only"
            }
        ],
        "screenshot": {
            "captured": false,
            "artifact_ref": Value::Null,
            "raw_bytes_chat_visible": false,
            "workflow_trace_visible": false
        },
        "console_log": {
            "captured": false,
            "artifact_ref": Value::Null,
            "chat_visible": false,
            "workflow_trace_visible": false
        },
        "network_log": {
            "captured": false,
            "artifact_ref": Value::Null,
            "chat_visible": false,
            "workflow_trace_visible": false
        }
    })
}

fn browser_materialization_artifact_quarantine_projection_with_ref(
    artifact_ref: &str,
    artifact_manifest: &Value,
) -> Value {
    json!({
        "version": "browser_materialization_artifact_quarantine_v1",
        "state": "created_ref_only",
        "artifact_ref": clean_text(artifact_ref, 260),
        "manifest_ref": artifact_manifest
            .get("manifest_ref")
            .cloned()
            .unwrap_or(Value::Null),
        "raw_html_ref": artifact_manifest
            .pointer("/artifacts/0/artifact_ref")
            .cloned()
            .unwrap_or(Value::Null),
        "screenshot_ref": artifact_manifest
            .pointer("/screenshot/artifact_ref")
            .cloned()
            .unwrap_or(Value::Null),
        "browser_trace_ref": artifact_manifest
            .pointer("/artifacts/2/artifact_ref")
            .cloned()
            .unwrap_or(Value::Null),
        "projection_contains_raw_artifacts": false,
        "evidence_receives_extracted_text_only": true,
        "raw_payload_chat_visible": false,
        "browser_trace_chat_visible": false,
        "raw_artifact_bytes_chat_visible": false,
        "console_log_chat_visible": false,
        "network_log_chat_visible": false
    })
}

fn browser_materialization_term_hints(title: &str, text: &str, domain: &str) -> Vec<String> {
    let mut seen = Vec::<String>::new();
    for token in format!("{title} {text} {domain}")
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|raw| clean_text(raw, 64).to_ascii_lowercase())
        .filter(|raw| raw.len() >= 4)
    {
        if !seen.iter().any(|existing| existing == &token) {
            seen.push(token);
        }
        if seen.len() >= 8 {
            break;
        }
    }
    seen
}

fn browser_materialization_evidence_pack_candidate(
    final_url: &str,
    final_url_safety: &Value,
    title: &str,
    main_text: &str,
    artifact_ref: &str,
    artifact_manifest: &Value,
) -> Value {
    let source_domain = extract_domain(final_url);
    let excerpt_hash = sha256_hex(&format!("{title}\n{main_text}\n{final_url}"));
    let final_url_status = final_url_safety
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let safe_final_url = final_url_safety
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let substantive_main_text = browser_materialization_main_text_substantive(main_text);
    let term_hints = browser_materialization_term_hints(title, main_text, &source_domain);
    let quality_flags = if safe_final_url && substantive_main_text {
        json!([
            "browser_enriched",
            "materialized_fixture",
            "raw_artifact_quarantined",
            "safe_final_url",
            "not_blocker_shell"
        ])
    } else if safe_final_url {
        json!([
            "browser_enriched",
            "materialized_fixture",
            "raw_artifact_quarantined",
            "safe_final_url",
            "content_too_thin",
            "low_confidence_raw",
            "not_promotable"
        ])
    } else {
        json!([
            "browser_enriched",
            "materialized_fixture",
            "raw_artifact_quarantined",
            "unsafe_final_url",
            "not_promotable"
        ])
    };
    json!({
        "version": "browser_materialization_evidence_pack_candidate_v1",
        "state": "evidence_pack_candidate_created",
        "pack_version": "evidence_pack_v1",
        "source_kind": "browser_materialized_page",
        "source_class": "web_page",
        "title": clean_text(title, 240),
        "locator": clean_text(final_url, 2_200),
        "source_scope": source_domain,
        "source_domain": source_domain,
        "snippet": clean_text(main_text, 1_800),
        "claim_hints": [clean_text(main_text, 420)],
        "term_hints": term_hints,
        "excerpt_hash": excerpt_hash,
        "score": 76.0,
        "score_components": {
            "relevance": 76.0,
            "source_trust_delta": 2.0,
            "freshness_delta": 0.0,
            "materialization_quality": 84.0,
            "artifact_quarantine": 100.0
        },
        "confidence": if safe_final_url && substantive_main_text {
            "usable"
        } else if safe_final_url {
            "low_confidence_raw"
        } else {
            "rejected"
        },
        "quality_flags": quality_flags,
        "coverage_facets": [],
        "freshness": {
            "status": "not_time_sensitive",
            "current_intent": false
        },
        "timestamp": Value::Null,
        "permissions": "public_web",
        "artifact_ref": clean_text(artifact_ref, 260),
        "artifact_manifest_ref": artifact_manifest.get("manifest_ref").cloned().unwrap_or(Value::Null),
        "promotion": {
            "version": "browser_materialization_evidence_promotion_v1",
            "decision": if safe_final_url && substantive_main_text {
                "candidate_ready_for_packaging"
            } else if safe_final_url {
                "candidate_retained_low_confidence_content_too_thin"
            } else {
                "rejected_by_final_url_safety"
            },
            "safety": {
                "status": final_url_status,
                "safe_final_url": safe_final_url,
                "raw_payload_chat_visible": false,
                "url_safety": final_url_safety.clone()
            },
            "components": {
                "substantive_main_text": substantive_main_text,
                "claim_hint_count": 1,
                "term_hint_count": browser_materialization_term_hints(title, main_text, final_url).len(),
                "artifact_manifest_present": artifact_manifest.get("manifest_ref").is_some()
            },
            "non_goals": [
                "candidate_is_not_final_answer_text",
                "browser_success_is_not_source_truth_without_packaging",
                "raw_payload_is_not_chat_visible"
            ]
        },
        "evidence_artifacts": {
            "materialized_page_ref": clean_text(artifact_ref, 260),
            "manifest_ref": artifact_manifest.get("manifest_ref").cloned().unwrap_or(Value::Null),
            "raw_html_ref": artifact_manifest.pointer("/artifacts/0/artifact_ref").cloned().unwrap_or(Value::Null),
            "reader_text_ref": artifact_manifest.pointer("/artifacts/1/artifact_ref").cloned().unwrap_or(Value::Null),
            "reader_screenshot_ref": artifact_manifest.pointer("/screenshot/artifact_ref").cloned().unwrap_or(Value::Null)
        },
        "raw_payload_chat_visible": false,
        "visibility": "synthesis_context_after_promotion",
        "promoted_to_evidence_pack": false
    })
}

fn browser_materialization_selected_provider(config: &Value) -> String {
    config
        .get("provider_order")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 80).to_ascii_lowercase())
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "local_browser".to_string())
}

fn browser_materialization_ref_hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

fn browser_materialization_local_fixture_config(config: &Value, provider: &str) -> Value {
    let primary_key = if provider == "local_js_rendered_fixture" {
        "local_js_rendered_fixture"
    } else {
        "local_static_fixture"
    };
    config
        .get(primary_key)
        .or_else(|| config.get("local_static_fixture"))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn browser_materialization_safe_relative_path(raw: &str) -> Option<PathBuf> {
    let cleaned = clean_text(raw, 800);
    if cleaned.is_empty() {
        return None;
    }
    let path = Path::new(&cleaned);
    if path.is_absolute() {
        return None;
    }
    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(row) => safe.push(row),
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    if safe.as_os_str().is_empty() {
        None
    } else {
        Some(safe)
    }
}

fn browser_materialization_local_fixture_path(
    root: &Path,
    fixture_config: &Value,
) -> Result<PathBuf, String> {
    browser_materialization_local_fixture_path_for_key(root, fixture_config, "fixture_rel_path")
}

fn browser_materialization_local_fixture_path_for_key(
    root: &Path,
    fixture_config: &Value,
    key: &str,
) -> Result<PathBuf, String> {
    let rel = fixture_config
        .get(key)
        .and_then(Value::as_str)
        .and_then(browser_materialization_safe_relative_path)
        .ok_or_else(|| format!("local_fixture_missing_safe_relative_path:{key}"))?;
    let candidate = root.join(rel);
    if candidate.exists() {
        let root_canonical = root
            .canonicalize()
            .map_err(|err| format!("local_static_fixture_root_canonicalize_failed:{err}"))?;
        let candidate_canonical = candidate
            .canonicalize()
            .map_err(|err| format!("local_static_fixture_canonicalize_failed:{err}"))?;
        if !candidate_canonical.starts_with(&root_canonical) {
            return Err("local_static_fixture_path_escapes_root".to_string());
        }
        return Ok(candidate_canonical);
    }
    Ok(candidate)
}

fn browser_materialization_local_fixture_final_url(fixture_config: &Value, url: &str) -> String {
    clean_text(
        fixture_config
            .get("final_url")
            .and_then(Value::as_str)
            .unwrap_or(url),
        2200,
    )
}

fn browser_materialization_links_summary_from_html(raw_html: &str, base_url: &str) -> Value {
    let rows = regex_anchor()
        .captures_iter(raw_html)
        .take(12)
        .filter_map(|captures| {
            let href = clean_text(captures.get(1).map(|m| m.as_str()).unwrap_or(""), 2200);
            if href.is_empty() {
                return None;
            }
            let resolved = resolve_fetch_redirect_url(base_url, &href).unwrap_or(href);
            let text = normalize_block_text(&strip_tags_to_text(
                captures.get(2).map(|m| m.as_str()).unwrap_or(""),
            ));
            Some(json!({
                "href": clean_text(&resolved, 2200),
                "text": clean_text(&text, 220)
            }))
        })
        .collect::<Vec<_>>();
    Value::Array(rows)
}

fn browser_materialization_fake_success(
    url: &str,
    config: &Value,
    runtime_metadata: &Value,
    pre_navigation_url_safety: Value,
) -> Value {
    let final_url = clean_text(url, 2200);
    let final_ssrf_guard = evaluate_fetch_ssrf_guard(&final_url, false, None);
    let final_url_safety =
        browser_materialization_observed_final_url_safety_projection(&final_url, &final_ssrf_guard);
    let artifact_hash =
        browser_materialization_ref_hash(&["fake_materialization", url, &final_url]);
    let artifact_ref = format!(
        "artifact://web_conduit/browser_materialization/fake/{}",
        clean_text(&artifact_hash, 64)
    );
    let artifact_manifest =
        browser_materialization_artifact_manifest_projection(&artifact_ref, &artifact_hash);
    let cleanup_status = browser_materialization_fake_cleanup_status_projection();
    let readiness_strategy = browser_materialization_readiness_strategy_projection(config);
    let retry_diagnostics = browser_materialization_retry_diagnostics_projection("none");
    let title = "Deterministic browser materialization fixture";
    let main_text = "Fake browser materialization provider returned a deterministic rendered page fixture for contract proof. This text is extracted content, not raw HTML.";
    let blocker_classification = browser_materialization_content_blocker_classification(main_text);
    let evidence_candidate = browser_materialization_evidence_pack_candidate(
        &final_url,
        &final_url_safety,
        title,
        main_text,
        &artifact_ref,
        &artifact_manifest,
    );
    let evidence_ref = json!({
        "source_kind": evidence_candidate
            .get("source_kind")
            .cloned()
            .unwrap_or_else(|| json!("browser_materialized_page")),
        "title": evidence_candidate
            .get("title")
            .cloned()
            .unwrap_or_else(|| json!(title)),
        "locator": evidence_candidate
            .get("locator")
            .cloned()
            .unwrap_or_else(|| json!(final_url.clone())),
        "excerpt_hash": evidence_candidate
            .get("excerpt_hash")
            .cloned()
            .unwrap_or_else(|| json!(sha256_hex(main_text))),
        "score": evidence_candidate
            .get("score")
            .cloned()
            .unwrap_or_else(|| json!(76.0)),
        "timestamp": evidence_candidate
            .get("timestamp")
            .cloned()
            .unwrap_or(Value::Null),
        "permissions": evidence_candidate
            .get("permissions")
            .cloned()
            .unwrap_or_else(|| json!("public_web")),
        "artifact_ref": artifact_ref.clone()
    });
    let materialized_page = json!({
        "version": "browser_materialized_page_v1",
        "provider": "fake_materialization",
        "source_url": clean_text(url, 2200),
        "pre_navigation_url_safety": pre_navigation_url_safety.clone(),
        "final_url": final_url.clone(),
        "final_url_safety": final_url_safety.clone(),
        "status_code": 200,
        "title": title,
        "main_text_or_markdown": main_text,
        "links_summary": [
            {
                "href": clean_text(url, 2200),
                "text": "source"
            }
        ],
        "blocker_classification": blocker_classification.clone(),
        "extraction_confidence": "usable",
        "artifact_ref": artifact_ref,
        "artifact_manifest_ref": artifact_manifest
            .get("manifest_ref")
            .cloned()
            .unwrap_or(Value::Null),
        "readiness_strategy": readiness_strategy.clone(),
        "cleanup_status": cleanup_status.clone(),
        "retry_diagnostics": retry_diagnostics.clone(),
        "raw_payload_chat_visible": false,
        "browser_trace_chat_visible": false
    });
    json!({
        "ok": true,
        "type": "web_conduit_browser_materialization",
        "capability": "browser_materialize_page",
        "provider": "fake_materialization",
        "reason": "Fake browser materialization provider produced a deterministic materialized-page contract proof.",
        "url": clean_text(url, 2200),
        "tool_execution_attempted": true,
        "browser_launch_attempted": false,
        "raw_payload_chat_visible": false,
        "chat_visible": false,
        "materialized_page": materialized_page,
        "evidence_candidate": evidence_candidate.clone(),
        "evidence_pack_candidates": [evidence_candidate],
        "evidence_refs": [evidence_ref],
        "artifact_ref": format!("artifact://web_conduit/browser_materialization/fake/{artifact_hash}"),
        "artifact_manifest": artifact_manifest.clone(),
        "materialized_page_contract": browser_materialization_output_contract_projection(config),
        "evidence_handoff_contract": browser_materialization_evidence_handoff_projection_with_state(
            config,
            "evidence_pack_candidate_created",
        ),
        "blocker_classification": blocker_classification,
        "artifact_quarantine": browser_materialization_artifact_quarantine_projection_with_ref(
            &format!("artifact://web_conduit/browser_materialization/fake/{artifact_hash}"),
            &artifact_manifest,
        ),
        "pre_navigation_url_safety": pre_navigation_url_safety.clone(),
        "final_url_safety": final_url_safety,
        "navigation_contract": browser_materialization_navigation_contract_projection(config),
        "readiness_strategy": readiness_strategy,
        "context_contract": browser_materialization_context_contract_projection(),
        "cleanup_status": cleanup_status,
        "retry_diagnostics": retry_diagnostics,
        "url_safety": pre_navigation_url_safety,
        "profile_compilation": runtime_metadata
            .pointer("/profile_compilation")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "readiness_lifecycle": runtime_metadata
            .pointer("/capability_contract/readiness_lifecycle")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "execution_gate": runtime_metadata
            .get("execution_gate")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "capability_contract_ref": "core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json#browser_materialization_capability_contract",
        "decision_authority": "web_conduit_policy_and_tool_cd"
    })
}

fn browser_materialization_local_fixture_error_code(provider: &str, suffix: &str) -> &'static str {
    match (provider, suffix) {
        ("local_js_rendered_fixture", "url_mismatch") => "local_js_rendered_fixture_url_mismatch",
        ("local_js_rendered_fixture", _) => "local_js_rendered_fixture_unavailable",
        ("local_static_fixture", "url_mismatch") => "local_static_fixture_url_mismatch",
        _ => "local_static_fixture_unavailable",
    }
}

fn browser_materialization_local_fixture_failure(
    provider: &str,
    error: &str,
    reason: &str,
    url: &str,
    config: &Value,
    runtime_metadata: &Value,
    url_safety: Value,
) -> Value {
    let mut out =
        browser_materialization_fail_closed(error, reason, url, config, runtime_metadata, url_safety);
    let provider = clean_text(provider, 80);
    let provider_metadata_key = if provider == "local_js_rendered_fixture" {
        "local_js_rendered_fixture"
    } else {
        "local_static_fixture"
    };
    out["provider"] = json!(provider);
    out["tool_execution_attempted"] = json!(true);
    out["cleanup_status"] =
        browser_materialization_local_fixture_cleanup_status_projection("completed_after_failure");
    out["retry_diagnostics"] = browser_materialization_retry_diagnostics_projection(error);
    out[provider_metadata_key] = json!({
        "version": "browser_materialization_local_static_fixture_v1",
        "source": "policy_owned_fixture",
        "fixture_path_chat_visible": false,
        "raw_fixture_payload_chat_visible": false,
        "cleanup_attempted": true
    });
    out
}

fn browser_materialization_local_fixture_success(
    root: &Path,
    url: &str,
    config: &Value,
    runtime_metadata: &Value,
    pre_navigation_url_safety: Value,
    request: &Value,
    provider: &str,
) -> Value {
    let provider = clean_text(provider, 80).to_ascii_lowercase();
    let provider = if provider == "local_js_rendered_fixture" {
        "local_js_rendered_fixture"
    } else {
        "local_static_fixture"
    };
    let fixture_config = browser_materialization_local_fixture_config(config, provider);
    let fixture_url = clean_text(
        fixture_config
            .get("fixture_url")
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    if fixture_url != clean_text(url, 2200) {
        return browser_materialization_local_fixture_failure(
            provider,
            browser_materialization_local_fixture_error_code(provider, "url_mismatch"),
            "Policy-owned local static fixture does not admit this URL.",
            url,
            config,
            runtime_metadata,
            pre_navigation_url_safety,
        );
    }
    let fixture_path = match browser_materialization_local_fixture_path(root, &fixture_config) {
        Ok(path) => path,
        Err(reason) => {
            return browser_materialization_local_fixture_failure(
                provider,
                browser_materialization_local_fixture_error_code(provider, "unavailable"),
                &reason,
                url,
                config,
                runtime_metadata,
                pre_navigation_url_safety,
            );
        }
    };
    let final_url = browser_materialization_local_fixture_final_url(&fixture_config, url);
    let final_ssrf_guard = evaluate_fetch_ssrf_guard(&final_url, false, None);
    let final_url_safety =
        browser_materialization_observed_final_url_safety_projection(&final_url, &final_ssrf_guard);
    if !final_ssrf_guard
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let mut out = browser_materialization_local_fixture_failure(
            provider,
            "url_safety_blocked",
            "Policy-owned local fixture final URL failed revalidation before fixture read or artifact creation.",
            url,
            config,
            runtime_metadata,
            pre_navigation_url_safety,
        );
        out["final_url_safety"] = final_url_safety;
        return out;
    }

    let raw_html = match fs::read_to_string(&fixture_path) {
        Ok(raw) => raw,
        Err(err) => {
            return browser_materialization_local_fixture_failure(
                provider,
                browser_materialization_local_fixture_error_code(provider, "unavailable"),
                &format!("policy_owned_fixture_read_failed:{err}"),
                url,
                config,
                runtime_metadata,
                pre_navigation_url_safety,
            );
        }
    };

    let extract_mode = clean_text(
        request
            .get("extract_mode")
            .and_then(Value::as_str)
            .unwrap_or("text"),
        40,
    )
    .to_ascii_lowercase();
    let extract_mode = if extract_mode == "markdown" {
        "markdown"
    } else {
        "text"
    };
    let content_type = clean_text(
        fixture_config
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or("text/html; charset=utf-8"),
        120,
    );
    let max_chars = request
        .get("max_response_bytes")
        .and_then(Value::as_u64)
        .or_else(|| config.get("max_response_bytes").and_then(Value::as_u64))
        .unwrap_or(350000)
        .clamp(256, 1_000_000) as usize;
    let (direct_text, title_from_html, direct_truncated, direct_extractor) =
        extract_fetch_content_with_extractor(&raw_html, &content_type, extract_mode, max_chars);
    let mut main_text = direct_text.clone();
    let mut truncated = direct_truncated;
    let mut extractor = direct_extractor.clone();
    let rendered_marker = clean_text(
        fixture_config
            .get("rendered_marker")
            .and_then(Value::as_str)
            .unwrap_or("Rendered JS fixture content"),
        220,
    );
    let mut js_render_proof = Value::Null;
    if provider == "local_js_rendered_fixture" {
        let rendered_path = match browser_materialization_local_fixture_path_for_key(
            root,
            &fixture_config,
            "rendered_text_rel_path",
        ) {
            Ok(path) => path,
            Err(reason) => {
                return browser_materialization_local_fixture_failure(
                    provider,
                    browser_materialization_local_fixture_error_code(provider, "unavailable"),
                    &reason,
                    url,
                    config,
                    runtime_metadata,
                    pre_navigation_url_safety,
                );
            }
        };
        let rendered_raw = match fs::read_to_string(&rendered_path) {
            Ok(raw) => raw,
            Err(err) => {
                return browser_materialization_local_fixture_failure(
                    provider,
                    browser_materialization_local_fixture_error_code(provider, "unavailable"),
                    &format!("policy_owned_rendered_fixture_read_failed:{err}"),
                    url,
                    config,
                    runtime_metadata,
                    pre_navigation_url_safety,
                );
            }
        };
        let normalized_rendered = normalize_block_text(&strip_invisible_unicode(&rendered_raw));
        let (rendered, rendered_truncated) = truncate_chars(&normalized_rendered, max_chars);
        main_text = rendered;
        truncated = rendered_truncated;
        extractor = "policy_owned_js_render_fixture".to_string();
        js_render_proof = json!({
            "version": "browser_materialization_local_js_render_proof_v1",
            "source": "policy_owned_rendered_fixture",
            "direct_fetch_probe_extractor": direct_extractor,
            "direct_fetch_contains_rendered_marker": !rendered_marker.is_empty()
                && direct_text.contains(&rendered_marker),
            "materialized_contains_rendered_marker": !rendered_marker.is_empty()
                && main_text.contains(&rendered_marker),
            "readiness_strategy_policy_owned": true,
            "caller_supplied_script_allowed": false,
            "raw_script_chat_visible": false,
            "raw_rendered_fixture_payload_chat_visible": false
        });
    }
    let title = title_from_html
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| {
            clean_text(
                fixture_config
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("Local static materialization fixture"),
                220,
            )
        });
    let extraction_confidence = if main_text.chars().count() >= 80 {
        "usable"
    } else {
        "low_confidence_raw"
    };
    let blocker_classification = browser_materialization_content_blocker_classification(&main_text);
    let links_summary = browser_materialization_links_summary_from_html(&raw_html, &final_url);
    let body_hash = sha256_hex(&raw_html);
    let artifact_hash = browser_materialization_ref_hash(&[provider, url, &final_url, &body_hash]);
    let artifact_segment = if provider == "local_js_rendered_fixture" {
        "local-js"
    } else {
        "local-static"
    };
    let artifact_ref = format!(
        "artifact://web_conduit/browser_materialization/{artifact_segment}/{}",
        clean_text(&artifact_hash, 64)
    );
    let artifact_manifest =
        browser_materialization_artifact_manifest_projection(&artifact_ref, &artifact_hash);
    let cleanup_status =
        browser_materialization_local_fixture_cleanup_status_projection("completed");
    let readiness_strategy = browser_materialization_readiness_strategy_projection(config);
    let retry_diagnostics = browser_materialization_retry_diagnostics_projection("none");
    let evidence_candidate = browser_materialization_evidence_pack_candidate(
        &final_url,
        &final_url_safety,
        &title,
        &main_text,
        &artifact_ref,
        &artifact_manifest,
    );
    let evidence_ref = json!({
        "source_kind": evidence_candidate
            .get("source_kind")
            .cloned()
            .unwrap_or_else(|| json!("browser_materialized_page")),
        "title": evidence_candidate
            .get("title")
            .cloned()
            .unwrap_or_else(|| json!(title.clone())),
        "locator": evidence_candidate
            .get("locator")
            .cloned()
            .unwrap_or_else(|| json!(final_url.clone())),
        "excerpt_hash": evidence_candidate
            .get("excerpt_hash")
            .cloned()
            .unwrap_or_else(|| json!(sha256_hex(&main_text))),
        "score": evidence_candidate
            .get("score")
            .cloned()
            .unwrap_or_else(|| json!(76.0)),
        "timestamp": evidence_candidate
            .get("timestamp")
            .cloned()
            .unwrap_or(Value::Null),
        "permissions": evidence_candidate
            .get("permissions")
            .cloned()
            .unwrap_or_else(|| json!("public_web")),
        "artifact_ref": artifact_ref.clone()
    });
    let mut materialized_page = json!({
        "version": "browser_materialized_page_v1",
        "provider": provider,
        "source_url": clean_text(url, 2200),
        "pre_navigation_url_safety": pre_navigation_url_safety.clone(),
        "final_url": final_url.clone(),
        "final_url_safety": final_url_safety.clone(),
        "status_code": 200,
        "title": title,
        "main_text_or_markdown": main_text,
        "content_truncated": truncated,
        "extractor": extractor,
        "links_summary": links_summary,
        "blocker_classification": blocker_classification.clone(),
        "extraction_confidence": extraction_confidence,
        "artifact_ref": artifact_ref,
        "artifact_manifest_ref": artifact_manifest
            .get("manifest_ref")
            .cloned()
            .unwrap_or(Value::Null),
        "readiness_strategy": readiness_strategy.clone(),
        "cleanup_status": cleanup_status.clone(),
        "retry_diagnostics": retry_diagnostics.clone(),
        "raw_payload_chat_visible": false,
        "browser_trace_chat_visible": false,
        "browser_handle_visible": false,
        "cdp_url_visible": false,
        "local_fixture": {
            "version": "browser_materialization_local_static_fixture_v1",
            "source": "policy_owned_fixture",
            "fixture_path_chat_visible": false,
            "raw_fixture_payload_chat_visible": false
        }
    });
    if provider == "local_js_rendered_fixture" {
        materialized_page["local_js_rendered_fixture"] = json!({
            "version": "browser_materialization_local_js_rendered_fixture_v1",
            "source": "policy_owned_rendered_fixture",
            "rendered_text_path_chat_visible": false,
            "raw_rendered_fixture_payload_chat_visible": false,
            "caller_supplied_script_allowed": false
        });
        materialized_page["js_render_proof"] = js_render_proof.clone();
    } else {
        materialized_page["local_static_fixture"] = json!({
            "version": "browser_materialization_local_static_fixture_v1",
            "source": "policy_owned_fixture",
            "fixture_path_chat_visible": false,
            "raw_fixture_payload_chat_visible": false
        });
    }
    json!({
        "ok": true,
        "type": "web_conduit_browser_materialization",
        "capability": "browser_materialize_page",
        "provider": provider,
        "reason": if provider == "local_js_rendered_fixture" {
            "Policy-owned local JS-rendered fixture materialized through the browser materialization contract."
        } else {
            "Policy-owned local static fixture materialized through the browser materialization contract."
        },
        "url": clean_text(url, 2200),
        "tool_execution_attempted": true,
        "browser_launch_attempted": false,
        "raw_payload_chat_visible": false,
        "chat_visible": false,
        "materialized_page": materialized_page,
        "evidence_candidate": evidence_candidate.clone(),
        "evidence_pack_candidates": [evidence_candidate],
        "evidence_refs": [evidence_ref],
        "artifact_ref": format!("artifact://web_conduit/browser_materialization/{artifact_segment}/{artifact_hash}"),
        "artifact_manifest": artifact_manifest.clone(),
        "materialized_page_contract": browser_materialization_output_contract_projection(config),
        "evidence_handoff_contract": browser_materialization_evidence_handoff_projection_with_state(
            config,
            "evidence_pack_candidate_created",
        ),
        "blocker_classification": blocker_classification,
        "artifact_quarantine": browser_materialization_artifact_quarantine_projection_with_ref(
            &format!("artifact://web_conduit/browser_materialization/local-static/{artifact_hash}"),
            &artifact_manifest,
        ),
        "pre_navigation_url_safety": pre_navigation_url_safety.clone(),
        "final_url_safety": final_url_safety,
        "navigation_contract": browser_materialization_navigation_contract_projection(config),
        "readiness_strategy": readiness_strategy,
        "context_contract": browser_materialization_context_contract_projection(),
        "cleanup_status": cleanup_status,
        "retry_diagnostics": retry_diagnostics,
        "url_safety": pre_navigation_url_safety,
        "profile_compilation": runtime_metadata
            .pointer("/profile_compilation")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "readiness_lifecycle": runtime_metadata
            .pointer("/capability_contract/readiness_lifecycle")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "execution_gate": runtime_metadata
            .get("execution_gate")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "js_render_proof": js_render_proof,
        "local_static_fixture": {
            "version": "browser_materialization_local_static_fixture_v1",
            "source": "policy_owned_fixture",
            "fixture_path_chat_visible": false,
            "raw_fixture_payload_chat_visible": false,
            "content_hash": body_hash,
            "cleanup_attempted": true
        },
        "capability_contract_ref": "core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json#browser_materialization_capability_contract",
        "decision_authority": "web_conduit_policy_and_tool_cd"
    })
}

fn browser_materialization_fail_closed(
    error: &str,
    reason: &str,
    url: &str,
    config: &Value,
    runtime_metadata: &Value,
    url_safety: Value,
) -> Value {
    json!({
        "ok": false,
        "type": "web_conduit_browser_materialization",
        "capability": "browser_materialize_page",
        "error": error,
        "reason": clean_text(reason, 240),
        "url": clean_text(url, 2200),
        "tool_execution_attempted": false,
        "browser_launch_attempted": false,
        "raw_payload_chat_visible": false,
        "chat_visible": false,
        "materialized_page": Value::Null,
        "evidence_candidate": Value::Null,
        "artifact_ref": Value::Null,
        "materialized_page_contract": browser_materialization_output_contract_projection(config),
        "evidence_handoff_contract": browser_materialization_evidence_handoff_projection(config),
        "blocker_classification": browser_materialization_error_blocker_classification(error),
        "artifact_quarantine": browser_materialization_artifact_quarantine_projection(),
        "pre_navigation_url_safety": url_safety.clone(),
        "final_url_safety": browser_materialization_final_url_safety_projection(),
        "navigation_contract": browser_materialization_navigation_contract_projection(config),
        "readiness_strategy": browser_materialization_readiness_strategy_projection(config),
        "context_contract": browser_materialization_context_contract_projection(),
        "cleanup_status": browser_materialization_cleanup_status_projection(),
        "retry_diagnostics": browser_materialization_retry_diagnostics_projection(error),
        "url_safety": url_safety,
        "profile_compilation": runtime_metadata
            .pointer("/profile_compilation")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "readiness_lifecycle": runtime_metadata
            .pointer("/capability_contract/readiness_lifecycle")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "execution_gate": runtime_metadata
            .get("execution_gate")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "capability_contract_ref": "core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json#browser_materialization_capability_contract",
        "decision_authority": "web_conduit_policy_and_tool_cd"
    })
}

pub fn api_browser_materialize_page(root: &Path, request: &Value) -> Value {
    let (policy, _) = load_policy(root);
    let config = browser_materialization_config_from_policy(&policy);
    let runtime_web_tools_metadata = runtime_web_tools_snapshot(root, &policy);
    let runtime_metadata = runtime_web_tools_metadata
        .get("browser_materialization")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let url = clean_text(request.get("url").and_then(Value::as_str).unwrap_or(""), 2200);
    let admission_ref = clean_text(
        request
            .get("admission_ref")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let blank_safety = json!({
        "version": "browser_materialization_url_safety_v1",
        "url": url,
        "ok": false,
        "status": "not_evaluated",
        "host": Value::Null,
        "error": Value::Null,
        "resolved_ip_addrs": []
    });
    if url.is_empty() {
        return browser_materialization_fail_closed(
            "missing_required_field",
            "Browser materialization requires a URL.",
            "",
            &config,
            &runtime_metadata,
            blank_safety,
        );
    }
    if admission_ref.is_empty() {
        return browser_materialization_fail_closed(
            "missing_required_field",
            "Browser materialization requires an admission_ref capability handle.",
            &url,
            &config,
            &runtime_metadata,
            blank_safety,
        );
    }

    let denied_fields = browser_materialization_request_field_list(
        &config,
        "/request_contract/denied_fields",
        &[
            "browser_args",
            "launch_args",
            "args",
            "stealthArgs",
            "stealth_args",
            "extra_args",
            "_strategy_args",
            "ignoreDefaultArgs",
            "ignore_default_args",
            "binary_path",
            "binaryPath",
            "download_url",
            "downloadUrl",
            "cache_dir",
            "cacheDir",
            "chromium_version",
            "chromiumVersion",
            "fingerprint_seed",
            "fingerprintSeed",
            "backend",
            "browser_backend",
            "browserBackend",
            "adapter",
            "adapter_kind",
            "launchOptions",
            "contextOptions",
            "context_options",
            "headless",
            "viewport",
            "userAgent",
            "user_agent",
            "timezone",
            "timezoneId",
            "timezone_id",
            "locale",
            "colorScheme",
            "humanize",
            "humanPreset",
            "humanConfig",
            "geoip",
            "script",
            "scripts",
            "javascript",
            "evaluate",
            "evaluate_script",
            "wait_script",
            "raw_wait_script",
            "cdp_command",
            "user_script",
            "proxy",
            "proxy_url",
            "proxy_credentials",
            "proxyUrl",
            "proxyCredentials",
            "session_id",
            "sessionId",
            "storage_state",
            "storageState",
            "local_file",
            "localFile",
            "userDataDir",
            "raw_html",
            "rawHtml",
            "html",
            "raw_payload",
            "rawPayload",
            "screenshot",
            "screenshot_bytes",
            "screenshotBytes",
            "browser_trace",
            "browserTrace",
            "console_logs",
            "consoleLogs",
            "network_logs",
            "networkLogs",
            "trace",
        ],
    );
    if let Some(field) = browser_materialization_first_denied_request_field(request, &denied_fields)
    {
        return browser_materialization_fail_closed(
            "unsafe_caller_control_rejected",
            &format!("Caller-supplied browser control field rejected: {field}."),
            &url,
            &config,
            &runtime_metadata,
            blank_safety,
        );
    }

    let ssrf_guard = evaluate_fetch_ssrf_guard(&url, false, None);
    let url_safety = browser_materialization_safety_projection(&url, &ssrf_guard);
    if !ssrf_guard
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return browser_materialization_fail_closed(
            "url_safety_blocked",
            "Browser materialization rejected the URL before adapter execution.",
            &url,
            &config,
            &runtime_metadata,
            url_safety,
        );
    }

    let enabled = config
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !enabled {
        return browser_materialization_fail_closed(
            "capability_not_enabled",
            "Browser materialization is declared but not enabled by policy.",
            &url,
            &config,
            &runtime_metadata,
            url_safety,
        );
    }

    let adapter_ready = config
        .get("adapter_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !adapter_ready {
        return browser_materialization_fail_closed(
            "adapter_not_ready",
            "Browser materialization is enabled, but no admitted adapter is ready.",
            &url,
            &config,
            &runtime_metadata,
            url_safety,
        );
    }

    if browser_materialization_selected_provider(&config) == "fake_materialization" {
        return browser_materialization_fake_success(&url, &config, &runtime_metadata, url_safety);
    }
    let selected_provider = browser_materialization_selected_provider(&config);
    if selected_provider == "local_static_fixture" || selected_provider == "local_js_rendered_fixture"
    {
        return browser_materialization_local_fixture_success(
            root,
            &url,
            &config,
            &runtime_metadata,
            url_safety,
            request,
            &selected_provider,
        );
    }

    browser_materialization_fail_closed(
        "browser_adapter_stub_only",
        "Browser materialization adapter boundary exists, but live execution is not implemented in this primitive yet.",
        &url,
        &config,
        &runtime_metadata,
        url_safety,
    )
}
