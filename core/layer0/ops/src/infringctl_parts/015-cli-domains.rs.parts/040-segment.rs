fn run_completion_domain(args: &[String]) -> i32 {
    let help_mode = args
        .iter()
        .any(|arg| matches!(arg.trim(), "--help" | "-h"));
    let json_mode = args
        .iter()
        .any(|arg| matches!(arg.trim(), "--json" | "--json=1"));
    if help_mode {
        println!("Usage: infring completion [--json]");
        println!("Prints completion candidates for core entry commands.");
        return 0;
    }
    if json_mode {
        emit_json_line(&json!({
            "ok": true,
            "type": "infring_completion",
            "commands": CORE_COMPLETION_COMMANDS
        }));
        return 0;
    }
    for row in CORE_COMPLETION_COMMANDS {
        println!("{row}");
    }
    0
}

fn run_repl_domain(root: &std::path::Path, args: &[String]) -> i32 {
    if args
        .iter()
        .any(|arg| matches!(arg.trim(), "--help" | "-h"))
    {
        println!("Usage: infring repl");
        println!("Lightweight REPL bootstrap for constrained installs.");
        return 0;
    }
    let status = crate::command_list_kernel::run(root, &[String::from("--mode=help")]);
    if status == 0 && std::io::stdin().is_terminal() {
        println!(
            "[infring repl] interactive shell is unavailable in slim runtime; showing command index."
        );
    }
    status
}

fn normalize_domain_subcommand(raw: Option<&String>, fallback: &str) -> String {
    let token = raw
        .map(|row| clean(row, 80).to_ascii_lowercase())
        .unwrap_or_else(|| fallback.to_string());
    if token.trim().is_empty() {
        fallback.to_string()
    } else {
        token
    }
}

fn normalize_resurrection_protocol_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 160))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["status".to_string()];
    }
    let mut head = rows[0].to_ascii_lowercase();
    let mut tail = rows[1..].to_vec();
    if head == "resurrection-protocol" || head == "resurrection_protocol" {
        head = tail
            .first()
            .map(|row| clean(row, 40).to_ascii_lowercase())
            .unwrap_or_else(|| "status".to_string());
        tail = tail.into_iter().skip(1).collect();
    }
    if head.trim().is_empty() {
        return vec!["status".to_string()];
    }
    if matches!(head.as_str(), "run" | "build" | "bundle") {
        return std::iter::once("checkpoint".to_string())
            .chain(tail)
            .collect();
    }
    if head == "verify" {
        return vec!["status".to_string()];
    }
    if matches!(head.as_str(), "status" | "restore") {
        return std::iter::once(head).chain(tail).collect();
    }
    std::iter::once(head).chain(tail).collect()
}

fn normalize_session_continuity_vault_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 160))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["status".to_string()];
    }
    let head = rows[0].to_ascii_lowercase();
    let tail = rows[1..].to_vec();
    if head == "session-continuity-vault" || head == "session_continuity_vault" {
        return normalize_session_continuity_vault_args(&tail);
    }
    if head.trim().is_empty() {
        return vec!["status".to_string()];
    }
    if head == "restore" {
        return std::iter::once("get".to_string()).chain(tail).collect();
    }
    if head == "archive" {
        return std::iter::once("put".to_string()).chain(tail).collect();
    }
    if head == "verify" {
        return std::iter::once("status".to_string()).chain(tail).collect();
    }
    std::iter::once(head).chain(tail).collect()
}

fn normalized_rust_hotpath_inventory_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 160))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["status".to_string()];
    }
    let first = rows[0].to_ascii_lowercase();
    let is_cmd = !first.starts_with("--");
    let command = if is_cmd { first } else { "status".to_string() };
    let normalized = if matches!(command.as_str(), "run" | "status" | "inventory") {
        command
    } else {
        "status".to_string()
    };
    let tail = if is_cmd {
        rows.into_iter().skip(1).collect::<Vec<_>>()
    } else {
        rows
    };
    std::iter::once(normalized).chain(tail).collect()
}

fn normalized_coverage_badge_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 200))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["run".to_string()];
    }
    let first = rows[0].to_ascii_lowercase();
    if first.starts_with("--") {
        return std::iter::once("run".to_string()).chain(rows).collect();
    }
    rows
}

fn normalized_local_runtime_partitioner_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 200))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["status".to_string()];
    }
    let first = rows[0].to_ascii_lowercase();
    let is_cmd = !first.starts_with("--");
    let normalized = if matches!(first.as_str(), "status" | "init" | "reset") {
        first
    } else {
        "status".to_string()
    };
    let tail = if is_cmd {
        rows.into_iter().skip(1).collect::<Vec<_>>()
    } else {
        rows
    };
    std::iter::once(normalized).chain(tail).collect()
}

fn normalized_top50_roi_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 160))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["run".to_string()];
    }
    let first = rows[0].to_ascii_lowercase();
    let is_cmd = !first.starts_with("--");
    let command = if is_cmd { first } else { "run".to_string() };
    let normalized = if matches!(command.as_str(), "run" | "queue" | "status") {
        command
    } else {
        "run".to_string()
    };
    let tail = if is_cmd {
        rows.into_iter().skip(1).collect::<Vec<_>>()
    } else {
        rows
    };
    std::iter::once(normalized).chain(tail).collect()
}

fn contains_token(args: &[String], token: &str) -> bool {
    args.iter()
        .any(|row| clean(row, 80).eq_ignore_ascii_case(token))
}

fn normalize_status_dashboard_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 200))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let has_web = contains_token(args, "--web") || contains_token(args, "web");
    if has_web {
        let filtered = rows
            .into_iter()
            .filter(|row| {
                !matches!(
                    row.as_str(),
                    "--dashboard" | "dashboard" | "--web" | "web"
                )
            })
            .collect::<Vec<_>>();
        return std::iter::once("start".to_string())
            .chain(filtered)
            .collect();
    }
    if rows.is_empty() {
        return vec!["status".to_string()];
    }
    let first = rows[0].to_ascii_lowercase();
    if first.starts_with("--") {
        return std::iter::once("status".to_string()).chain(rows).collect();
    }
    rows
}

