fn run_all(
    policy: &Policy,
    args: &HashMap<String, String>,
    apply: bool,
    strict: bool,
    root: &Path,
) -> Result<Value, String> {
    let mut lanes = Vec::new();
    for id in IDS {
        lanes.push(run_one(policy, id, args, apply, strict, root)?);
    }

    let ok = lanes
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    let failed = lanes
        .iter()
        .filter_map(|row| {
            if row.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                row.get("lane_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            }
        })
        .collect::<Vec<_>>();

    let out = json!({
        "ok": ok,
        "type": "fluxlattice_program",
        "action": "run-all",
        "ts": now_iso(),
        "strict": strict,
        "apply": apply,
        "lane_count": lanes.len(),
        "lanes": lanes,
        "failed_lane_ids": failed
    });

    if apply {
        let row = json!({
            "schema_id": "fluxlattice_program_receipt",
            "schema_version": "1.0",
            "artifact_type": "receipt",
            "receipt_id": format!("flux_{}", stable_hash(&serde_json::to_string(&json!({"action":"run-all","ts":now_iso()})).unwrap_or_else(|_| "{}".to_string()), 16)),
            "ok": out["ok"],
            "type": out["type"],
            "action": out["action"],
            "ts": out["ts"],
            "strict": out["strict"],
            "apply": out["apply"],
            "lane_count": out["lane_count"],
            "lanes": out["lanes"],
            "failed_lane_ids": out["failed_lane_ids"]
        });
        write_receipt(policy, &row, true)?;
    }

    Ok(out)
}

pub fn usage() {
    println!("Usage:");
    println!("  node client/runtime/systems/ops/fluxlattice_program.js list");
    println!(
        "  node client/runtime/systems/ops/fluxlattice_program.js run --id=V4-ETH-001 [--apply=1|0] [--strict=1|0]"
    );
    println!("  node client/runtime/systems/ops/fluxlattice_program.js run-all [--apply=1|0] [--strict=1|0]");
    println!("  node client/runtime/systems/ops/fluxlattice_program.js status");
}

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
        .unwrap_or_else(|| root.join("client/runtime/config/fluxlattice_program_policy.json"));
    let policy_path = if policy_arg.is_absolute() {
        policy_arg
    } else {
        root.join(policy_arg)
    };

    let policy = load_policy(root, &policy_path);
    if !policy.enabled {
        println!(
            "{}",
            json!({"ok": false, "error": "fluxlattice_program_disabled"})
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
                    json!({"ok": false, "type": "fluxlattice_program", "action": "run", "error": "id_required"})
                );
                return 1;
            }
            let strict = to_bool(
                parsed.flags.get("strict").map(String::as_str),
                policy.strict_default,
            );
            let apply = to_bool(parsed.flags.get("apply").map(String::as_str), true);
            match run_one(&policy, &id, &parsed.flags, apply, strict, root) {
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
            match run_all(&policy, &parsed.flags, apply, strict, root) {
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
    fn list_has_expected_item_count() {
        let dir = tempdir().expect("tempdir");
        let policy = default_policy(dir.path());
        let out = list(&policy, dir.path());
        assert_eq!(out["item_count"].as_u64(), Some(16));
    }

    #[test]
    fn disabled_policy_fails_closed() {
        let dir = tempdir().expect("tempdir");
        let policy_path = dir.path().join("flux_policy.json");
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

    #[test]
    fn run_requires_id() {
        let dir = tempdir().expect("tempdir");
        let exit = run(dir.path(), &["run".to_string(), "--apply=0".to_string()]);
        assert_eq!(exit, 1);
    }

    #[test]
    fn sec_014_generates_chain_hash() {
        let dir = tempdir().expect("tempdir");
        let policy = default_policy(dir.path());
        let args = HashMap::from([(String::from("deny"), String::from("1"))]);
        let receipt = run_one(&policy, "V4-SEC-014", &args, false, true, dir.path()).expect("run");
        assert_eq!(
            receipt["checks"]["receipt_chain_hash_len_64"].as_bool(),
            Some(true)
        );
    }
}

