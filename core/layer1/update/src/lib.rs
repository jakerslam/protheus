// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeSet;

const MAX_VERSION_LEN: usize = 64;
const MAX_CAPABILITY_LEN: usize = 64;
const MAX_CAPABILITY_COUNT: usize = 128;
const MAX_UPDATE_SIZE_HARD_CAP_BYTES: u64 = 1_000_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePackage {
    pub version: String,
    pub artifact_sha256: String,
    pub size_bytes: u64,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePolicy {
    pub max_size_bytes: u64,
    pub required_capability: String,
    pub allow_prerelease: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateDecision {
    Approved,
    Rejected(String),
}

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                ch,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .collect()
}

fn sanitize_token(raw: &str, max_len: usize, lowercase: bool) -> String {
    let mut token: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    token = token.trim().to_string();
    if lowercase {
        token = token.to_ascii_lowercase();
    }
    if token.chars().count() > max_len {
        token = token.chars().take(max_len).collect();
    }
    token
}

fn valid_version(version: &str) -> bool {
    if version.is_empty() || version.len() > MAX_VERSION_LEN {
        return false;
    }
    if version.starts_with(['.', '-', '_']) || version.ends_with(['.', '-', '_']) {
        return false;
    }
    if !version.chars().any(|ch| ch.is_ascii_digit()) {
        return false;
    }
    version
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}

fn normalize_capabilities(values: &[String]) -> Vec<String> {
    let mut deduped = BTreeSet::<String>::new();
    for value in values {
        let normalized = sanitize_token(value, MAX_CAPABILITY_LEN, true);
        if !normalized.is_empty() {
            deduped.insert(normalized);
        }
    }
    deduped.into_iter().take(MAX_CAPABILITY_COUNT).collect()
}

impl UpdatePolicy {
    pub fn evaluate(&self, package: &UpdatePackage) -> UpdateDecision {
        let version = sanitize_token(&package.version, MAX_VERSION_LEN, false);
        let required_capability =
            sanitize_token(&self.required_capability, MAX_CAPABILITY_LEN, true);
        let capabilities = normalize_capabilities(&package.capabilities);
        let artifact_sha = sanitize_token(&package.artifact_sha256, 64, true);

        if self.max_size_bytes == 0 {
            return UpdateDecision::Rejected("policy_invalid_max_size".to_string());
        }
        if self.max_size_bytes > MAX_UPDATE_SIZE_HARD_CAP_BYTES {
            return UpdateDecision::Rejected("policy_max_size_exceeds_hard_cap".to_string());
        }
        if required_capability.is_empty() {
            return UpdateDecision::Rejected("policy_invalid_required_capability".to_string());
        }
        if !valid_version(&version) {
            return UpdateDecision::Rejected("version_missing".to_string());
        }
        if !self.allow_prerelease && version.contains('-') {
            return UpdateDecision::Rejected("prerelease_blocked".to_string());
        }
        if package.size_bytes == 0 {
            return UpdateDecision::Rejected("artifact_size_invalid".to_string());
        }
        if package.size_bytes > self.max_size_bytes {
            return UpdateDecision::Rejected("artifact_size_exceeds_policy".to_string());
        }
        if package.size_bytes > MAX_UPDATE_SIZE_HARD_CAP_BYTES {
            return UpdateDecision::Rejected("artifact_size_exceeds_hard_cap".to_string());
        }
        if !valid_sha256(&artifact_sha) {
            return UpdateDecision::Rejected("artifact_sha256_invalid".to_string());
        }
        if !capabilities.iter().any(|cap| cap == &required_capability) {
            return UpdateDecision::Rejected("missing_required_capability".to_string());
        }
        UpdateDecision::Approved
    }
}

fn valid_sha256(raw: &str) -> bool {
    raw.len() == 64 && raw.chars().all(|ch| ch.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{UpdateDecision, UpdatePackage, UpdatePolicy};

    fn policy() -> UpdatePolicy {
        UpdatePolicy {
            max_size_bytes: 64 * 1024 * 1024,
            required_capability: "update.apply".to_string(),
            allow_prerelease: false,
        }
    }

    #[test]
    fn update_policy_approves_valid_release_package() {
        let package = UpdatePackage {
            version: "1.2.3".to_string(),
            artifact_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            size_bytes: 8 * 1024 * 1024,
            capabilities: vec!["update.apply".to_string(), "status.read".to_string()],
        };
        assert_eq!(policy().evaluate(&package), UpdateDecision::Approved);
    }

    #[test]
    fn update_policy_rejects_invalid_hash_or_missing_capability() {
        let invalid_hash = UpdatePackage {
            version: "1.2.3".to_string(),
            artifact_sha256: "xyz".to_string(),
            size_bytes: 8 * 1024 * 1024,
            capabilities: vec!["update.apply".to_string()],
        };
        assert_eq!(
            policy().evaluate(&invalid_hash),
            UpdateDecision::Rejected("artifact_sha256_invalid".to_string())
        );

        let missing_capability = UpdatePackage {
            version: "1.2.3".to_string(),
            artifact_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            size_bytes: 8 * 1024 * 1024,
            capabilities: vec!["status.read".to_string()],
        };
        assert_eq!(
            policy().evaluate(&missing_capability),
            UpdateDecision::Rejected("missing_required_capability".to_string())
        );
    }

    #[test]
    fn update_policy_rejects_prerelease_when_blocked() {
        let package = UpdatePackage {
            version: "2.0.0-rc1".to_string(),
            artifact_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            size_bytes: 8 * 1024 * 1024,
            capabilities: vec!["update.apply".to_string()],
        };
        assert_eq!(
            policy().evaluate(&package),
            UpdateDecision::Rejected("prerelease_blocked".to_string())
        );
    }
}
