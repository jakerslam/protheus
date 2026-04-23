fn release_semver_parse_triplet(raw: &str) -> Option<(u64, u64, u64, String)> {
    let normalized = normalize_version_text(raw);
    let parsed = Version::parse(&normalized).ok()?;
    Some((
        parsed.major,
        parsed.minor,
        parsed.patch,
        format!("{}.{}.{}", parsed.major, parsed.minor, parsed.patch),
    ))
}

fn release_semver_latest_tag(root: &std::path::Path) -> String {
    let output = release_semver_run_git(root, &["tag", "--list", "--sort=-v:refname", "v*"]);
    for row in output.lines() {
        let tag = clean(row, 120);
        if !tag.is_empty() && release_semver_parse_triplet(&tag).is_some() {
            return tag;
        }
    }
    String::new()
}

fn release_semver_base_version(root: &std::path::Path, previous_tag: &str) -> (u64, u64, u64, String) {
    if let Some(base) = release_semver_parse_triplet(previous_tag) {
        return base;
    }

    let package_version = read_json_file(&release_semver_package_json_path(root))
        .and_then(|row| {
            row.get("version")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "0.0.0".to_string());

    if let Some(base) = release_semver_parse_triplet(&package_version) {
        return base;
    }

    (0, 0, 0, "0.0.0".to_string())
}

fn release_semver_read_commits(root: &std::path::Path, range_expr: &str) -> Vec<ReleaseCommitRow> {
    let format = "%H%x1f%s%x1f%b%x1e";
    let mut args = vec!["log".to_string(), format!("--format={format}")];
    if !range_expr.trim().is_empty() {
        args.push(range_expr.to_string());
    }
    let ref_args = args.iter().map(String::as_str).collect::<Vec<_>>();
    let raw = release_semver_run_git(root, &ref_args);
    raw.split('\u{1e}')
        .map(str::trim)
        .filter(|row| !row.is_empty())
        .filter_map(|row| {
            let parts = row.split('\u{1f}').collect::<Vec<_>>();
            if parts.len() < 2 {
                return None;
            }
            let sha = clean(parts.first().copied().unwrap_or_default(), 80);
            let subject = clean(parts.get(1).copied().unwrap_or_default(), 400);
            let body = clean(parts.get(2).copied().unwrap_or_default(), 6000);
            if sha.is_empty() || subject.is_empty() {
                return None;
            }
            Some(ReleaseCommitRow { sha, subject, body })
        })
        .collect()
}

fn release_semver_is_release_chore(subject: &str) -> bool {
    let normalized = clean(subject, 220).to_ascii_lowercase();
    if !normalized.starts_with("chore(release):") {
        return false;
    }
    let remainder = normalized.trim_start_matches("chore(release):").trim();
    release_semver_parse_triplet(remainder).is_some()
}

fn release_semver_conventional_type(subject: &str) -> (String, bool) {
    let row = clean(subject, 260);
    let head = row.split(':').next().unwrap_or_default().trim();
    if head.is_empty() {
        return (String::new(), false);
    }
    let mut lowered = head.to_ascii_lowercase();
    let breaking = lowered.ends_with('!');
    if breaking {
        lowered.pop();
    }
    let ty = lowered
        .split_once('(')
        .map(|(prefix, _)| prefix.to_string())
        .unwrap_or(lowered)
        .trim()
        .to_string();
    if ty.is_empty() {
        (String::new(), breaking)
    } else {
        (ty, breaking)
    }
}

fn release_semver_is_breaking_change(subject: &str, body: &str) -> bool {
    let (_ty, breaking_bang) = release_semver_conventional_type(subject);
    if breaking_bang {
        return true;
    }
    let upper = clean(body, 6000).to_ascii_uppercase();
    upper.contains("BREAKING CHANGE:")
        || upper.contains("BREAKING_CHANGE:")
        || upper.contains("BREAKING-CHANGE:")
}

fn release_semver_classify_bump(commits: &[ReleaseCommitRow]) -> String {
    let mut saw_minor = false;
    let mut saw_patch = false;
    for row in commits {
        if release_semver_is_release_chore(&row.subject) {
            continue;
        }
        if release_semver_is_breaking_change(&row.subject, &row.body) {
            return "major".to_string();
        }
        let (ty, _breaking_bang) = release_semver_conventional_type(&row.subject);
        if ty == "feat" {
            saw_minor = true;
        } else {
            saw_patch = true;
        }
    }
    if saw_minor {
        "minor".to_string()
    } else if saw_patch {
        "patch".to_string()
    } else {
        "none".to_string()
    }
}

fn release_semver_bump_version(base: &(u64, u64, u64, String), bump: &str) -> String {
    let (major, minor, patch, _normalized) = base;
    match bump {
        "major" => format!("{}.0.0", major + 1),
        "minor" => format!("{}.{}.0", major, minor + 1),
        _ => format!("{}.{}.{}", major, minor, patch + 1),
    }
}

fn release_semver_update_package_version(root: &std::path::Path, version: &str) -> bool {
    let path = release_semver_package_json_path(root);
    let Some(mut payload) = read_json_file(&path) else {
        return false;
    };
    let Some(object) = payload.as_object_mut() else {
        return false;
    };
    if object.get("version").and_then(Value::as_str) == Some(version) {
        return false;
    }
    object.insert("version".to_string(), Value::String(version.to_string()));
    write_json_file(&path, &payload);
    true
}

fn release_semver_update_package_lock_version(root: &std::path::Path, version: &str) -> bool {
    let path = release_semver_package_lock_path(root);
    let Some(mut payload) = read_json_file(&path) else {
        return false;
    };
    let Some(object) = payload.as_object_mut() else {
        return false;
    };
    let mut changed = false;

    if object.get("version").and_then(Value::as_str) != Some(version) {
        object.insert("version".to_string(), Value::String(version.to_string()));
        changed = true;
    }

    if let Some(packages) = object.get_mut("packages").and_then(Value::as_object_mut) {
        if let Some(root_pkg) = packages.get_mut("").and_then(Value::as_object_mut) {
            if root_pkg.get("version").and_then(Value::as_str) != Some(version) {
                root_pkg.insert("version".to_string(), Value::String(version.to_string()));
                changed = true;
            }
        }
    }

    if changed {
        write_json_file(&path, &payload);
    }
    changed
}

fn release_semver_write_runtime_version_data(
    root: &std::path::Path,
    version: &str,
    bump_kind: &str,
    previous_tag: &str,
    next_tag: &str,
    release_ready: bool,
) -> bool {
    let path = release_semver_runtime_version_path(root);
    let release_channel = if next_tag.trim().is_empty() || next_tag == "none" {
        "stable".to_string()
    } else if next_tag.to_ascii_lowercase().contains("-alpha") {
        "alpha".to_string()
    } else if next_tag.to_ascii_lowercase().contains("-beta") {
        "beta".to_string()
    } else {
        "stable".to_string()
    };
    let payload = json!({
        "schema_version": 1,
        "version": normalize_version_text(version),
        "tag": if next_tag.trim().is_empty() || next_tag == "none" {
            format!("v{}", normalize_version_text(version))
        } else {
            clean(next_tag, 120)
        },
        "previous_tag": if previous_tag.trim().is_empty() {
            Value::Null
        } else {
            Value::String(clean(previous_tag, 120))
        },
        "release_channel": release_channel,
        "bump": clean(bump_kind, 16),
        "release_ready": release_ready,
        "source": "release_semver_contract",
    });
    let changed = read_json_file(&path).map(|prior| prior != payload).unwrap_or(true);
    write_json_file(&path, &payload);
    changed
}

fn run_release_semver_contract_domain(root: &std::path::Path, args: &[String]) -> i32 {
    let opts = parse_release_semver_contract_args(args);
    if matches!(opts.command.as_str(), "help" | "--help" | "-h") {
        println!("Usage: infring release-semver-contract [run|status] [--write=0|1] [--strict=0|1] [--pretty=0|1] [--channel=alpha|beta|stable]");
        return 0;
    }

    let requested_channel = release_semver_normalize_channel(
        &if !opts.channel.trim().is_empty() {
            opts.channel.clone()
        } else if let Ok(value) = env::var("INFRING_RELEASE_CHANNEL") {
            value
        } else if let Ok(value) = env::var("INFRING_RELEASE_CHANNEL") {
            value
        } else {
            release_semver_channel_policy_default(root)
        },
    );

    let previous_tag = release_semver_latest_tag(root);
    let range = if previous_tag.is_empty() {
        "HEAD".to_string()
    } else {
        format!("{previous_tag}..HEAD")
    };
    let commits = release_semver_read_commits(root, &range)
        .into_iter()
        .filter(|row| !release_semver_is_release_chore(&row.subject))
        .collect::<Vec<_>>();
    let bump = release_semver_classify_bump(&commits);
    let release_ready = bump != "none" && !commits.is_empty();
    let base = release_semver_base_version(root, &previous_tag);
    let current_version = base.3.clone();
    let next_version = if release_ready {
        release_semver_bump_version(&base, &bump)
    } else {
        current_version.clone()
    };
    let next_tag = if release_ready {
        release_semver_tag_for_channel(&next_version, &requested_channel)
    } else {
        "none".to_string()
    };

    let mut wrote_version = false;
    if opts.write && release_ready {
        wrote_version = release_semver_update_package_version(root, &next_version) || wrote_version;
        wrote_version =
            release_semver_update_package_lock_version(root, &next_version) || wrote_version;
        wrote_version = release_semver_write_runtime_version_data(
            root,
            &next_version,
            &bump,
            &previous_tag,
            &next_tag,
            release_ready,
        ) || wrote_version;
    } else if opts.write {
        let stable_version = if release_ready {
            next_version.clone()
        } else {
            current_version.clone()
        };
        let _ = release_semver_write_runtime_version_data(
            root,
            &stable_version,
            &bump,
            &previous_tag,
            &next_tag,
            release_ready,
        );
    }

    let commits_payload = commits
        .iter()
        .take(60)
        .map(|row| {
            let classification = if release_semver_is_breaking_change(&row.subject, &row.body) {
                "major".to_string()
            } else {
                let (ty, _breaking_bang) = release_semver_conventional_type(&row.subject);
                if ty == "feat" {
                    "minor".to_string()
                } else {
                    "patch".to_string()
                }
            };
            json!({
                "sha": row.sha,
                "subject": row.subject,
                "classification": classification
            })
        })
        .collect::<Vec<_>>();

    let output = json!({
        "ok": true,
        "mode": "conventional_commits",
        "release_channel": requested_channel,
        "release_ready": release_ready,
        "previous_tag": if previous_tag.is_empty() { "none".to_string() } else { previous_tag.clone() },
        "next_tag": next_tag,
        "current_version": current_version,
        "next_version": next_version,
        "bump": bump,
        "commits_scanned": commits.len(),
        "commits": commits_payload,
        "write_requested": opts.write,
        "version_bumped": wrote_version
    });

    if opts.pretty {
        if let Ok(raw) = serde_json::to_string_pretty(&output) {
            println!("{raw}");
        } else {
            println!("{{\"ok\":false,\"error\":\"serialize_failed\"}}");
        }
    } else {
        emit_json_line(&output);
    }

    if opts.strict && output.get("ok").and_then(Value::as_bool) != Some(true) {
        1
    } else {
        0
    }
}

