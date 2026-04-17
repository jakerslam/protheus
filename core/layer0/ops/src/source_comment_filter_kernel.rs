// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Imported pattern contract (RTK intake):
// - source: local/workspace/vendor/rtk/src/core/filter.rs
// - concept: language-aware filter levels for comment/boilerplate compaction.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::path::Path;
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FilterLevel {
    None,
    Minimal,
    Aggressive,
}

impl FilterLevel {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "none" => Self::None,
            "aggressive" => Self::Aggressive,
            _ => Self::Minimal,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Minimal => "minimal",
            Self::Aggressive => "aggressive",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Shell,
    Data,
    Unknown,
}

impl Language {
    fn canonical_extension(ext: &str) -> String {
        let mut token = ext.trim().trim_start_matches('.').to_ascii_lowercase();
        if token.is_empty() {
            return token;
        }
        if token.ends_with("rc") && matches!(token.as_str(), "bashrc" | "zshrc" | "profile") {
            return "sh".to_string();
        }
        token = token.replace('-', "");
        match token.as_str() {
            "jsx" | "node" => "js".to_string(),
            "mts" | "cts" => "ts".to_string(),
            "markdown" => "md".to_string(),
            _ => token,
        }
    }
    fn from_extension(ext: &str) -> Self {
        match Self::canonical_extension(ext).as_str() {
            "rs" => Self::Rust,
            "py" | "pyw" => Self::Python,
            "js" | "mjs" | "cjs" => Self::JavaScript,
            "ts" | "tsx" => Self::TypeScript,
            "go" => Self::Go,
            "sh" | "bash" | "zsh" => Self::Shell,
            "json" | "yaml" | "yml" | "toml" | "xml" | "csv" | "tsv" | "md" | "txt" => Self::Data,
            _ => Self::Unknown,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::Shell => "shell",
            Self::Data => "data",
            Self::Unknown => "unknown",
        }
    }
}

fn infer_extension_from_path(raw: &str) -> String {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return String::new();
    }
    let path = Path::new(candidate);
    path.extension()
        .and_then(|v| v.to_str())
        .map(Language::canonical_extension)
        .unwrap_or_default()
}

fn resolve_input_language(input: &Map<String, Value>) -> (Language, String, String) {
    let explicit_ext = clean_text(input.get("extension").and_then(Value::as_str).unwrap_or(""), 24);
    let explicit_lang = clean_text(input.get("language").and_then(Value::as_str).unwrap_or(""), 32);
    let file_path = clean_text(input.get("file_path").and_then(Value::as_str).unwrap_or(""), 520);

    if !explicit_lang.is_empty() {
        let lang = Language::from_extension(&explicit_lang);
        if lang != Language::Unknown {
            return (
                lang,
                Language::canonical_extension(&explicit_lang),
                "language".to_string(),
            );
        }
    }
    if !explicit_ext.is_empty() {
        let ext = Language::canonical_extension(&explicit_ext);
        return (Language::from_extension(&ext), ext, "extension".to_string());
    }
    let inferred = infer_extension_from_path(&file_path);
    if !inferred.is_empty() {
        return (
            Language::from_extension(&inferred),
            inferred,
            "file_path".to_string(),
        );
    }
    (Language::Unknown, String::new(), "default".to_string())
}

#[derive(Clone, Copy)]
struct CommentPatterns {
    line: Option<&'static str>,
    block_start: Option<&'static str>,
    block_end: Option<&'static str>,
    doc_line: Option<&'static str>,
}

fn patterns(lang: Language) -> CommentPatterns {
    match lang {
        Language::Rust => CommentPatterns {
            line: Some("//"),
            block_start: Some("/*"),
            block_end: Some("*/"),
            doc_line: Some("///"),
        },
        Language::Python => CommentPatterns {
            line: Some("#"),
            block_start: Some("\"\"\""),
            block_end: Some("\"\"\""),
            doc_line: None,
        },
        Language::JavaScript | Language::TypeScript | Language::Go => CommentPatterns {
            line: Some("//"),
            block_start: Some("/*"),
            block_end: Some("*/"),
            doc_line: None,
        },
        Language::Shell => CommentPatterns {
            line: Some("#"),
            block_start: None,
            block_end: None,
            doc_line: None,
        },
        Language::Data => CommentPatterns {
            line: None,
            block_start: None,
            block_end: None,
            doc_line: None,
        },
        Language::Unknown => CommentPatterns {
            line: Some("//"),
            block_start: Some("/*"),
            block_end: Some("*/"),
            doc_line: None,
        },
    }
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn trailing_whitespace_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[ \t]+$").expect("valid trailing whitespace regex"))
}

fn import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^(use |import |from |#include|require\(|pub use )")
            .expect("valid import regex")
    })
}

fn signature_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^(pub\s+)?(async\s+)?(fn|def|function|class|struct|enum|trait|impl)\s+\w+")
            .expect("valid signature regex")
    })
}

fn normalize_blank_lines(lines: &[String]) -> String {
    let mut out = Vec::<String>::new();
    let mut blank_run = 0usize;
    for line in lines {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 2 {
                continue;
            }
            out.push(String::new());
            continue;
        }
        blank_run = 0;
        out.push(line.clone());
    }
    out.join("\n").trim().to_string()
}

fn minimal_filter(content: &str, lang: Language) -> String {
    if lang == Language::Data {
        return content.to_string();
    }
    let p = patterns(lang);
    let mut out = Vec::<String>::new();
    let mut in_block = false;
    for raw_line in content.lines() {
        let line = trailing_whitespace_re().replace(raw_line, "").to_string();
        let trimmed = line.trim();
        if in_block {
            if let Some(end) = p.block_end {
                if trimmed.contains(end) {
                    in_block = false;
                }
            }
            continue;
        }
        if let (Some(start), Some(end)) = (p.block_start, p.block_end) {
            if trimmed.contains(start) {
                if !trimmed.contains(end) || trimmed.find(start) < trimmed.find(end) {
                    in_block = true;
                }
                continue;
            }
        }
        if let Some(marker) = p.line {
            if trimmed.starts_with(marker) {
                if let Some(doc) = p.doc_line {
                    if trimmed.starts_with(doc) {
                        out.push(line);
                    }
                }
                continue;
            }
            if let Some(idx) = line.find(marker) {
                let keep = line[..idx].trim_end().to_string();
                if keep.is_empty() {
                    out.push(String::new());
                } else {
                    out.push(keep);
                }
                continue;
            }
        }
        out.push(line);
    }
    normalize_blank_lines(&out)
}

fn aggressive_filter(content: &str, lang: Language) -> String {
    if lang == Language::Data {
        return minimal_filter(content, lang);
    }
    let base = minimal_filter(content, lang);
    let mut out = Vec::<String>::new();
    for line in base.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let keep = trimmed.starts_with("#!")
            || import_re().is_match(trimmed)
            || signature_re().is_match(trimmed)
            || matches!(trimmed, "{" | "}" | "];" | ");")
            || trimmed.starts_with("let ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("pub const ");
        if keep {
            out.push(trimmed.to_string());
        }
    }
    normalize_blank_lines(&out)
}

fn apply_filter(content: &str, lang: Language, level: FilterLevel) -> String {
    match level {
        FilterLevel::None => content.to_string(),
        FilterLevel::Minimal => minimal_filter(content, lang),
        FilterLevel::Aggressive => aggressive_filter(content, lang),
    }
}

fn usage() {
    println!("source-comment-filter-kernel commands:");
    println!(
        "  protheus-ops source-comment-filter-kernel <filter|detect-language> [--payload=<json>|--payload-base64=<base64_json>]"
    );
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("source_comment_filter_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("source_comment_filter_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("source_comment_filter_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("source_comment_filter_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

pub fn run(_root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("source_comment_filter_kernel_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let (lang, extension, detected_from) = resolve_input_language(input);
    let response = match command.as_str() {
        "detect-language" => cli_receipt(
            "source_comment_filter_kernel_detect_language",
            json!({
              "ok": true,
              "extension": extension,
              "detected_from": detected_from,
              "language": lang.as_str()
            }),
        ),
        "filter" => {
            let content = clean_text(
                input.get("text").and_then(Value::as_str).unwrap_or(""),
                400_000,
            );
            let level = FilterLevel::from_str(
                input
                    .get("level")
                    .and_then(Value::as_str)
                    .unwrap_or("minimal"),
            );
            let filtered = apply_filter(&content, lang, level);
            let input_len = content.chars().count();
            let output_len = filtered.chars().count();
            let reduction_pct = if input_len == 0 {
                0.0
            } else {
                (((input_len.saturating_sub(output_len)) as f64) / (input_len as f64)) * 100.0
            };
            cli_receipt(
                "source_comment_filter_kernel_filter",
                json!({
                  "ok": true,
                  "language": lang.as_str(),
                  "level": level.as_str(),
                  "input_chars": input_len,
                  "output_chars": output_len,
                  "reduction_pct": reduction_pct,
                  "filtered_text": filtered
                }),
            )
        }
        _ => cli_error(
            "source_comment_filter_kernel_error",
            "source_comment_filter_kernel_unknown_command",
        ),
    };
    let ok = response.get("ok").and_then(Value::as_bool).unwrap_or(false);
    print_json_line(&response);
    if ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_filter_strips_rust_comments() {
        let src = "fn a() {\n  // note\n  let x = 1; // trailing\n}\n";
        let out = minimal_filter(src, Language::Rust);
        assert!(out.contains("let x = 1;"));
        assert!(!out.contains("// note"));
        assert!(!out.contains("trailing"));
    }

    #[test]
    fn aggressive_filter_keeps_signatures_and_imports() {
        let src = "use std::fs;\nfn alpha() {\n  let x = 1;\n  println!(\"{}\", x);\n}\n";
        let out = aggressive_filter(src, Language::Rust);
        assert!(out.contains("use std::fs;"));
        assert!(out.contains("fn alpha()"));
        assert!(!out.contains("println!"));
    }

    #[test]
    fn data_language_passes_through_minimal() {
        let src = "{\"ok\":true}\n";
        let out = minimal_filter(src, Language::Data);
        assert_eq!(out, src.trim());
    }
}
