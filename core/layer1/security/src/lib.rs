// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("lib_parts/010-parse-args.rs");
include!("lib_parts/020-verify-integrity-policy.rs");
include!("lib_parts/030-run-emergency-stop.rs");
include!("lib_parts/040-run-capability-lease.rs");
include!("lib_parts/050-run-startup-attestation.rs");
include!("lib_parts/060-web-conduit-policy.rs");

pub fn normalize_security_conduit_mode(raw: &str) -> String {
    let mut collapsed = String::new();
    let mut last_was_sep = false;
    for ch in raw.trim().chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if matches!(ch, '-' | '_' | ' ') {
            Some('_')
        } else {
            None
        };
        let Some(next) = normalized else { continue };
        if next == '_' {
            if last_was_sep {
                continue;
            }
            last_was_sep = true;
        } else {
            last_was_sep = false;
        }
        collapsed.push(next);
    }
    collapsed
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

pub fn resolve_security_conduit_mode_with_contract(
    raw: &str,
    strict_contract: bool,
) -> (String, bool, &'static str) {
    let normalized = normalize_security_conduit_mode(raw);
    if normalized.is_empty() {
        return (
            "strict".to_string(),
            false,
            "conduit_mode_empty_after_normalization",
        );
    }
    let is_known = matches!(
        normalized.as_str(),
        "strict" | "trusted_env_proxy" | "trusted" | "env_proxy"
    );
    if !is_known && strict_contract {
        return (
            "strict".to_string(),
            false,
            "conduit_mode_unknown_under_strict_contract",
        );
    }
    let resolved = if is_known {
        normalized
    } else {
        "strict".to_string()
    };
    let reason = if is_known {
        "conduit_mode_contract_ok"
    } else {
        "conduit_mode_fallback_to_strict_non_strict_contract"
    };
    (resolved, true, reason)
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

    #[test]
    fn strict_conduit_contract_rejects_unknown_mode() {
        let (mode, ok, reason) =
            resolve_security_conduit_mode_with_contract("custom-forward-proxy", true);
        assert_eq!(mode, "strict");
        assert!(!ok);
        assert_eq!(reason, "conduit_mode_unknown_under_strict_contract");
    }
}
