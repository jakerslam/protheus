// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/security (authoritative)

fn web_conduit_parse_domains(raw: Option<&Value>) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let Some(Value::Array(rows)) = raw else {
        return out;
    };
    for row in rows {
        let source = row
            .as_str()
            .map(|v| v.to_string())
            .unwrap_or_else(|| row.to_string().trim_matches('"').to_string());
        let cleaned = clean(source, 220).to_ascii_lowercase();
        if cleaned.is_empty() {
            continue;
        }
        out.push(cleaned.trim_matches('.').to_string());
    }
    out.sort();
    out.dedup();
    out
}

fn web_conduit_extract_domain(raw_url: &str) -> String {
    let mut url = clean(raw_url, 2200).to_ascii_lowercase();
    if url.is_empty() {
        return String::new();
    }
    if let Some(rest) = url.strip_prefix("http://") {
        url = rest.to_string();
    } else if let Some(rest) = url.strip_prefix("https://") {
        url = rest.to_string();
    }
    let without_path = url.split(['/', '?', '#']).next().unwrap_or_default().trim();
    let host = without_path
        .split('@')
        .next_back()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default()
        .trim_matches('.');
    clean(host, 220).to_ascii_lowercase()
}

fn web_conduit_domain_matches(domain: &str, rule: &str) -> bool {
    if domain.is_empty() || rule.is_empty() {
        return false;
    }
    if domain == rule {
        return true;
    }
    domain.ends_with(&format!(".{rule}"))
}

pub fn evaluate_web_conduit_policy(_repo_root: &Path, request: &Value, policy: &Value) -> Value {
    let policy_scope = policy
        .get("web_conduit")
        .cloned()
        .unwrap_or_else(|| policy.clone());
    let policy_obj = policy_scope.as_object().cloned().unwrap_or_default();
    let mode = clean(
        policy
            .get("mode")
            .and_then(Value::as_str)
            .or_else(|| policy_obj.get("mode").and_then(Value::as_str))
            .unwrap_or("production"),
        32,
    )
    .to_ascii_lowercase();
    let policy_version = clean(
        policy
            .get("version")
            .and_then(Value::as_str)
            .or_else(|| policy_obj.get("version").and_then(Value::as_str))
            .unwrap_or("v1"),
        40,
    );
    let enabled = policy_obj
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let require_human_for_sensitive = policy_obj
        .get("require_human_for_sensitive")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let max_response_bytes = policy_obj
        .get("max_response_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(350_000)
        .clamp(4096, 4_000_000);
    let timeout_ms = policy_obj
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(9000)
        .clamp(1000, 120_000);
    let rate_limit_per_minute = policy_obj
        .get("rate_limit_per_minute")
        .and_then(Value::as_u64)
        .unwrap_or(30)
        .clamp(1, 600);
    let allow_domains = web_conduit_parse_domains(policy_obj.get("allow_domains"));
    let deny_domains = web_conduit_parse_domains(policy_obj.get("deny_domains"));
    let sensitive_domains = web_conduit_parse_domains(policy_obj.get("sensitive_domains"));

    let requested_url = clean(
        request
            .get("requested_url")
            .or_else(|| request.get("url"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    let domain = web_conduit_extract_domain(&requested_url);
    let requests_last_minute = request
        .get("requests_last_minute")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let human_approved = request
        .get("human_approved")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let scheme_ok = requested_url.starts_with("http://") || requested_url.starts_with("https://");
    let mut allow = enabled && !requested_url.is_empty() && !domain.is_empty() && scheme_ok;
    let mut reason = if allow {
        "policy_allow".to_string()
    } else if !enabled {
        "web_conduit_disabled".to_string()
    } else if requested_url.is_empty() {
        "requested_url_required".to_string()
    } else if !scheme_ok {
        "unsupported_url_scheme".to_string()
    } else {
        "domain_required".to_string()
    };

    let deny_match = deny_domains
        .iter()
        .find(|rule| web_conduit_domain_matches(&domain, rule))
        .cloned();
    if allow && deny_match.is_some() {
        allow = false;
        reason = "domain_denylisted".to_string();
    }

    if allow && !allow_domains.is_empty() {
        let is_allowlisted = allow_domains
            .iter()
            .any(|rule| web_conduit_domain_matches(&domain, rule));
        if !is_allowlisted {
            allow = false;
            reason = "domain_not_allowlisted".to_string();
        }
    }

    let sensitive_match = sensitive_domains
        .iter()
        .find(|rule| web_conduit_domain_matches(&domain, rule))
        .cloned();
    let requires_human_approval =
        require_human_for_sensitive && sensitive_match.is_some() && !human_approved;
    if allow && requires_human_approval {
        allow = false;
        reason = "human_approval_required_for_sensitive_domain".to_string();
    }

    if allow && requests_last_minute >= rate_limit_per_minute {
        allow = false;
        reason = "rate_limit_exceeded".to_string();
    }

    json!({
        "ok": true,
        "allow": allow,
        "decision": if allow { "allow" } else { "deny" },
        "reason": reason,
        "mode": mode,
        "requested_url": requested_url,
        "domain": domain,
        "human_approved": human_approved,
        "requires_human_approval": requires_human_approval,
        "requests_last_minute": requests_last_minute,
        "policy": {
            "version": policy_version,
            "enabled": enabled,
            "max_response_bytes": max_response_bytes,
            "timeout_ms": timeout_ms,
            "rate_limit_per_minute": rate_limit_per_minute,
            "allow_domains": allow_domains,
            "deny_domains": deny_domains,
            "sensitive_domains": sensitive_domains,
            "require_human_for_sensitive": require_human_for_sensitive
        },
        "matches": {
            "deny_rule": deny_match,
            "sensitive_rule": sensitive_match
        }
    })
}

#[cfg(test)]
mod web_conduit_policy_tests {
    use super::*;

    fn base_policy() -> Value {
        json!({
            "version": "v1",
            "mode": "production",
            "web_conduit": {
                "enabled": true,
                "max_response_bytes": 128000,
                "timeout_ms": 9000,
                "rate_limit_per_minute": 5,
                "allow_domains": [],
                "deny_domains": ["blocked.example"],
                "sensitive_domains": ["accounts.google.com"],
                "require_human_for_sensitive": true
            }
        })
    }

    #[test]
    fn sensitive_domain_requires_human_approval() {
        let request = json!({
            "requested_url": "https://accounts.google.com/signin",
            "requests_last_minute": 0,
            "human_approved": false
        });
        let out = evaluate_web_conduit_policy(Path::new("."), &request, &base_policy());
        assert_eq!(out.get("allow").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("reason").and_then(Value::as_str),
            Some("human_approval_required_for_sensitive_domain")
        );
    }

    #[test]
    fn rate_limit_is_fail_closed() {
        let request = json!({
            "requested_url": "https://example.com",
            "requests_last_minute": 6,
            "human_approved": false
        });
        let out = evaluate_web_conduit_policy(Path::new("."), &request, &base_policy());
        assert_eq!(out.get("allow").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("reason").and_then(Value::as_str),
            Some("rate_limit_exceeded")
        );
    }

    #[test]
    fn standard_domain_is_allowed_when_policy_permits() {
        let request = json!({
            "requested_url": "https://example.com/docs",
            "requests_last_minute": 0,
            "human_approved": false
        });
        let out = evaluate_web_conduit_policy(Path::new("."), &request, &base_policy());
        assert_eq!(out.get("allow").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("decision").and_then(Value::as_str), Some("allow"));
    }
}
