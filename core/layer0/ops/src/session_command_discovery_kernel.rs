// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Imported pattern contract (RTK intake):
// - source: local/workspace/vendor/rtk/src/discover/{lexer.rs,registry.rs,rules.rs,report.rs}
// - concept: rule-based shell command discovery and classification with deterministic report receipts.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use regex::{Regex, RegexSet};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SupportStatus {
    Existing,
    Passthrough,
}

impl SupportStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Existing => "existing",
            Self::Passthrough => "passthrough",
        }
    }
}

#[derive(Clone, Copy)]
struct DiscoverRule {
    pattern: &'static str,
    canonical: &'static str,
    category: &'static str,
    savings_pct: f64,
    subcmd_savings: &'static [(&'static str, f64)],
    subcmd_status: &'static [(&'static str, SupportStatus)],
}

include!("session_command_discovery_kernel_parts/010-rules.rs");

#[derive(Debug, PartialEq)]
enum Classification {
    Supported {
        command_key: String,
        canonical: &'static str,
        category: &'static str,
        savings_pct: f64,
        status: SupportStatus,
    },
    Unsupported {
        base_command: String,
    },
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    Arg,
    Operator,
    Pipe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedToken {
    kind: TokenKind,
    value: String,
    offset: usize,
}

fn regex_set() -> &'static RegexSet {
    static SET: OnceLock<RegexSet> = OnceLock::new();
    SET.get_or_init(|| RegexSet::new(RULES.iter().map(|row| row.pattern)).expect("valid regex set"))
}

fn compiled_rules() -> &'static Vec<Regex> {
    static COMPILED: OnceLock<Vec<Regex>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        RULES
            .iter()
            .map(|row| Regex::new(row.pattern).expect("valid regex"))
            .collect()
    })
}

fn env_prefix_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let dq = r#""(?:[^"\\]|\\.)*""#;
        let sq = r#"'(?:[^'\\]|\\.)*'"#;
        let uq = r#"[^\s]*"#;
        let val = format!("(?:{}|{}|{})", dq, sq, uq);
        let assign = format!(r#"[A-Z_][A-Z0-9_]*={}"#, val);
        Regex::new(&format!(r#"^(?:sudo\s+|env\s+|{}\s+)+"#, assign))
            .expect("valid env prefix regex")
    })
}

fn git_global_opt_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^(?:(?:-C\s+\S+|-c\s+\S+|--git-dir(?:=\S+|\s+\S+)|--work-tree(?:=\S+|\s+\S+)|--no-pager|--no-optional-locks|--bare|--literal-pathspecs)\s+)+").expect("valid git global opts regex")
    })
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn category_avg_tokens(category: &str, command_key: &str) -> usize {
    match category {
        "Git" => match command_key {
            "log" | "diff" | "show" => 200,
            _ => 40,
        },
        "Cargo" => match command_key {
            "test" => 500,
            _ => 150,
        },
        "Tests" => 800,
        "Files" => 100,
        "Build" => 300,
        "Infra" => 120,
        "Network" => 150,
        "GitHub" => 200,
        "PackageManager" => 150,
        "Python" => 220,
        _ => 150,
    }
}

fn tokenize_shell(input: &str) -> Vec<ParsedToken> {
    let mut out = Vec::<ParsedToken>::new();
    let mut current = String::new();
    let mut current_start = 0usize;
    let mut pos = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        let ch_len = ch.len_utf8();
        if escaped {
            current.push('\\');
            current.push(ch);
            pos += ch_len;
            escaped = false;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            if current.is_empty() {
                current_start = pos;
            }
            pos += ch_len;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            current.push(ch);
            pos += ch_len;
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            if current.is_empty() {
                current_start = pos;
            }
            current.push(ch);
            pos += ch_len;
            continue;
        }
        match ch {
            ';' => {
                flush_arg(&mut out, &mut current, current_start);
                out.push(ParsedToken {
                    kind: TokenKind::Operator,
                    value: ";".to_string(),
                    offset: pos,
                });
                pos += ch_len;
            }
            '|' => {
                flush_arg(&mut out, &mut current, current_start);
                let start = pos;
                pos += ch_len;
                if chars.peek() == Some(&'|') {
                    chars.next();
                    pos += 1;
                    out.push(ParsedToken {
                        kind: TokenKind::Operator,
                        value: "||".to_string(),
                        offset: start,
                    });
                } else {
                    out.push(ParsedToken {
                        kind: TokenKind::Pipe,
                        value: "|".to_string(),
                        offset: start,
                    });
                }
            }
            '&' => {
                flush_arg(&mut out, &mut current, current_start);
                let start = pos;
                pos += ch_len;
                if chars.peek() == Some(&'&') {
                    chars.next();
                    pos += 1;
                    out.push(ParsedToken {
                        kind: TokenKind::Operator,
                        value: "&&".to_string(),
                        offset: start,
                    });
                } else {
                    current.push('&');
                    current_start = start;
                }
            }
            c if c.is_whitespace() => {
                flush_arg(&mut out, &mut current, current_start);
                pos += c.len_utf8();
            }
            _ => {
                if current.is_empty() {
                    current_start = pos;
                }
                current.push(ch);
                pos += ch_len;
            }
        }
    }
    if escaped {
        current.push('\\');
    }
    flush_arg(&mut out, &mut current, current_start);
    out
}

fn flush_arg(tokens: &mut Vec<ParsedToken>, current: &mut String, offset: usize) {
    if current.is_empty() {
        return;
    }
    tokens.push(ParsedToken {
        kind: TokenKind::Arg,
        value: std::mem::take(current),
        offset,
    });
}

fn push_chain_segment(out: &mut Vec<String>, source: &str, start: usize, end: usize) {
    let segment = source[start..end].trim();
    if !segment.is_empty() {
        out.push(segment.to_string());
    }
}

fn split_command_chain(cmd: &str) -> Vec<String> {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return vec![];
    }
    if trimmed.contains("<<") || trimmed.contains("$((") {
        return vec![trimmed.to_string()];
    }
    let tokens = tokenize_shell(trimmed);
    let mut out = Vec::<String>::new();
    let mut seg_start = 0usize;
    for token in &tokens {
        match token.kind {
            TokenKind::Operator => {
                push_chain_segment(&mut out, trimmed, seg_start, token.offset);
                seg_start = token.offset + token.value.len();
            }
            TokenKind::Pipe => {
                push_chain_segment(&mut out, trimmed, seg_start, token.offset);
                return out;
            }
            TokenKind::Arg => {}
        }
    }
    push_chain_segment(&mut out, trimmed, seg_start, trimmed.len());
    out
}

fn strip_absolute_path(cmd: &str) -> String {
    let Some((first, rest)) = parse_first_token_with_rest(cmd) else {
        return cmd.to_string();
    };
    if first.starts_with('/') && first.contains('/') {
        if let Some(last) = first.rsplit('/').next() {
            if last
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
            {
                if rest.is_empty() {
                    return last.to_string();
                }
                return format!("{last} {rest}");
            }
        }
    }
    cmd.to_string()
}

fn strip_git_global_opts(cmd: &str) -> String {
    if !cmd.starts_with("git ") {
        return cmd.to_string();
    }
    let tail = cmd[4..].trim_start();
    let stripped = git_global_opt_regex().replace(tail, "");
    if stripped.trim().is_empty() {
        "git".to_string()
    } else {
        format!("git {}", stripped.trim())
    }
}

fn parse_first_token_with_rest(command: &str) -> Option<(String, String)> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }
    let first = trimmed.chars().next()?;
    if first == '"' || first == '\'' {
        if let Some(end) = trimmed[1..].find(first) {
            let token = trimmed[1..1 + end].to_string();
            let rest = trimmed[1 + end + 1..].trim().to_string();
            return Some((token, rest));
        }
        return Some((trimmed[1..].to_string(), String::new()));
    }
    let split_at = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let token = trimmed[..split_at].to_string();
    let rest = trimmed[split_at..].trim().to_string();
    Some((token, rest))
}

fn extract_base_command(cmd: &str) -> String {
    let normalized = strip_absolute_path(cmd);
    let Some((first, rest)) = parse_first_token_with_rest(&normalized) else {
        return String::new();
    };
    if let Some((second, _)) = parse_first_token_with_rest(&rest) {
        if !second.starts_with('-') && !second.contains('/') && !second.contains('.') {
            return format!("{first} {second}");
        }
    }
    first
}

fn normalize_explicit_tool_alias(token: &str) -> Option<&'static str> {
    let lower = token.trim().to_ascii_lowercase();
    let tool_name = lower
        .strip_prefix("tool::")
        .or_else(|| lower.strip_prefix("tool:"))?;
    match tool_name {
        "batch_query" | "batch-query" => Some("batch-query"),
        "web_search" | "search_web" | "web_query" | "web-query" => Some("web-search"),
        _ => None,
    }
}

fn subcmd_savings(rule: &DiscoverRule, subcmd: &str) -> Option<f64> {
    rule.subcmd_savings
        .iter()
        .find(|(label, _)| *label == subcmd)
        .map(|(_, pct)| *pct)
}

fn subcmd_status(rule: &DiscoverRule, subcmd: &str) -> Option<SupportStatus> {
    rule.subcmd_status
        .iter()
        .find(|(label, _)| *label == subcmd)
        .map(|(_, status)| *status)
}

fn classify_explicit_tool_alias(cmd: &str) -> Option<Classification> {
    let (first, _) = parse_first_token_with_rest(cmd)?;
    let alias = normalize_explicit_tool_alias(&first)?;
    match alias {
        "batch-query" => Some(Classification::Supported {
            command_key: "batch-query".to_string(),
            canonical: "infring batch-query",
            category: "Tooling",
            savings_pct: 92.0,
            status: SupportStatus::Existing,
        }),
        "web-search" => Some(Classification::Supported {
            command_key: "web-search".to_string(),
            canonical: "infring web search",
            category: "Network",
            savings_pct: 88.0,
            status: SupportStatus::Existing,
        }),
        _ => None,
    }
}

fn classify_command(raw: &str) -> Classification {
    let trimmed = raw.trim();
    if trimmed.is_empty() || IGNORED_EXACT.iter().any(|row| *row == trimmed) {
        return Classification::Ignored;
    }
    if IGNORED_PREFIXES
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
    {
        return Classification::Ignored;
    }
    let stripped_env = env_prefix_regex().replace(trimmed, "").to_string();
    let normalized = strip_git_global_opts(&strip_absolute_path(stripped_env.trim()));
    let cmd = normalized.trim();
    if cmd.is_empty() {
        return Classification::Ignored;
    }
    if (cmd.starts_with("cat ") || cmd.starts_with("head ") || cmd.starts_with("tail "))
        && cmd
            .split_whitespace()
            .skip(1)
            .any(|tok| tok.starts_with('>') || tok == "<" || tok.starts_with(">>"))
    {
        return Classification::Unsupported {
            base_command: cmd.split_whitespace().next().unwrap_or("cat").to_string(),
        };
    }
    if let Some(classified) = classify_explicit_tool_alias(cmd) {
        return classified;
    }

    let matches = regex_set().matches(cmd).into_iter().collect::<Vec<_>>();
    if let Some(&idx) = matches.last() {
        let rule = &RULES[idx];
        let mut command_key = extract_base_command(cmd);
        let mut savings = rule.savings_pct;
        let mut status = SupportStatus::Existing;
        if let Some(captures) = compiled_rules()[idx].captures(cmd) {
            if let Some(sub) = captures.get(1) {
                let subcmd = sub.as_str().trim().to_ascii_lowercase();
                if !subcmd.is_empty() {
                    command_key = subcmd.clone();
                }
                savings = subcmd_savings(rule, &subcmd).unwrap_or(savings);
                status = subcmd_status(rule, &subcmd).unwrap_or(status);
            }
        }
        return Classification::Supported {
            command_key,
            canonical: rule.canonical,
            category: rule.category,
            savings_pct: savings,
            status,
        };
    }
    let base = extract_base_command(cmd);
    if base.is_empty() {
        Classification::Ignored
    } else {
        Classification::Unsupported { base_command: base }
    }
}

fn classify_command_list(input: &[String], limit: usize) -> Value {
    let mut supported =
        BTreeMap::<String, (usize, &'static str, &'static str, f64, SupportStatus, usize)>::new();
    let mut unsupported = HashMap::<String, (usize, String)>::new();
    let mut ignored = 0usize;
    let mut total = 0usize;
    let mut seen = HashSet::<String>::new();

    for raw in input {
        for segment in split_command_chain(raw) {
            total += 1;
            match classify_command(&segment) {
                Classification::Supported {
                    command_key,
                    canonical,
                    category,
                    savings_pct,
                    status,
                } => {
                    let est_tokens = ((category_avg_tokens(category, &command_key) as f64)
                        * (savings_pct / 100.0))
                        .round() as usize;
                    let entry = supported
                        .entry(format!(
                            "{category}|{canonical}|{command_key}|{}",
                            status.as_str()
                        ))
                        .or_insert((0, canonical, category, savings_pct, status, 0));
                    entry.0 += 1;
                    entry.5 += est_tokens;
                }
                Classification::Unsupported { base_command } => {
                    let row = unsupported
                        .entry(base_command.clone())
                        .or_insert((0, clean_text(&segment, 220)));
                    row.0 += 1;
                }
                Classification::Ignored => {
                    ignored += 1;
                }
            }
        }
    }

    fn sort_and_limit(rows: &mut Vec<Value>, limit: usize) {
        rows.sort_by(|a, b| {
            let ac = a.get("count").and_then(Value::as_u64).unwrap_or(0);
            let bc = b.get("count").and_then(Value::as_u64).unwrap_or(0);
            bc.cmp(&ac)
        });
        rows.truncate(limit.max(1));
    }

    fn sum_row_key(rows: &[Value], key: &str) -> usize {
        rows.iter()
            .map(|row| row.get(key).and_then(Value::as_u64).unwrap_or(0) as usize)
            .sum::<usize>()
    }

    let mut supported_rows = supported
        .into_iter()
        .map(|(key, row)| {
            let command_key = key.split('|').nth(2).unwrap_or("").to_string();
            json!({
                "command": command_key,
                "count": row.0,
                "canonical": row.1,
                "category": row.2,
                "estimated_savings_tokens": row.5,
                "estimated_savings_pct": row.3,
                "status": row.4.as_str(),
            })
        })
        .collect::<Vec<_>>();
    sort_and_limit(&mut supported_rows, limit);

    let mut unsupported_rows = unsupported
        .into_iter()
        .map(|(base_command, row)| {
            json!({
                "base_command": base_command,
                "count": row.0,
                "example": row.1,
            })
        })
        .collect::<Vec<_>>();
    sort_and_limit(&mut unsupported_rows, limit);

    let supported_count = sum_row_key(&supported_rows, "count");
    let unsupported_count = sum_row_key(&unsupported_rows, "count");
    let total_estimated_savings_tokens = sum_row_key(&supported_rows, "estimated_savings_tokens");

    // Track unique commands from incoming payload for quick operator visibility.
    for row in input {
        let normalized = clean_text(row, 180);
        if !normalized.is_empty() {
            seen.insert(normalized);
        }
    }

    json!({
        "ok": true,
        "type": "session_command_discovery_report",
        "total_commands": total,
        "supported_count": supported_count,
        "unsupported_count": unsupported_count,
        "ignored_count": ignored,
        "total_estimated_savings_tokens": total_estimated_savings_tokens,
        "unique_input_commands": seen.len(),
        "supported": supported_rows,
        "unsupported": unsupported_rows,
    })
}

pub(crate) fn split_command_chain_for_kernel(raw: &str) -> Vec<String> {
    split_command_chain(raw)
}

pub(crate) fn classify_command_detail_for_kernel(raw: &str) -> Value {
    match classify_command(raw) {
        Classification::Supported {
            command_key,
            canonical,
            category,
            savings_pct,
            status,
        } => json!({
            "supported": true,
            "ignored": false,
            "command_key": command_key,
            "canonical": canonical,
            "category": category,
            "estimated_savings_pct": savings_pct,
            "status": status.as_str(),
        }),
        Classification::Unsupported { base_command } => json!({
            "supported": false,
            "ignored": false,
            "base_command": base_command,
        }),
        Classification::Ignored => json!({
            "supported": false,
            "ignored": true,
        }),
    }
}

pub(crate) fn classify_command_list_for_kernel(input: &[String], limit: usize) -> Value {
    classify_command_list(input, limit)
}
include!("session_command_discovery_kernel_parts/020-run-and-tests.rs");
