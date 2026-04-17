//! V4-SETTLE-001 Rust settle primitive scaffold.
//! Compile + memory-map + re-exec contract helpers.

use std::collections::BTreeMap;

const MAX_RUNTIME_HASH_LEN: usize = 128;
const MAX_TARGET_LEN: usize = 64;
const MAX_MODULE_LEN: usize = 64;
const MIN_MAPPED_BYTES: usize = 4096;
const MAX_MAPPED_BYTES: usize = 256 * 1024 * 1024;

fn strip_invisible_unicode(input: &str) -> String {
    input
        .chars()
        .filter(|c| !matches!(*c, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'))
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

fn normalize_token(input: &str, max_len: usize) -> String {
    strip_invisible_unicode(input)
        .trim()
        .chars()
        .take(max_len)
        .collect()
}

fn normalize_runtime_hash(input: &str) -> String {
    let filtered: String = normalize_token(input, MAX_RUNTIME_HASH_LEN)
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(MAX_RUNTIME_HASH_LEN)
        .collect();
    if filtered.is_empty() {
        "unknown".to_string()
    } else {
        filtered
    }
}

fn normalize_target(input: &str) -> String {
    let normalized: String = normalize_token(input, MAX_TARGET_LEN)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(*c, '-' | '_' | ':' | '.'))
        .take(MAX_TARGET_LEN)
        .collect();
    if normalized.is_empty() {
        "unspecified".to_string()
    } else {
        normalized
    }
}

fn normalize_module(module: Option<&str>) -> String {
    let normalized: String = module
        .map(|raw| normalize_token(raw, MAX_MODULE_LEN))
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(*c, '-' | '_'))
        .take(MAX_MODULE_LEN)
        .collect();
    if normalized.is_empty() {
        "core".to_string()
    } else {
        normalized
    }
}

fn normalize_map_size(size_hint: usize) -> usize {
    size_hint.clamp(MIN_MAPPED_BYTES, MAX_MAPPED_BYTES)
}

#[derive(Debug, Clone)]
pub struct SettleRequest {
    pub runtime_hash: String,
    pub target: String,
    pub module: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SettleReceipt {
    pub runtime_hash: String,
    pub target: String,
    pub mapped_bytes: usize,
    pub reexec_ready: bool,
    pub metadata: BTreeMap<String, String>,
}

pub fn compile_runtime_image(req: &SettleRequest) -> SettleReceipt {
    let runtime_hash = normalize_runtime_hash(req.runtime_hash.as_str());
    let target = normalize_target(req.target.as_str());
    let module = normalize_module(req.module.as_deref());
    let mut metadata = BTreeMap::new();
    metadata.insert("phase".into(), "compiled".into());
    metadata.insert("module".into(), module);
    metadata.insert("sanitized".into(), "true".into());

    SettleReceipt {
        runtime_hash,
        target,
        mapped_bytes: MIN_MAPPED_BYTES,
        reexec_ready: true,
        metadata,
    }
}

pub fn memory_map_image(receipt: &mut SettleReceipt, size_hint: usize) {
    receipt.mapped_bytes = normalize_map_size(size_hint);
    receipt.metadata.insert("phase".into(), "mapped".into());
}

pub fn health_check(receipt: &SettleReceipt) -> bool {
    receipt.reexec_ready
        && !receipt.runtime_hash.is_empty()
        && receipt.runtime_hash.chars().all(|c| c.is_ascii_hexdigit())
        && !receipt.target.is_empty()
        && receipt.mapped_bytes >= MIN_MAPPED_BYTES
        && receipt.mapped_bytes <= MAX_MAPPED_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settle_contract_smoke() {
        let req = SettleRequest {
            runtime_hash: "abc123".into(),
            target: "binary".into(),
            module: Some("autonomy".into()),
        };
        let mut receipt = compile_runtime_image(&req);
        memory_map_image(&mut receipt, 8192);
        assert!(health_check(&receipt));
    }

    #[test]
    fn sanitize_and_clamp_untrusted_inputs() {
        let req = SettleRequest {
            runtime_hash: " \u{200B}not-a-hash*** ".into(),
            target: " \u{200C}binary/root ".into(),
            module: Some(" \u{FEFF}a@utonomy ".into()),
        };
        let mut receipt = compile_runtime_image(&req);
        memory_map_image(&mut receipt, usize::MAX);
        assert_eq!(receipt.runtime_hash, "a");
        assert_eq!(receipt.target, "binaryroot");
        assert_eq!(receipt.metadata.get("module").unwrap(), "autonomy");
        assert_eq!(receipt.mapped_bytes, MAX_MAPPED_BYTES);
        assert!(health_check(&receipt));
    }
}
