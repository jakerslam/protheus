// SPDX-License-Identifier: Apache-2.0
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CompressionReport {
    pub raw_input_bytes: usize,
    pub input_bytes: usize,
    pub encoded_units: usize,
    pub estimated_encoded_bytes: usize,
    pub ratio: f64,
    pub status: String,
    pub should_warn: bool,
    pub should_block: bool,
    pub sanitized: bool,
    pub truncated: bool,
}

const MAX_REPORT_INPUT_BYTES: usize = 128 * 1024;
const COMPRESSION_RATIO_WARN_ABOVE: f64 = 2.0;
const COMPRESSION_RATIO_BLOCK_ABOVE: f64 = 2.75;

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

fn sanitize_report_content(content: &str) -> (String, bool, bool) {
    let stripped = strip_invisible_unicode(content)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .collect::<String>();
    let truncated = stripped.len() > MAX_REPORT_INPUT_BYTES;
    let sanitized = stripped != content;
    let bounded = stripped.chars().take(MAX_REPORT_INPUT_BYTES).collect::<String>();
    (bounded, sanitized || truncated, truncated)
}

pub fn rle_encode(input: &[u8]) -> Vec<(u8, u16)> {
    if input.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<(u8, u16)> = Vec::new();
    let mut cur = input[0];
    let mut count: u16 = 1;
    for b in &input[1..] {
        if *b == cur && count < u16::MAX {
            count += 1;
            continue;
        }
        out.push((cur, count));
        cur = *b;
        count = 1;
    }
    out.push((cur, count));
    out
}

#[allow(dead_code)]
pub fn rle_decode(input: &[(u8, u16)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (byte, count) in input {
        for _ in 0..*count {
            out.push(*byte);
        }
    }
    out
}

pub fn report_for(content: &str) -> CompressionReport {
    let raw_input_bytes = content.len();
    let (sanitized_content, sanitized, truncated) = sanitize_report_content(content);
    let bytes = sanitized_content.as_bytes();
    let encoded = rle_encode(bytes);
    let estimated = encoded.len() * 3;
    let ratio = if bytes.is_empty() {
        1.0
    } else {
        (estimated as f64 / bytes.len() as f64).clamp(0.0, 64.0)
    };
    let should_warn = ratio >= COMPRESSION_RATIO_WARN_ABOVE;
    let should_block = ratio >= COMPRESSION_RATIO_BLOCK_ABOVE;
    let status = if should_block {
        "blocked"
    } else if should_warn {
        "warn"
    } else {
        "ok"
    };
    CompressionReport {
        raw_input_bytes,
        input_bytes: bytes.len(),
        encoded_units: encoded.len(),
        estimated_encoded_bytes: estimated,
        ratio,
        status: status.to_string(),
        should_warn,
        should_block,
        sanitized,
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_round_trip() {
        let src = b"aaabbbccccdd";
        let enc = rle_encode(src);
        let dec = rle_decode(&enc);
        assert_eq!(src.to_vec(), dec);
    }

    #[test]
    fn report_sanitizes_invisible_unicode() {
        let out = report_for("a\u{200B}b");
        assert_eq!(out.input_bytes, 2);
        assert!(out.sanitized);
    }

    #[test]
    fn report_blocks_pathological_ratio() {
        let out = report_for("abcdef");
        assert!(out.should_block);
        assert_eq!(out.status, "blocked");
    }
}
