#[path = "../idle_dream_lane.rs"]
mod idle_dream_lane;
use serde_json::json;
use std::path::PathBuf;

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

fn sanitize_cli_token(raw: &str) -> String {
    strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .collect::<String>()
        .trim()
        .chars()
        .take(160)
        .collect::<String>()
}

fn sanitize_argv(raw_args: &[String]) -> Vec<String> {
    raw_args
        .iter()
        .map(|arg| sanitize_cli_token(arg))
        .collect::<Vec<String>>()
}

fn has_parent_component(path: &std::path::Path) -> bool {
    path.components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
}

fn resolve_repo_root() -> (PathBuf, Option<&'static str>) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let Some(raw_root) = std::env::var("PROTHEUS_ROOT").ok() else {
        return (cwd, None);
    };
    let sanitized_root = sanitize_cli_token(&raw_root);
    if sanitized_root.is_empty() {
        return (cwd, Some("protheus_root_empty"));
    }
    let candidate = PathBuf::from(&sanitized_root);
    if has_parent_component(&candidate) {
        return (cwd, Some("protheus_root_parent_blocked"));
    }
    let resolved = if candidate.is_absolute() {
        candidate
    } else {
        cwd.join(candidate)
    };
    (resolved, None)
}

fn main() {
    let args = sanitize_argv(&std::env::args().skip(1).collect::<Vec<String>>());
    let (repo_root, repo_root_warning) = resolve_repo_root();
    if let Some(code) = idle_dream_lane::maybe_run(&repo_root, &args) {
        std::process::exit(code);
    }
    eprintln!(
        "{}",
        json!({
            "ok": false,
            "type": "idle_dream_cycle_cli_error",
            "error": "unknown_command",
            "argv": args,
            "repo_root": repo_root.to_string_lossy(),
            "repo_root_warning": repo_root_warning
        })
    );
    std::process::exit(2);
}
