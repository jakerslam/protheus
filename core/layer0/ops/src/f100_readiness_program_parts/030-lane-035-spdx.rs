fn lane_035_spdx(root: &Path, policy: &Policy, apply: bool) -> Value {
    let lane_policy = get_lane_policy(policy, "V6-F100-035")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let roots = lane_policy
        .get("roots")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|v| resolve_path(root, Some(v), v))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![root.join("crates")]);

    let exts = lane_policy
        .get("extensions")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["rs".to_string()]);

    let excludes = lane_policy
        .get("exclude_paths")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let baseline_path = resolve_path(
        root,
        lane_policy
            .get("baseline_missing_path")
            .and_then(Value::as_str),
        "client/runtime/config/spdx_header_guard_baseline.txt",
    );

    let mut scanned = 0usize;
    let mut missing = Vec::<String>::new();

    for scan_root in roots {
        if !scan_root.exists() {
            continue;
        }
        for entry in WalkDir::new(&scan_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let p = entry.path();
            let rel = p
                .strip_prefix(root)
                .unwrap_or(p)
                .to_string_lossy()
                .replace('\\', "/");
            if excludes.iter().any(|e| rel.starts_with(e)) {
                continue;
            }
            let ext = p
                .extension()
                .map(|v| v.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if !exts.iter().any(|v| v == &ext) {
                continue;
            }

            scanned += 1;
            let body = fs::read_to_string(p).unwrap_or_default();
            let has_spdx = body
                .lines()
                .take(5)
                .any(|line| line.contains("SPDX-License-Identifier: Apache-2.0"));
            if !has_spdx {
                if apply {
                    let mut new_body = String::new();
                    let comment = "// SPDX-License-Identifier: Apache-2.0\n";
                    if body.starts_with("#!") {
                        if let Some((first, rest)) = body.split_once('\n') {
                            new_body.push_str(first);
                            new_body.push('\n');
                            new_body.push_str(comment);
                            new_body.push_str(rest);
                        } else {
                            new_body.push_str(&body);
                            new_body.push('\n');
                            new_body.push_str(comment);
                        }
                    } else {
                        new_body.push_str(comment);
                        new_body.push_str(&body);
                    }
                    let _ = fs::write(p, new_body);
                } else {
                    missing.push(rel);
                }
            }
        }
    }

    if apply {
        missing.clear();
        for scan_root in lane_policy
            .get("roots")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|v| resolve_path(root, Some(v), v))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![root.join("crates")])
        {
            if !scan_root.exists() {
                continue;
            }
            for entry in WalkDir::new(&scan_root)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
            {
                let p = entry.path();
                let rel = p
                    .strip_prefix(root)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .replace('\\', "/");
                if excludes.iter().any(|e| rel.starts_with(e)) {
                    continue;
                }
                let ext = p
                    .extension()
                    .map(|v| v.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                if !exts.iter().any(|v| v == &ext) {
                    continue;
                }
                let body = fs::read_to_string(p).unwrap_or_default();
                let has_spdx = body
                    .lines()
                    .take(5)
                    .any(|line| line.contains("SPDX-License-Identifier: Apache-2.0"));
                if !has_spdx {
                    missing.push(rel);
                }
            }
        }
        let _ = write_text_atomic(&baseline_path, "");
    }

    let baseline = fs::read_to_string(&baseline_path)
        .unwrap_or_default()
        .lines()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect::<BTreeSet<_>>();
    let missing_set = missing.iter().cloned().collect::<BTreeSet<_>>();
    let unexpected_missing = missing_set
        .difference(&baseline)
        .cloned()
        .collect::<Vec<_>>();

    let ok = unexpected_missing.is_empty();

    json!({
        "ok": ok,
        "lane": "V6-F100-035",
        "type": "f100_spdx_header_guard",
        "apply": apply,
        "scanned_files": scanned,
        "missing_count": missing.len(),
        "unexpected_missing": unexpected_missing,
        "baseline_missing_path": baseline_path
    })
}

fn lane_036_root_rationalization(root: &Path, policy: &Policy, apply: bool) -> Value {
    let lane_policy = get_lane_policy(policy, "V6-F100-036")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let archive_root = resolve_path(
        root,
        lane_policy.get("archive_root").and_then(Value::as_str),
        "research/archive/root_surface",
    );
    let dirs = lane_policy
        .get("root_dirs")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![
                "drafts".to_string(),
                "notes".to_string(),
                "experiments".to_string(),
            ]
        });

    let mut moved = Vec::new();
    if apply {
        for d in &dirs {
            let from = root.join(d);
            if from.exists() {
                let to = archive_root.join(d);
                if let Some(parent) = to.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let target = if to.exists() {
                    archive_root.join(format!("{}_{}", d, now_iso().replace(':', "-")))
                } else {
                    to
                };
                if fs::rename(&from, &target).is_ok() {
                    moved.push(json!({"from": d, "to": target}));
                }
            }
        }
    }

    let root_absent = dirs.iter().all(|d| !root.join(d).exists());
    let archive_present = dirs.iter().all(|d| archive_root.join(d).exists())
        || dirs.iter().all(|d| {
            fs::read_dir(&archive_root)
                .ok()
                .map(|mut it| {
                    it.any(|e| {
                        e.ok()
                            .map(|x| x.file_name().to_string_lossy().starts_with(d))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        });

    json!({
        "ok": root_absent && archive_present,
        "lane": "V6-F100-036",
        "type": "f100_root_surface_rationalization",
        "apply": apply,
        "archive_root": archive_root,
        "root_dirs": dirs,
        "moved": moved,
        "root_absent": root_absent,
        "archive_present": archive_present
    })
}

fn run_lane(root: &Path, policy: &Policy, lane: &str, apply: bool) -> Value {
    match lane {
        "V6-F100-004" => lane_004_compliance_bundle(root, policy),
        "V6-F100-005" => lane_005_million_user(root, policy),
        "V6-F100-006" => lane_006_multi_tenant(root, policy),
        "V6-F100-007" => lane_007_interface_lifecycle(root, policy),
        "V6-F100-008" => lane_008_oncall(root, policy),
        "V7-F100-005" => lane_005_million_user_with_id(root, policy, "V7-F100-005"),
        "V7-F100-006" => lane_006_multi_tenant_with_id(root, policy, "V7-F100-006"),
        "V7-F100-007" => lane_007_interface_lifecycle_with_id(root, policy, "V7-F100-007"),
        "V7-F100-008" => lane_008_oncall_with_id(root, policy, "V7-F100-008"),
        "V6-F100-009" => lane_009_onboarding(root, policy),
        "V6-F100-010" => lane_010_architecture_pack(root, policy),
        "V6-F100-011" => lane_011_surface_consistency(root, policy),
        "V6-F100-012" => lane_012_scorecard(root, policy),
        "V6-F100-035" => lane_035_spdx(root, policy, apply),
        "V6-F100-036" => lane_036_root_rationalization(root, policy, apply),
        _ => json!({
            "ok": false,
            "lane": lane,
            "type": "f100_readiness_program_unknown_lane",
            "error": "unknown_lane"
        }),
    }
}

fn load_policy(root: &Path, policy_override: Option<&String>) -> Policy {
    let policy_path = policy_override
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(DEFAULT_POLICY_REL));

    let raw = fs::read_to_string(&policy_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| json!({}));

    let outputs = raw.get("outputs").and_then(Value::as_object);
    Policy {
        strict_default: raw
            .get("strict_default")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        state_root: resolve_path(
            root,
            outputs
                .and_then(|o| o.get("state_root"))
                .and_then(Value::as_str),
            "local/state/ops/f100_readiness_program",
        ),
        latest_path: resolve_path(
            root,
            outputs
                .and_then(|o| o.get("latest_path"))
                .and_then(Value::as_str),
            "local/state/ops/f100_readiness_program/latest.json",
        ),
        history_path: resolve_path(
            root,
            outputs
                .and_then(|o| o.get("history_path"))
                .and_then(Value::as_str),
            "local/state/ops/f100_readiness_program/history.jsonl",
        ),
        policy_path,
        raw,
    }
}

fn status(policy: &Policy, lane: &str) -> Value {
    let (lane_latest, lane_history) = lane_state_paths(policy, lane);
    let latest = read_json(&lane_latest)
        .unwrap_or_else(|| json!({ "ok": false, "error": "latest_missing" }));
    let mut out = json!({
        "ok": latest.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "type": "f100_readiness_program_status",
        "lane_program": LANE_ID,
        "lane": lane,
        "ts": now_iso(),
        "policy_path": policy.policy_path,
        "lane_latest_path": lane_latest,
        "lane_history_path": lane_history,
        "latest": latest
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(argv: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "f100_readiness_program_cli_error",
        "lane_program": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": code
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

