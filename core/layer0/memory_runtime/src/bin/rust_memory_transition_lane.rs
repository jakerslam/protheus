#[path = "../lane_contracts.rs"]
mod lane_contracts;
#[path = "../transition_lane.rs"]
mod transition_lane;

use serde_json::json;
use std::path::PathBuf;

const MAX_ARGV_COUNT: usize = 64;

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
        .filter(|arg| !arg.is_empty())
        .take(MAX_ARGV_COUNT)
        .collect::<Vec<String>>()
}

fn argv_contract(args: &[String], raw_count: usize) -> (bool, &'static str) {
    if raw_count > MAX_ARGV_COUNT {
        return (false, "argv_count_exceeded");
    }
    if args.iter().any(|arg| arg.contains("..")) {
        return (false, "argv_parent_traversal_blocked");
    }
    (true, "argv_contract_ok")
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
    if candidate.is_absolute() {
        return (cwd, Some("protheus_root_absolute_blocked"));
    }
    let resolved = if candidate.is_absolute() {
        candidate
    } else {
        cwd.join(candidate)
    };
    (resolved, None)
}

fn main() {
    let raw_args = std::env::args().skip(1).collect::<Vec<String>>();
    let args = sanitize_argv(&raw_args);
    let (argv_ok, argv_reason) = argv_contract(&args, raw_args.len());
    let (repo_root, repo_root_warning) = resolve_repo_root();
    if !argv_ok {
        eprintln!(
            "{}",
            json!({
                "ok": false,
                "type": "rust_memory_transition_lane_cli_error",
                "error": argv_reason,
                "argv": args
            })
        );
        std::process::exit(2);
    }
    if let Some(code) = transition_lane::maybe_run(&repo_root, &args) {
        std::process::exit(code);
    }
    eprintln!(
        "{}",
        json!({
            "ok": false,
            "type": "rust_memory_transition_lane_cli_error",
            "error": "unknown_command",
            "argv": args,
            "argv_contract": {"ok": argv_ok, "reason": argv_reason},
            "repo_root": repo_root.to_string_lossy(),
            "repo_root_warning": repo_root_warning
        })
    );
    std::process::exit(2);
}
