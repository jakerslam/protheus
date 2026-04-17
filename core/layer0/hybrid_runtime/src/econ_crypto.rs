use serde_json::json;
use sha2::{Digest, Sha256};

const MAX_LEDGER_LINES: usize = 512;
const MAX_LEDGER_LINE_CHARS: usize = 256;

pub fn checked_margin_bps(revenue_cents: i128, cost_cents: i128) -> Option<i64> {
    if revenue_cents <= 0 || cost_cents < 0 {
        return None;
    }
    let profit = revenue_cents.checked_sub(cost_cents)?;
    let scaled = profit.checked_mul(10_000)?;
    let bps = scaled.checked_div(revenue_cents)?;
    i64::try_from(bps).ok()
}

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

fn sanitize_ledger_line(raw: &str) -> String {
    strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .map(|ch| if ch.is_whitespace() { ' ' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(MAX_LEDGER_LINE_CHARS)
        .collect::<String>()
}

fn normalize_ledger_lines(lines: &[String]) -> (Vec<String>, usize, usize) {
    let mut normalized: Vec<String> = Vec::new();
    let mut dropped = 0usize;
    let mut truncated = 0usize;
    for line in lines.iter().take(MAX_LEDGER_LINES) {
        let out = sanitize_ledger_line(line);
        if out.is_empty() {
            dropped += 1;
            continue;
        }
        if out.len() < line.len() {
            truncated += 1;
        }
        normalized.push(out);
    }
    if lines.len() > MAX_LEDGER_LINES {
        dropped += lines.len() - MAX_LEDGER_LINES;
    }
    (normalized, dropped, truncated)
}

pub fn ledger_hash(lines: &[String]) -> String {
    let (normalized, _, _) = normalize_ledger_lines(lines);
    let mut h = Sha256::new();
    for (idx, line) in normalized.iter().enumerate() {
        h.update(idx.to_le_bytes());
        h.update(line.as_bytes());
        h.update([0u8]);
    }
    format!("{:x}", h.finalize())
}

pub fn sample_report() -> serde_json::Value {
    let revenue = 1_250_000_i128;
    let cost = 820_000_i128;
    let margin_bps = checked_margin_bps(revenue, cost);
    let lines = [
        "rev:1250000".to_string(),
        "cost:820000".to_string(),
        "ops:120000".to_string(),
        "ops:\u{200B}120000\u{0000}".to_string(),
    ];
    let hash = ledger_hash(&lines);
    let (normalized, dropped, truncated) = normalize_ledger_lines(&lines);

    json!({
        "ok": true,
        "lane": "V5-RUST-HYB-006",
        "economics": {
            "revenue_cents": revenue,
            "cost_cents": cost,
            "margin_bps": margin_bps
        },
        "integrity": {
            "ledger_hash": hash,
            "hash_alg": "sha256",
            "normalized_line_count": normalized.len(),
            "dropped_lines": dropped,
            "truncated_lines": truncated
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn margin_is_computed() {
        assert_eq!(checked_margin_bps(1000, 500), Some(5000));
    }

    #[test]
    fn invalid_inputs_fail() {
        assert_eq!(checked_margin_bps(0, 10), None);
        assert_eq!(checked_margin_bps(100, -1), None);
    }

    #[test]
    fn ledger_hash_ignores_invisible_and_control_chars() {
        let a = vec!["rev:100".to_string(), "cost:10".to_string()];
        let b = vec!["rev:\u{200B}100".to_string(), "cost:10\u{0000}".to_string()];
        assert_eq!(ledger_hash(&a), ledger_hash(&b));
    }
}
