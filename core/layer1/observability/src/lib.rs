include!("lib_parts/010-mod-blob.rs");
include!("lib_parts/020-evaluate-trace-window.rs");
include!("lib_parts/030-into-c-string-ptr.rs");

const ASSIM122_TRACE_WINDOW_MIN_MS: u64 = 1;
const ASSIM122_TRACE_WINDOW_MAX_MS: u64 = 86_400_000;

pub fn assim122_normalize_trace_channel(raw: &str) -> String {
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
        .take(64)
        .collect::<String>()
}

pub fn assim122_trace_window_contract(window_ms: u64) -> (u64, bool, bool, &'static str) {
    let normalized = window_ms.clamp(ASSIM122_TRACE_WINDOW_MIN_MS, ASSIM122_TRACE_WINDOW_MAX_MS);
    let should_warn = normalized >= 3_600_000;
    let should_block = window_ms == 0;
    let reason = if should_block {
        "trace_window_zero_blocked"
    } else if should_warn {
        "trace_window_high_warn"
    } else {
        "trace_window_contract_ok"
    };
    (normalized, should_warn, should_block, reason)
}
