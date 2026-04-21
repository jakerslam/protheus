
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

fn is_wrapper_token(token: &str) -> bool {
    matches!(
        token.trim().to_ascii_lowercase().as_str(),
        "sudo" | "command" | "nohup" | "time" | "stdbuf" | "chrt" | "nice" | "ionice" | "timeout"
    )
}

fn is_env_assignment_token(token: &str) -> bool {
    let trimmed = token.trim();
    let Some((left, right)) = trimmed.split_once('=') else {
        return false;
    };
    !left.trim().is_empty() && !right.trim().is_empty()
}

fn strip_leading_wrappers(cmd: &str) -> String {
    let mut current = cmd.trim().to_string();
    let mut guard = 0usize;
    while !current.is_empty() && guard < 12 {
        guard += 1;
        let Some((first, rest)) = parse_first_token_with_rest(&current) else {
            break;
        };
        if is_wrapper_token(&first) {
            if rest.trim().is_empty() {
                break;
            }
            current = rest;
            continue;
        }
        if first.trim().eq_ignore_ascii_case("env") {
            if rest.trim().is_empty() {
                break;
            }
            let mut env_tail = rest;
            let mut advanced = false;
            loop {
                let Some((next, next_rest)) = parse_first_token_with_rest(&env_tail) else {
                    break;
                };
                let next_trimmed = next.trim();
                if next_trimmed.is_empty() {
                    break;
                }
                if next_trimmed.starts_with('-') || is_env_assignment_token(next_trimmed) {
                    advanced = true;
                    env_tail = next_rest;
                    if env_tail.trim().is_empty() {
                        break;
                    }
                    continue;
                }
                break;
            }
            if advanced && !env_tail.trim().is_empty() {
                current = env_tail;
                continue;
            }
            break;
        }
        break;
    }
    current
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
    let normalized = strip_absolute_path(&strip_leading_wrappers(cmd));
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
    let normalized = strip_git_global_opts(&strip_absolute_path(
        &strip_leading_wrappers(stripped_env.trim()),
    ));
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
