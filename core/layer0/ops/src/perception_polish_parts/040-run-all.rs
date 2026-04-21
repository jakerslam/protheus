
fn run_all(
    policy: &Policy,
    args: &std::collections::HashMap<String, String>,
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
        "type": "perception_polish_program",
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
            "schema_id": "perception_polish_program_receipt",
            "schema_version": "1.0",
            "artifact_type": "receipt",
            "receipt_id": format!("perception_{}", stable_hash(&serde_json::to_string(&json!({"action":"run-all","ts":now_iso()})).unwrap_or_else(|_| "{}".to_string()), 16)),
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
    println!("  node client/runtime/systems/ops/perception_polish_program.js list");
    println!("  node client/runtime/systems/ops/perception_polish_program.js run --id=V4-ILLUSION-001 [--apply=1|0] [--strict=1|0]");
    println!(
        "  node client/runtime/systems/ops/perception_polish_program.js run-all [--apply=1|0] [--strict=1|0]"
    );
    println!("  node client/runtime/systems/ops/perception_polish_program.js status");
}

fn print_json_value(payload: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(payload).unwrap_or_else(|_| "{}".to_string())
    );
}

fn print_and_exit(payload: Value) -> i32 {
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
    print_json_value(&payload);
    if ok {
        0
    } else {
        1
    }
}

fn print_result(result: Result<Value, String>) -> i32 {
    match result {
        Ok(payload) => print_and_exit(payload),
        Err(err) => print_and_exit(json!({"ok": false, "error": err})),
    }
}

fn resolve_policy_path(root: &Path, parsed: &crate::ParsedArgs) -> PathBuf {
    let policy_arg = parsed
        .flags
        .get("policy")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            root.join("client/runtime/config/perception_polish_program_policy.json")
        });
    if policy_arg.is_absolute() {
        policy_arg
    } else {
        root.join(policy_arg)
    }
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

    let policy_path = resolve_policy_path(root, &parsed);
    let policy = load_policy(root, &policy_path);
    if !policy.enabled {
        return print_and_exit(json!({"ok": false, "error": "perception_polish_program_disabled"}));
    }

    match cmd.as_str() {
        "list" => print_and_exit(list(&policy, root)),
        "status" => print_and_exit(status(&policy, root)),
        "run" => {
            let id = normalize_id(parsed.flags.get("id").map(String::as_str).unwrap_or(""));
            if id.is_empty() {
                return print_and_exit(
                    json!({"ok": false, "type": "perception_polish_program", "action": "run", "error": "id_required"}),
                );
            }
            let strict = to_bool(
                parsed.flags.get("strict").map(String::as_str),
                policy.strict_default,
            );
            let apply = to_bool(parsed.flags.get("apply").map(String::as_str), true);
            print_result(run_one(&policy, &id, &parsed.flags, apply, strict, root))
        }
        "run-all" => {
            let strict = to_bool(
                parsed.flags.get("strict").map(String::as_str),
                policy.strict_default,
            );
            let apply = to_bool(parsed.flags.get("apply").map(String::as_str), true);
            print_result(run_all(&policy, &parsed.flags, apply, strict, root))
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
    fn list_has_four_items() {
        let dir = tempdir().expect("tempdir");
        let policy = default_policy(dir.path());
        let out = list(&policy, dir.path());
        assert_eq!(out["item_count"].as_u64(), Some(4));
    }

    #[test]
    fn disabled_policy_fails_closed() {
        let dir = tempdir().expect("tempdir");
        let p = dir.path().join("perception_policy.json");
        fs::write(
            &p,
            serde_json::to_string_pretty(&json!({"enabled": false})).expect("encode"),
        )
        .expect("write");
        let exit = run(
            dir.path(),
            &[
                "status".to_string(),
                format!("--policy={}", p.to_string_lossy()),
            ],
        );
        assert_eq!(exit, 1);
    }
}
