// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("main_parts/010-now-iso.rs");
include!("main_parts/020-analytics-policy.rs");
include!("main_parts/030-cmd-test-harness.rs");

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                *ch,
                '\u{200B}'
                    | '\u{200C}'
                    | '\u{200D}'
                    | '\u{200E}'
                    | '\u{200F}'
                    | '\u{202A}'
                    | '\u{202B}'
                    | '\u{202C}'
                    | '\u{202D}'
                    | '\u{202E}'
                    | '\u{2060}'
                    | '\u{FEFF}'
            )
        })
        .collect::<String>()
}

fn proxy_url_contract_ok(url: &str) -> bool {
    if url.contains('\0') || url.contains('\r') || url.contains('\n') || url.contains("..") {
        return false;
    }
    let folded = url.to_ascii_lowercase();
    if !(folded.starts_with("http://") || folded.starts_with("https://")) || folded.contains(' ') {
        return false;
    }
    let without_scheme = folded
        .strip_prefix("http://")
        .or_else(|| folded.strip_prefix("https://"))
        .unwrap_or_default();
    if without_scheme.is_empty() || without_scheme.starts_with('/') {
        return false;
    }
    let authority = without_scheme.split('/').next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    authority.chars().any(|ch| ch.is_ascii_alphanumeric())
}

fn normalize_proxy_env_value(raw: Option<&str>) -> Option<String> {
    let sanitized = strip_invisible_unicode(raw.unwrap_or(""))
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .collect::<String>();
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        None
    } else if !proxy_url_contract_ok(trimmed) {
        None
    } else {
        Some(trimmed.chars().take(256).collect::<String>())
    }
}

pub fn resolve_memory_abstraction_proxy_url(
    protocol: &str,
    http_proxy_lower: Option<&str>,
    https_proxy_lower: Option<&str>,
    http_proxy_upper: Option<&str>,
    https_proxy_upper: Option<&str>,
) -> Option<String> {
    let lower_http = normalize_proxy_env_value(http_proxy_lower);
    let lower_https = normalize_proxy_env_value(https_proxy_lower);
    let upper_http = normalize_proxy_env_value(http_proxy_upper);
    let upper_https = normalize_proxy_env_value(https_proxy_upper);

    let http_proxy = lower_http.or(upper_http);
    let https_proxy = lower_https.or(upper_https);
    if protocol.eq_ignore_ascii_case("https") {
        https_proxy.or(http_proxy)
    } else {
        http_proxy
    }
}

pub fn resolve_memory_abstraction_proxy_url_with_contract(
    protocol: &str,
    http_proxy_lower: Option<&str>,
    https_proxy_lower: Option<&str>,
    http_proxy_upper: Option<&str>,
    https_proxy_upper: Option<&str>,
    strict_contract: bool,
) -> (Option<String>, bool, &'static str) {
    let normalized_protocol = protocol.trim().to_ascii_lowercase();
    if strict_contract && normalized_protocol != "http" && normalized_protocol != "https" {
        return (
            None,
            false,
            "proxy_protocol_unsupported_under_strict_contract",
        );
    }
    let resolved = resolve_memory_abstraction_proxy_url(
        protocol,
        http_proxy_lower,
        https_proxy_lower,
        http_proxy_upper,
        https_proxy_upper,
    );
    if strict_contract && resolved.is_none() {
        return (None, false, "proxy_url_missing_under_strict_contract");
    }
    let reason = if resolved.is_some() {
        "proxy_url_contract_ok"
    } else {
        "proxy_url_missing_non_strict_contract"
    };
    (resolved, true, reason)
}

pub fn memory_abstraction_has_proxy_for_protocol(
    protocol: &str,
    http_proxy_lower: Option<&str>,
    https_proxy_lower: Option<&str>,
    http_proxy_upper: Option<&str>,
    https_proxy_upper: Option<&str>,
) -> bool {
    resolve_memory_abstraction_proxy_url(
        protocol,
        http_proxy_lower,
        https_proxy_lower,
        http_proxy_upper,
        https_proxy_upper,
    )
    .is_some()
}

#[cfg(test)]
mod assim120_memory_abstraction_tests {
    use super::*;

    #[test]
    fn lowercase_proxy_env_takes_precedence() {
        let out = resolve_memory_abstraction_proxy_url(
            "https",
            Some("http://lower-http:8080"),
            Some("http://lower-https:8080"),
            Some("http://upper-http:8080"),
            Some("http://upper-https:8080"),
        );
        assert_eq!(out.as_deref(), Some("http://lower-https:8080"));
    }

    #[test]
    fn https_falls_back_to_http_proxy_when_https_missing() {
        let out = resolve_memory_abstraction_proxy_url(
            "https",
            Some("http://lower-http:8080"),
            None,
            None,
            None,
        );
        assert_eq!(out.as_deref(), Some("http://lower-http:8080"));
        assert!(memory_abstraction_has_proxy_for_protocol(
            "https",
            Some("http://lower-http:8080"),
            None,
            None,
            None
        ));
    }

    #[test]
    fn invalid_proxy_schemes_are_rejected() {
        let out = resolve_memory_abstraction_proxy_url(
            "https",
            Some("javascript:alert(1)"),
            Some("ftp://proxy"),
            None,
            None,
        );
        assert!(out.is_none());
    }

    #[test]
    fn strict_contract_requires_supported_protocol() {
        let (_url, ok, reason) = resolve_memory_abstraction_proxy_url_with_contract(
            "socks5",
            Some("http://proxy"),
            None,
            None,
            None,
            true,
        );
        assert!(!ok);
        assert_eq!(reason, "proxy_protocol_unsupported_under_strict_contract");
    }
}
