// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("lib_parts/010-parse-args.rs");
include!("lib_parts/020-verify-integrity-policy.rs");
include!("lib_parts/030-run-emergency-stop.rs");
include!("lib_parts/040-run-capability-lease.rs");
include!("lib_parts/050-run-startup-attestation.rs");
include!("lib_parts/060-web-conduit-policy.rs");

pub fn normalize_security_conduit_mode(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if matches!(ch, '-' | '_' | ' ') {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .replace("__", "_")
        .trim_matches('_')
        .chars()
        .take(48)
        .collect::<String>()
}

pub fn security_conduit_mode_is_trusted(raw: &str) -> bool {
    matches!(
        normalize_security_conduit_mode(raw).as_str(),
        "trusted_env_proxy" | "trusted" | "env_proxy"
    )
}

pub fn is_cross_origin_redirect_header_safe(header: &str) -> bool {
    matches!(
        header.trim().to_ascii_lowercase().as_str(),
        "accept"
            | "accept-encoding"
            | "accept-language"
            | "cache-control"
            | "content-language"
            | "content-type"
            | "if-match"
            | "if-modified-since"
            | "if-none-match"
            | "if-unmodified-since"
            | "pragma"
            | "range"
            | "user-agent"
    )
}

#[cfg(test)]
mod assim120_security_lib_tests {
    use super::*;

    #[test]
    fn conduit_mode_aliases_normalize() {
        assert_eq!(normalize_security_conduit_mode("Trusted-Env Proxy"), "trusted_env_proxy");
        assert!(security_conduit_mode_is_trusted("trusted-env-proxy"));
        assert!(!security_conduit_mode_is_trusted("strict"));
    }

    #[test]
    fn redirect_header_safety_filter_matches_policy() {
        assert!(is_cross_origin_redirect_header_safe("Accept"));
        assert!(!is_cross_origin_redirect_header_safe("Authorization"));
    }
}
