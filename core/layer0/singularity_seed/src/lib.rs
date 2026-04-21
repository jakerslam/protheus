// Split from lib.rs into focused include parts for maintainability.
include!("lib_parts/010-prelude-and-shared.rs");
include!("lib_parts/020-blob-version-to-normalize-cycle-request.rs");
include!("lib_parts/030-normalize-cycle-request-with-contract-to-loop-blob-path.rs");
include!("lib_parts/040-default-states-to-freeze-seed.rs");
include!("lib_parts/050-guarded-cycle-to-show-seed-state-wasm.rs");
include!("lib_parts/060-mod-tests.rs");

pub fn assim122_normalize_seed_lane_id(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.') {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(96)
        .collect::<String>()
}

pub fn assim122_seed_lane_contract(raw: &str, strict_contract: bool) -> (String, bool, &'static str) {
    let normalized = assim122_normalize_seed_lane_id(raw);
    if normalized.is_empty() {
        return (normalized, false, "seed_lane_empty_after_normalization");
    }
    if strict_contract && normalized != raw.trim().to_ascii_lowercase() {
        return (
            normalized,
            false,
            "seed_lane_modified_under_strict_contract",
        );
    }
    (normalized, true, "seed_lane_contract_ok")
}
