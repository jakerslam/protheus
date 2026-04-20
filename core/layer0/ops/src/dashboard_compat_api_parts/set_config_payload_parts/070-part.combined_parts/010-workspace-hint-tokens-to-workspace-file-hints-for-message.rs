fn workspace_hint_tokens(message: &str, limit: usize) -> Vec<String> {
    let mut tokens = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for raw in clean_text(message, 600)
        .to_ascii_lowercase()
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'))
    {
        let token = raw.trim();
        if token.len() < 3 {
            continue;
        }
        if matches!(
            token,
            "the"
                | "and"
                | "for"
                | "with"
                | "that"
                | "this"
                | "from"
                | "have"
                | "your"
                | "you"
                | "are"
                | "was"
                | "were"
                | "will"
                | "into"
                | "about"
                | "what"
                | "when"
                | "then"
                | "than"
                | "just"
                | "they"
                | "them"
                | "able"
                | "make"
                | "made"
                | "need"
                | "want"
                | "does"
                | "did"
                | "done"
                | "not"
                | "too"
                | "very"
                | "also"
                | "like"
                | "been"
                | "being"
                | "each"
                | "more"
                | "most"
                | "over"
                | "under"
                | "after"
                | "before"
                | "because"
                | "while"
                | "where"
                | "which"
                | "would"
                | "could"
                | "should"
        ) {
            continue;
        }
        if seen.insert(token.to_string()) {
            tokens.push(token.to_string());
            if tokens.len() >= limit.max(1) {
                break;
            }
        }
    }
    tokens
}

fn should_infer_workspace_hints(message: &str) -> bool {
    let lowered = clean_text(message, 600).to_ascii_lowercase();
    [
        "file",
        "files",
        "module",
        "code",
        "api",
        "function",
        "class",
        "refactor",
        "patch",
        "update",
        "fix",
        "test",
        "workspace",
        "repo",
        "project",
        "notes",
        "docs",
        "meeting",
    ]
    .iter()
    .any(|token| lowered.contains(token))
}

fn should_skip_workspace_hint_entry(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
    let ignored = [
        ".git",
        "node_modules",
        "target",
        "dist",
        "build",
        ".next",
        ".cache",
        "artifacts",
        "backups",
        "tmp",
    ];
    ignored.iter().any(|value| *value == name)
}

fn workspace_file_hints_for_message(
    root: &Path,
    row: Option<&Value>,
    message: &str,
    limit: usize,
) -> Vec<Value> {
    if !should_infer_workspace_hints(message) {
        return Vec::new();
    }
    let tokens = workspace_hint_tokens(message, 8);
    if tokens.is_empty() {
        return Vec::new();
    }
    let workspace_base = workspace_base_for_agent(root, row);
    if !workspace_base.exists() {
        return Vec::new();
    }
    let lowered_message = clean_text(message, 600).to_ascii_lowercase();
    let code_focus = lowered_message.contains("code")
        || lowered_message.contains("api")
        || lowered_message.contains("function")
        || lowered_message.contains("test")
        || lowered_message.contains("module")
        || lowered_message.contains("refactor");
    let mut scored = Vec::<(i64, String, Vec<String>)>::new();
    let mut scanned = 0usize;
    let max_scan = 2200usize;
    for entry in WalkDir::new(&workspace_base)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !should_skip_workspace_hint_entry(entry))
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        scanned += 1;
        if scanned > max_scan {
            break;
        }
        let path = entry.path();
        let rel = path.strip_prefix(&workspace_base).unwrap_or(path);
        let rel_text = rel.to_string_lossy().replace('\\', "/");
        let rel_lc = rel_text.to_ascii_lowercase();
        let mut score = 0i64;
        let mut matches = Vec::<String>::new();
        for token in &tokens {
            if rel_lc.contains(token) {
                score += 5;
                matches.push(token.clone());
            } else if rel_lc
                .rsplit('/')
                .next()
                .map(|tail| tail.starts_with(token))
                .unwrap_or(false)
            {
                score += 3;
                matches.push(token.clone());
            }
        }
        if score <= 0 {
            continue;
        }
        if code_focus {
            let ext = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if matches!(
                ext.as_str(),
                "rs" | "ts" | "tsx" | "py" | "go" | "java" | "kt" | "cpp" | "c" | "h"
            ) {
                score += 2;
            }
        }
        scored.push((score, rel_text, matches));
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.len().cmp(&b.1.len())));
    scored
        .into_iter()
        .take(limit.clamp(1, 8))
        .map(|(score, path, matches)| {
            let match_count = matches.len();
            json!({
                "path": path,
                "score": score,
                "matches": matches,
                "reason": format!("matched {} workspace keywords", match_count)
            })
        })
        .collect::<Vec<_>>()
}
