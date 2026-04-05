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

const RULES: &[DiscoverRule] = &[
    DiscoverRule {
        pattern: r"^git\s+(?:-[Cc]\s+\S+\s+)*(status|log|diff|show|add|commit|push|pull|branch|fetch|stash|worktree)",
        canonical: "infring git",
        category: "Git",
        savings_pct: 70.0,
        subcmd_savings: &[
            ("diff", 80.0),
            ("show", 80.0),
            ("add", 59.0),
            ("commit", 59.0),
        ],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^gh\s+(pr|issue|run|repo|api|release)",
        canonical: "infring github",
        category: "GitHub",
        savings_pct: 82.0,
        subcmd_savings: &[("pr", 87.0), ("run", 82.0), ("issue", 80.0)],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^cargo\s+(build|test|clippy|check|fmt|install)",
        canonical: "infring cargo",
        category: "Cargo",
        savings_pct: 80.0,
        subcmd_savings: &[("test", 90.0), ("check", 80.0)],
        subcmd_status: &[("fmt", SupportStatus::Passthrough)],
    },
    DiscoverRule {
        pattern: r"^(cat|head|tail)\s+",
        canonical: "infring read",
        category: "Files",
        savings_pct: 60.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^(rg|grep)\s+",
        canonical: "infring grep",
        category: "Files",
        savings_pct: 75.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^ls(\s|$)",
        canonical: "infring ls",
        category: "Files",
        savings_pct: 65.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^find\s+",
        canonical: "infring find",
        category: "Files",
        savings_pct: 70.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^tree(\s|$)",
        canonical: "infring tree",
        category: "Files",
        savings_pct: 70.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^diff\s+",
        canonical: "infring diff",
        category: "Files",
        savings_pct: 60.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^curl\s+",
        canonical: "infring web fetch",
        category: "Network",
        savings_pct: 70.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^wget\s+",
        canonical: "infring web fetch",
        category: "Network",
        savings_pct: 65.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^(pnpm|npm|npx)\s+",
        canonical: "infring npm",
        category: "PackageManager",
        savings_pct: 70.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^(python\s+-m\s+)?pytest(\s|$)",
        canonical: "infring pytest",
        category: "Tests",
        savings_pct: 90.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^ruff\s+(check|format)",
        canonical: "infring ruff",
        category: "Python",
        savings_pct: 80.0,
        subcmd_savings: &[("format", 75.0), ("check", 80.0)],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^docker\s+(ps|images|logs|run|exec|build|compose\s+(ps|logs|build))",
        canonical: "infring docker",
        category: "Infra",
        savings_pct: 85.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^kubectl\s+(get|logs|describe|apply)",
        canonical: "infring kubectl",
        category: "Infra",
        savings_pct: 85.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
];

const IGNORED_EXACT: &[&str] = &["", "exit", "clear", "pwd", "history", "reset"];
const IGNORED_PREFIXES: &[&str] = &["echo ", "printf ", "export ", "alias "];

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
                let segment = trimmed[seg_start..token.offset].trim();
                if !segment.is_empty() {
                    out.push(segment.to_string());
                }
                seg_start = token.offset + token.value.len();
            }
            TokenKind::Pipe => {
                let segment = trimmed[seg_start..token.offset].trim();
                if !segment.is_empty() {
                    out.push(segment.to_string());
                }
                return out;
            }
            TokenKind::Arg => {}
        }
    }
    let segment = trimmed[seg_start..].trim();
    if !segment.is_empty() {
        out.push(segment.to_string());
    }
    out
}

fn strip_absolute_path(cmd: &str) -> String {
    let mut parts = cmd.splitn(2, char::is_whitespace);
    let first = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");
    if first.starts_with('/') && first.contains('/') {
        if let Some(last) = first.rsplit('/').next() {
            if last
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
            {
                if rest.trim().is_empty() {
                    return last.to_string();
                }
                return format!("{last} {}", rest.trim());
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

fn extract_base_command(cmd: &str) -> String {
    let parts = cmd.split_whitespace().collect::<Vec<_>>();
    if parts.is_empty() {
        return String::new();
    }
    if parts.len() >= 2 {
        let second = parts[1];
        if !second.starts_with('-') && !second.contains('/') && !second.contains('.') {
            return format!("{} {}", parts[0], second);
        }
    }
    parts[0].to_string()
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
                if let Some((_, pct)) = rule
                    .subcmd_savings
                    .iter()
                    .find(|(label, _)| *label == subcmd)
                {
                    savings = *pct;
                }
                if let Some((_, mapped)) = rule
                    .subcmd_status
                    .iter()
                    .find(|(label, _)| *label == subcmd)
                {
                    status = *mapped;
                }
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
    supported_rows.sort_by(|a, b| {
        let ac = a.get("count").and_then(Value::as_u64).unwrap_or(0);
        let bc = b.get("count").and_then(Value::as_u64).unwrap_or(0);
        bc.cmp(&ac)
    });
    supported_rows.truncate(limit.max(1));

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
    unsupported_rows.sort_by(|a, b| {
        let ac = a.get("count").and_then(Value::as_u64).unwrap_or(0);
        let bc = b.get("count").and_then(Value::as_u64).unwrap_or(0);
        bc.cmp(&ac)
    });
    unsupported_rows.truncate(limit.max(1));

    let supported_count = supported_rows
        .iter()
        .map(|row| row.get("count").and_then(Value::as_u64).unwrap_or(0) as usize)
        .sum::<usize>();
    let unsupported_count = unsupported_rows
        .iter()
        .map(|row| row.get("count").and_then(Value::as_u64).unwrap_or(0) as usize)
        .sum::<usize>();
    let total_estimated_savings_tokens = supported_rows
        .iter()
        .map(|row| {
            row.get("estimated_savings_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize
        })
        .sum::<usize>();

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
