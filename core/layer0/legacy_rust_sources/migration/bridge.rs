use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedTarget {
    pub input: String,
    pub slug: Option<String>,
    pub remote_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceTransfer {
    pub id: String,
    pub source: String,
    pub target: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationCheckpoint {
    pub migration_id: String,
    pub source_workspace: String,
    pub target_workspace: String,
    pub remote_before: Option<String>,
    pub remote_after: Option<String>,
    pub touched_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedReceipt {
    pub migration_id: String,
    pub event_type: String,
    pub key_id: String,
    pub signature: String,
}

fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|ch| {
            !matches!(
                *ch,
                '\u{200B}'
                    | '\u{200C}'
                    | '\u{200D}'
                    | '\u{2060}'
                    | '\u{FEFF}'
                    | '\u{202A}'
                    | '\u{202B}'
                    | '\u{202C}'
                    | '\u{202D}'
                    | '\u{202E}'
            ) && (!ch.is_control() || ch.is_ascii_whitespace())
        })
        .collect::<String>()
        .trim()
        .replace('\n', " ")
        .replace('\r', " ")
        .chars()
        .take(320)
        .collect::<String>()
}

fn normalize_slug(raw: &str) -> Option<String> {
    let cleaned = sanitize(raw).trim_end_matches(".git").trim().to_ascii_lowercase();
    let mut parts = cleaned.split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    if parts.next().is_some() {
        return None;
    }
    let is_valid = |value: &str| {
        !value.is_empty()
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    };
    if is_valid(owner) && is_valid(repo) {
        Some(format!("{owner}/{repo}"))
    } else {
        None
    }
}

pub fn normalize_repo_target(raw: &str) -> NormalizedTarget {
    let input = sanitize(raw);
    let cleaned = input.trim_end_matches(".git");

    let slug = if let Some(idx) = cleaned.find("github.com/") {
        let tail = &cleaned[idx + "github.com/".len()..];
        normalize_slug(tail)
    } else if cleaned.starts_with("git@") && cleaned.contains(':') {
        let tail = cleaned.split(':').nth(1).unwrap_or_default();
        normalize_slug(tail)
    } else if cleaned.split('/').count() == 2 && !cleaned.contains("://") {
        normalize_slug(cleaned)
    } else {
        None
    };

    let remote_url = match &slug {
        Some(value) => format!("https://github.com/{}.git", value),
        None => input.clone(),
    };

    NormalizedTarget {
        input,
        slug,
        remote_url,
    }
}

pub fn workspace_name_from_target(raw: &str) -> String {
    let normalized = normalize_repo_target(raw);
    if let Some(slug) = normalized.slug {
        let mut parts = slug.split('/');
        let _org = parts.next();
        if let Some(repo) = parts.next() {
            let candidate = repo.trim_end_matches(".git").trim();
            if !candidate.is_empty() {
                return candidate.to_string();
            }
        }
    }

    let fallback = normalized
        .remote_url
        .split('/')
        .last()
        .unwrap_or("infring-workspace")
        .trim_end_matches(".git")
        .trim();
    if fallback.is_empty() {
        "infring-workspace".to_string()
    } else {
        fallback.to_string()
    }
}

pub fn sign_receipt(migration_id: &str, event_type: &str, key_material: &str) -> SignedReceipt {
    let sanitized_key = sanitize(key_material);
    let key = if sanitized_key.is_empty() {
        "migration_dev_key".to_string()
    } else {
        sanitized_key
    };
    let key_id = short_hash(&format!("key:{}", key), 12);
    let signature = short_hash(
        &format!(
            "migration_id={}|event_type={}|key={}",
            sanitize(migration_id),
            sanitize(event_type),
            key
        ),
        48,
    );

    SignedReceipt {
        migration_id: sanitize(migration_id),
        event_type: sanitize(event_type),
        key_id,
        signature,
    }
}

pub fn short_hash(value: &str, width: usize) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    let raw = format!("{:016x}", hasher.finish());
    let width = width.min(256);
    if width == 0 {
        return raw;
    }
    let repeat = (width / raw.len()) + 1;
    raw.repeat(repeat).chars().take(width).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_slug_to_https_remote() {
        let n = normalize_repo_target("infring-labs/core");
        assert_eq!(n.slug, Some("infring-labs/core".to_string()));
        assert_eq!(n.remote_url, "https://github.com/infring-labs/core.git");
    }

    #[test]
    fn infers_workspace_name() {
        assert_eq!(workspace_name_from_target("acme/runtime"), "runtime");
        assert_eq!(workspace_name_from_target("https://github.com/acme/runtime.git"), "runtime");
    }

    #[test]
    fn generates_stable_signature() {
        let a = sign_receipt("migr_1", "run", "abc");
        let b = sign_receipt("migr_1", "run", "abc");
        assert_eq!(a.signature, b.signature);
        assert_eq!(a.key_id, b.key_id);
    }
}
