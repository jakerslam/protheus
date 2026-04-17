pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let cmd = clean(
        parsed
            .positional
            .first()
            .cloned()
            .unwrap_or_else(|| "status".to_string()),
        80,
    )
    .to_ascii_lowercase();

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let policy_arg = parsed
        .flags
        .get("policy")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var("SCALE_READINESS_PROGRAM_POLICY_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    root.join("client/runtime/config/scale_readiness_program_policy.json")
                })
        });
    let policy_path = if policy_arg.is_absolute() {
        policy_arg
    } else {
        root.join(policy_arg)
    };

    let policy = load_policy(root, &policy_path);
    if !policy.enabled {
        println!(
            "{}",
            json!({"ok": false, "error": "scale_readiness_program_disabled"})
        );
        return 1;
    }

    match cmd.as_str() {
        "list" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&list(&policy, root))
                    .unwrap_or_else(|_| "{}".to_string())
            );
            0
        }
        "status" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&status(&policy, root))
                    .unwrap_or_else(|_| "{}".to_string())
            );
            0
        }
        "run" => {
            let id = normalize_id(parsed.flags.get("id").map(String::as_str).unwrap_or(""));
            if id.is_empty() {
                println!(
                    "{}",
                    json!({"ok": false, "type": "scale_readiness_program", "action": "run", "error": "id_required"})
                );
                return 1;
            }
            let strict = to_bool(
                parsed.flags.get("strict").map(String::as_str),
                policy.strict_default,
            );
            let apply = to_bool(parsed.flags.get("apply").map(String::as_str), true);
            match run_one(&policy, &id, apply, strict, root) {
                Ok(out) => {
                    let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
                    );
                    if ok {
                        0
                    } else {
                        1
                    }
                }
                Err(err) => {
                    println!("{}", json!({"ok": false, "error": err}));
                    1
                }
            }
        }
        "run-all" => {
            let strict = to_bool(
                parsed.flags.get("strict").map(String::as_str),
                policy.strict_default,
            );
            let apply = to_bool(parsed.flags.get("apply").map(String::as_str), true);
            match run_all(&policy, apply, strict, root) {
                Ok(out) => {
                    let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
                    );
                    if ok {
                        0
                    } else {
                        1
                    }
                }
                Err(err) => {
                    println!("{}", json!({"ok": false, "error": err}));
                    1
                }
            }
        }
        _ => {
            usage();
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn list_contains_all_scale_ids() {
        let dir = tempdir().expect("tempdir");
        let policy = default_policy(dir.path());
        let out = list(&policy, dir.path());
        assert_eq!(out["item_count"].as_u64(), Some(10));
    }

    #[test]
    fn disabled_policy_fail_closed() {
        let dir = tempdir().expect("tempdir");
        let policy_path = dir.path().join("scale_policy.json");
        fs::write(
            &policy_path,
            serde_json::to_string_pretty(&json!({"enabled": false})).expect("encode"),
        )
        .expect("write");
        let exit = run(
            dir.path(),
            &[
                "status".to_string(),
                format!("--policy={}", policy_path.to_string_lossy()),
            ],
        );
        assert_eq!(exit, 1);
    }
}
