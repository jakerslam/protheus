fn resolve_release_check(root: &std::path::Path, force: bool, timeout_ms: u64) -> Value {
    let current_version = read_current_version(root);
    if !force {
        if let Some((checked_at_ms, cached_result)) = read_version_cache(root) {
            let now_ms = chrono::Utc::now().timestamp_millis();
            if now_ms - checked_at_ms < VERSION_CACHE_TTL_MS {
                let cache_version = normalize_version_text(
                    cached_result
                        .get("current_version")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                );
                if cache_version == normalize_version_text(&current_version) {
                    let mut merged = cached_result;
                    merged["source"] = Value::String(
                        clean(merged.get("source").and_then(Value::as_str).unwrap_or("cache"), 80),
                    );
                    merged["cache_hit"] = Value::Bool(true);
                    return merged;
                }
            }
        }
    }

    let prefer_prerelease = Version::parse(&normalize_version_text(&current_version))
        .ok()
        .map(|v| !v.pre.is_empty())
        .unwrap_or(false);
    let (local_latest, local_changelog, local_released_at) = read_release_channel_metadata(root);
    let remote = fetch_latest_release(timeout_ms, prefer_prerelease);

    let mut latest_version = current_version.clone();
    let mut changelog_line = String::new();
    let mut released_at = String::new();
    let mut source = "local_current".to_string();
    let mut check_warning = String::new();

    match remote {
        Ok((candidate, remote_source)) => {
            latest_version = normalize_version_text(&candidate.latest_version);
            changelog_line = clean(&candidate.changelog_line, 240);
            released_at = clean(&candidate.released_at, 40);
            source = remote_source;
        }
        Err(err) if !local_latest.is_empty() => {
            latest_version = local_latest;
            changelog_line = local_changelog;
            released_at = local_released_at;
            source = "release_channel".to_string();
            check_warning = err;
        }
        Err(err) => {
            check_warning = err;
        }
    }

    if latest_version.is_empty() {
        latest_version = current_version.clone();
    }
    let update_available = compare_versions(&latest_version, &current_version) == Ordering::Greater;
    let mut result = json!({
        "ok": true,
        "type": "infring_version_cli",
        "current_version": current_version,
        "latest_version": latest_version,
        "update_available": update_available,
        "changelog_line": changelog_line,
        "released_at": released_at,
        "source": clean(source, 80),
        "checked_at": crate::now_iso()
    });
    if !check_warning.is_empty() {
        result["check_warning"] = Value::String(clean(check_warning, 240));
    }
    write_version_cache(root, &result);
    result
}

fn emit_json_line(payload: &Value) {
    println!("{}", serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string()));
}

fn print_version_help() {
    println!("Usage: infring version|update|check-quiet [flags]");
    println!();
    println!("Commands:");
    println!("  version      Show local version and update signal.");
    println!("  update       Check latest release and print upgrade command.");
    println!("  check-quiet  Emit one-line notice only when update is available.");
    println!();
    println!("Flags:");
    println!("  --json        Print JSON payload.");
    println!("  --quiet       Suppress advisory notes.");
    println!("  --force       Bypass cache for remote release check.");
    println!("  --apply       Run installer command after check (`update` only).");
}

fn run_version_mode(root: &std::path::Path, opts: &VersionCliOpts) -> i32 {
    let check = resolve_release_check(root, opts.force, 1800);
    let mut payload = check.clone();
    payload["command"] = Value::String("version".to_string());
    if opts.json {
        emit_json_line(&payload);
        return 0;
    }
    let current = check.get("current_version").and_then(Value::as_str).unwrap_or("0.0.0-unknown");
    println!("infring {current}");
    if check.get("update_available").and_then(Value::as_bool) == Some(true) {
        let latest = check.get("latest_version").and_then(Value::as_str).unwrap_or(current);
        println!("[infring update] available: {latest} (current {current})");
        if let Some(changelog) = check.get("changelog_line").and_then(Value::as_str) {
            if !changelog.trim().is_empty() {
                println!("[infring update] {changelog}");
            }
        }
    }
    if !opts.quiet {
        if let Some(warning) = check.get("check_warning").and_then(Value::as_str) {
            if !warning.trim().is_empty() {
                println!("[infring update] note: {warning}");
            }
        }
    }
    0
}

fn run_update_mode(root: &std::path::Path, opts: &VersionCliOpts) -> i32 {
    let check = resolve_release_check(root, true, 2200);
    let mut apply_status = 0i32;
    if opts.apply {
        println!("[infring update] applying via: {VERSION_INSTALL_COMMAND}");
        let status = Command::new("sh")
            .arg("-c")
            .arg(VERSION_INSTALL_COMMAND)
            .current_dir(root)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();
        apply_status = status.ok().and_then(|row| row.code()).unwrap_or(1);
    }

    let mut payload = check.clone();
    payload["command"] = Value::String("update".to_string());
    payload["install_command"] = Value::String(VERSION_INSTALL_COMMAND.to_string());
    payload["apply_requested"] = Value::Bool(opts.apply);
    payload["apply_exit_code"] = Value::from(apply_status);

    if opts.json {
        emit_json_line(&payload);
        return apply_status;
    }

    let current = check.get("current_version").and_then(Value::as_str).unwrap_or("0.0.0-unknown");
    if check.get("update_available").and_then(Value::as_bool) == Some(true) {
        let latest = check.get("latest_version").and_then(Value::as_str).unwrap_or(current);
        println!("[infring update] update available: {latest} (current {current})");
        if let Some(changelog) = check.get("changelog_line").and_then(Value::as_str) {
            if !changelog.trim().is_empty() {
                println!("[infring update] {changelog}");
            }
        }
        if !opts.apply {
            println!("[infring update] install: {VERSION_INSTALL_COMMAND}");
        }
    } else {
        println!("[infring update] already up to date ({current})");
    }

    if !opts.quiet {
        if let Some(warning) = check.get("check_warning").and_then(Value::as_str) {
            if !warning.trim().is_empty() {
                println!("[infring update] note: {warning}");
            }
        }
    }
    apply_status
}

fn run_check_quiet_mode(root: &std::path::Path, opts: &VersionCliOpts) -> i32 {
    let check = resolve_release_check(root, opts.force, 1200);
    let mut payload = check.clone();
    payload["command"] = Value::String("check-quiet".to_string());
    if opts.json {
        emit_json_line(&payload);
        return 0;
    }
    if check.get("update_available").and_then(Value::as_bool) == Some(true) {
        let latest = check
            .get("latest_version")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let current = check
            .get("current_version")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        println!(
            "[infring update] Update available: {latest} (current {current}). Run: infring update"
        );
    }
    0
}

fn run_version_cli_domain(root: &std::path::Path, args: &[String]) -> i32 {
    let opts = parse_version_cli_args(args);
    let command = clean(&opts.command, 40).to_ascii_lowercase();
    match command.as_str() {
        "help" => {
            print_version_help();
            0
        }
        "version" => run_version_mode(root, &opts),
        "update" => run_update_mode(root, &opts),
        "check-quiet" => run_check_quiet_mode(root, &opts),
        _ => {
            let payload = json!({
                "ok": false,
                "type": "infring_version_cli",
                "error": "unknown_command",
                "command": clean(&command, 40)
            });
            if opts.json {
                emit_json_line(&payload);
            } else {
                eprintln!(
                    "[infring version] unknown subcommand: {}. Try: infring version --help",
                    payload.get("command").and_then(Value::as_str).unwrap_or("unknown")
                );
            }
            2
        }
    }
}

#[derive(Debug, Clone)]
struct ReleaseSemverContractOpts {
    command: String,
    write: bool,
    strict: bool,
    pretty: bool,
    channel: String,
}

impl Default for ReleaseSemverContractOpts {
    fn default() -> Self {
        Self {
            command: "run".to_string(),
            write: false,
            strict: false,
            pretty: true,
            channel: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ReleaseCommitRow {
    sha: String,
    subject: String,
    body: String,
}

fn release_semver_parse_bool(raw: &str, fallback: bool) -> bool {
    let token = clean(raw, 24).to_ascii_lowercase();
    if token.is_empty() {
        return fallback;
    }
    matches!(token.as_str(), "1" | "true" | "yes" | "on")
}

fn parse_release_semver_contract_args(argv: &[String]) -> ReleaseSemverContractOpts {
    let mut opts = ReleaseSemverContractOpts::default();
    let mut tokens = argv
        .iter()
        .map(|row| clean(row, 200))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();

    if let Some(first) = tokens.first() {
        if !first.starts_with("--") {
            opts.command = clean(first, 40).to_ascii_lowercase();
            tokens.remove(0);
        }
    }

    for token in tokens {
        if token == "--write" {
            opts.write = true;
        } else if let Some(value) = token.strip_prefix("--write=") {
            opts.write = release_semver_parse_bool(value, false);
        } else if token == "--strict" {
            opts.strict = true;
        } else if let Some(value) = token.strip_prefix("--strict=") {
            opts.strict = release_semver_parse_bool(value, false);
        } else if token == "--pretty" {
            opts.pretty = true;
        } else if let Some(value) = token.strip_prefix("--pretty=") {
            opts.pretty = release_semver_parse_bool(value, true);
        } else if let Some(value) = token.strip_prefix("--channel=") {
            opts.channel = clean(value, 40).to_ascii_lowercase();
        } else if matches!(token.as_str(), "--help" | "-h") {
            opts.command = "help".to_string();
        }
    }

    opts
}

fn release_semver_package_json_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("package.json")
}

fn release_semver_package_lock_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("package-lock.json")
}

fn release_semver_channel_policy_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("client")
        .join("runtime")
        .join("config")
        .join("release_channel_policy.json")
}

fn release_semver_runtime_version_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("client")
        .join("runtime")
        .join("config")
        .join("runtime_version.json")
}

fn release_semver_normalize_channel(raw: &str) -> String {
    let channel = clean(raw, 40).to_ascii_lowercase();
    if matches!(channel.as_str(), "alpha" | "beta" | "stable") {
        channel
    } else {
        "stable".to_string()
    }
}

fn release_semver_channel_policy_default(root: &std::path::Path) -> String {
    let path = release_semver_channel_policy_path(root);
    let configured = read_json_file(&path)
        .and_then(|row| {
            row.get("default_channel")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "stable".to_string());
    release_semver_normalize_channel(&configured)
}

fn release_semver_tag_for_channel(version: &str, channel: &str) -> String {
    let normalized = release_semver_normalize_channel(channel);
    if normalized == "stable" {
        format!("v{}", normalize_version_text(version))
    } else {
        format!("v{}-{normalized}", normalize_version_text(version))
    }
}

fn release_semver_run_git(root: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output();
    let Ok(output) = output else {
        return String::new();
    };
    if !output.status.success() {
        return String::new();
    }
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

