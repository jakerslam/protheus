// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::json;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use crate::contract_lane_utils as lane_utils;

const LANE: &str = "spine_conduit_bridge_kernel";

fn usage() {
    println!("spine-conduit-bridge-kernel commands:");
    println!(
        "  protheus-ops spine-conduit-bridge-kernel run-domain --domain=<name> [--normalize-spine=1|0] -- <args...>"
    );
    println!(
        "  protheus-ops spine-conduit-bridge-kernel normalize-spine-args -- <args...>"
    );
}

fn normalize_spine_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| row.trim())
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["status".to_string()];
    }
    let head = rows[0].trim().to_ascii_lowercase();
    if head != "run" {
        return rows;
    }
    let mode_raw = rows.get(1).map(String::as_str).unwrap_or("").trim();
    let mode = if mode_raw.eq_ignore_ascii_case("eyes") {
        "eyes"
    } else {
        "daily"
    };
    let mut normalized = vec![mode.to_string()];
    let date_token = rows.get(2).map(String::as_str).unwrap_or("").trim();
    let has_date = date_token.len() == 10
        && date_token
            .chars()
            .enumerate()
            .all(|(idx, ch)| if idx == 4 || idx == 7 { ch == '-' } else { ch.is_ascii_digit() });
    let rest_start = if has_date { 3usize } else { 2usize };
    if has_date {
        normalized.push(date_token.to_string());
    }
    normalized.extend(rows.into_iter().skip(rest_start));
    normalized
}

fn collect_passthrough(args: &[String]) -> Vec<String> {
    if let Some(idx) = args.iter().position(|row| row.trim() == "--") {
        return args
            .iter()
            .skip(idx + 1)
            .map(|row| row.trim())
            .filter(|row| !row.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
    }

    let mut out = Vec::new();
    let mut i = 0usize;
    while i < args.len() {
        let token = args[i].trim();
        if token.is_empty() {
            i += 1;
            continue;
        }
        if token.starts_with("--domain=") || token.starts_with("--normalize-spine=") {
            i += 1;
            continue;
        }
        if token == "--domain" || token == "--normalize-spine" {
            if let Some(next) = args.get(i + 1) {
                if !next.trim_start().starts_with("--") {
                    i += 2;
                    continue;
                }
            }
            i += 1;
            continue;
        }
        out.push(token.to_string());
        i += 1;
    }
    out
}

fn resolve_command_and_args(domain: &str) -> (String, Vec<String>) {
    let explicit = std::env::var("PROTHEUS_OPS_BIN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if let Some(cmd) = explicit {
        return (cmd, vec![domain.to_string()]);
    }
    if let Ok(current) = std::env::current_exe() {
        return (
            current.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }
    (
        "cargo".to_string(),
        vec![
            "run".to_string(),
            "--quiet".to_string(),
            "-p".to_string(),
            "protheus-ops-core".to_string(),
            "--bin".to_string(),
            "protheus-ops".to_string(),
            "--".to_string(),
            domain.to_string(),
        ],
    )
}

fn run_domain(root: &Path, domain: &str, args: &[String]) -> Result<i32, String> {
    let clean_domain = lane_utils::clean_token(Some(domain), "spine");
    let normalize_spine = lane_utils::parse_bool(
        lane_utils::parse_flag(args, "normalize-spine", true).as_deref(),
        clean_domain.eq("spine"),
    );
    let mut pass_args = collect_passthrough(args);
    if clean_domain == "spine" && normalize_spine {
        pass_args = normalize_spine_args(&pass_args);
    }

    let (command, mut command_args) = resolve_command_and_args(&clean_domain);
    command_args.extend(pass_args);

    let run = Command::new(&command)
        .args(&command_args)
        .current_dir(root)
        .output()
        .map_err(|err| format!("{LANE}_spawn_failed:{err}"))?;

    if !run.stdout.is_empty() {
        io::stdout()
            .write_all(&run.stdout)
            .map_err(|err| format!("{LANE}_stdout_write_failed:{err}"))?;
    }
    if !run.stderr.is_empty() {
        io::stderr()
            .write_all(&run.stderr)
            .map_err(|err| format!("{LANE}_stderr_write_failed:{err}"))?;
    }

    let status = run.status.code().unwrap_or(1);
    Ok(status)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    match command.as_str() {
        "normalize-spine-args" => {
            let normalized = normalize_spine_args(&collect_passthrough(argv));
            let payload = json!({
                "ok": true,
                "args": normalized
            });
            lane_utils::print_json_line(&lane_utils::cli_receipt(LANE, payload));
            0
        }
        "run-domain" => {
            let domain = lane_utils::parse_flag(argv, "domain", true)
                .unwrap_or_else(|| "spine".to_string());
            match run_domain(root, &domain, &argv[1..]) {
                Ok(status) => status,
                Err(err) => {
                    lane_utils::print_json_line(&lane_utils::cli_error(LANE, &err));
                    1
                }
            }
        }
        _ => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                LANE,
                &format!("{LANE}_unknown_command:{command}"),
            ));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_spine_defaults_to_status() {
        assert_eq!(normalize_spine_args(&[]), vec!["status".to_string()]);
    }

    #[test]
    fn normalize_spine_run_mode() {
        let args = vec![
            "run".to_string(),
            "eyes".to_string(),
            "2026-03-28".to_string(),
            "--limit=5".to_string(),
        ];
        assert_eq!(
            normalize_spine_args(&args),
            vec![
                "eyes".to_string(),
                "2026-03-28".to_string(),
                "--limit=5".to_string()
            ]
        );
    }

    #[test]
    fn collect_passthrough_after_double_dash() {
        let args = vec![
            "--domain=spine".to_string(),
            "--".to_string(),
            "run".to_string(),
            "daily".to_string(),
        ];
        assert_eq!(
            collect_passthrough(&args),
            vec!["run".to_string(), "daily".to_string()]
        );
    }
}
