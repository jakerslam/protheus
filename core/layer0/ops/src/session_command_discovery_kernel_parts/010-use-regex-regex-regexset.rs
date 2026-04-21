// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Imported pattern contract (RTK intake):
// - source: local/workspace/vendor/rtk/src/discover/{lexer.rs,registry.rs,rules.rs,report.rs}
// - concept: rule-based shell command discovery and classification with deterministic report receipts.

use regex::{Regex, RegexSet};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;

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

include!("010-rules.rs");

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
