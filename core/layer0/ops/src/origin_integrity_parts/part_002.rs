fn check_seed_certificate(
    root: &Path,
    policy: &OriginIntegrityPolicy,
    certificate_path: &Path,
) -> Value {
    let remote = read_json(certificate_path).unwrap_or_else(|_| json!({}));
    let local = evaluate_invariants(root, policy, "seed-bootstrap-verify-local");

    let remote_verify_sha = remote
        .get("verify_script_sha256")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let local_verify_sha = {
        let verify_path = resolve_path(root, &policy.verify_script_relpath);
        if verify_path.is_file() {
            sha256_file(&verify_path).unwrap_or_default()
        } else {
            String::new()
        }
    };

    let remote_safety_hash = remote
        .get("safety_plane_state_hash")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let local_safety_hash = local
        .get("state_binding")
        .and_then(|v| v.get("safety_plane_state_hash"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let certificate_type_ok = remote
        .get("type")
        .and_then(Value::as_str)
        .map(|v| v == "origin_verify_certificate")
        .unwrap_or(false);

    let mut out = json!({
        "ok": certificate_type_ok
            && !remote_verify_sha.is_empty()
            && !local_verify_sha.is_empty()
            && remote_verify_sha == local_verify_sha
            && !remote_safety_hash.is_empty()
            && !local_safety_hash.is_empty()
            && remote_safety_hash == local_safety_hash
            && local.get("ok").and_then(Value::as_bool) == Some(true),
        "type": "seed_bootstrap_verify",
        "ts": now_iso(),
        "certificate_path": certificate_path.display().to_string(),
        "certificate_type_ok": certificate_type_ok,
        "verify_script_sha256": {
            "remote": remote_verify_sha,
            "local": local_verify_sha,
            "match": remote_verify_sha == local_verify_sha
        },
        "safety_plane_state_hash": {
            "remote": remote_safety_hash,
            "local": local_safety_hash,
            "match": remote_safety_hash == local_safety_hash
        },
        "local_origin_integrity_ok": local.get("ok").and_then(Value::as_bool) == Some(true)
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let policy_path = parsed.flags.get("policy").map(String::as_str);
    let (policy, resolved_policy_path) = match load_policy(root, policy_path) {
        Ok(v) => v,
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "origin_integrity_enforcer",
                "command": command,
                "error": err,
                "ts": now_iso()
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            println!(
                "{}",
                serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
            );
            return 1;
        }
    };

    let strict = parse_bool(
        parsed.flags.get("strict").map(String::as_str),
        policy.strict_default,
    );
    let latest_path = resolve_path(root, &policy.paths.latest_path);
    let receipts_path = resolve_path(root, &policy.paths.receipts_path);
    let certificate_path = resolve_path(root, &policy.paths.certificate_path);

    match command.as_str() {
        "run" => {
            let mut out = evaluate_invariants(root, &policy, "run");
            out["policy_path"] = Value::String(resolved_policy_path.display().to_string());
            let _ = write_json_atomic(&latest_path, &out);
            let _ = append_jsonl(&receipts_path, &out);
            println!(
                "{}",
                serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
            );
            if out.get("ok").and_then(Value::as_bool) == Some(true) {
                0
            } else if strict {
                1
            } else {
                0
            }
        }
        "status" => {
            let latest = read_json(&latest_path).ok();
            let mut out = json!({
                "ok": latest
                    .as_ref()
                    .and_then(|v| v.get("ok"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                "type": "origin_integrity_status",
                "ts": now_iso(),
                "policy_path": resolved_policy_path.display().to_string(),
                "latest_path": latest_path.display().to_string(),
                "latest": latest
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            println!(
                "{}",
                serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
            );
            0
        }
        "certificate" => {
            let mut run_receipt = evaluate_invariants(root, &policy, "certificate");
            run_receipt["policy_path"] = Value::String(resolved_policy_path.display().to_string());
            let _ = write_json_atomic(&latest_path, &run_receipt);
            let _ = append_jsonl(&receipts_path, &run_receipt);

            if run_receipt.get("ok").and_then(Value::as_bool) != Some(true) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&run_receipt).unwrap_or_else(|_| "{}".to_string())
                );
                return if strict { 1 } else { 0 };
            }

            let cert = build_certificate(root, &policy, &run_receipt);
            let _ = write_json_atomic(&certificate_path, &cert);
            let _ = append_jsonl(&receipts_path, &cert);
            println!(
                "{}",
                serde_json::to_string_pretty(&cert).unwrap_or_else(|_| "{}".to_string())
            );
            if cert.get("ok").and_then(Value::as_bool) == Some(true) {
                0
            } else if strict {
                1
            } else {
                0
            }
        }
        "seed-bootstrap-verify" => {
            let certificate = parsed.flags.get("certificate").map(String::as_str);
            let Some(certificate_raw) = certificate else {
                let mut out = json!({
                    "ok": false,
                    "type": "seed_bootstrap_verify",
                    "ts": now_iso(),
                    "error": "certificate_required"
                });
                out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
                println!(
                    "{}",
                    serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
                );
                return 1;
            };
            let cert_path = resolve_path(root, certificate_raw);
            let out = check_seed_certificate(root, &policy, &cert_path);
            let _ = write_json_atomic(&latest_path, &out);
            let _ = append_jsonl(&receipts_path, &out);
            println!(
                "{}",
                serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
            );
            if out.get("ok").and_then(Value::as_bool) == Some(true) {
                0
            } else if strict {
                1
            } else {
                0
            }
        }
        _ => {
            let mut out = json!({
                "ok": false,
                "type": "origin_integrity_enforcer",
                "ts": now_iso(),
                "error": format!("unknown_command:{}", command)
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            println!(
                "{}",
                serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
            );
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_json_fixture(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(
            path,
            serde_json::to_string_pretty(value).expect("encode fixture"),
        )
        .expect("write fixture");
    }

    #[test]
    fn safety_plane_hash_is_stable_for_same_input() {
        let mut hasher = Sha256::new();
        hasher.update(b"a|b|c");
        let first = hex::encode(hasher.finalize());
        let second = sha256_bytes(b"a|b|c");
        assert_eq!(first, second);
    }

    #[test]
    fn parse_last_json_uses_last_json_line() {
        let payload = parse_last_json("line one\n{\"ok\":false}\n{\"ok\":true,\"x\":1}\n")
            .expect("json payload");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(payload.get("x").and_then(Value::as_i64), Some(1));
    }

    #[test]
    fn native_dependency_boundary_check_detects_allowed_relative_imports() {
        let root = tempdir().expect("tempdir");
        let policy_path = root
            .path()
            .join("client/runtime/config/dependency_boundary_manifest.json");
        write_json_fixture(
            &policy_path,
            &json!({
                "layers": {
                    "core": ["client/runtime/lib"],
                    "systems": ["client/runtime/systems"]
                },
                "allow_imports": {
                    "systems": ["core", "systems"]
                },
                "enforce_layers": ["systems"],
                "scan": {
                    "include_dirs": ["client/runtime/lib", "client/runtime/systems"],
                    "include_ext": [".ts"],
                    "exclude_contains": []
                },
                "conduit_boundary": {
                    "include_dirs": ["client/runtime/systems"],
                    "include_ext": [".ts"],
                    "exclude_contains": [],
                    "allowlisted_files": [],
                    "forbidden_patterns": ["spawnSync('cargo'"]
                }
            }),
        );
        let helper = root.path().join("client/runtime/lib/helper.ts");
        let system = root.path().join("client/runtime/systems/agent.ts");
        fs::create_dir_all(helper.parent().expect("helper parent")).expect("helper parent");
        fs::create_dir_all(system.parent().expect("system parent")).expect("system parent");
        fs::write(&helper, "export const helper = 1;\n").expect("write helper");
        fs::write(
            &system,
            "import { helper } from '../lib/helper';\nexport { helper };\n",
        )
        .expect("write system");

        let payload =
            run_dependency_boundary_check_native(root.path(), &policy_path).expect("native check");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("scanned_files").and_then(Value::as_u64),
            Some(2)
        );
    }

    #[test]
    fn run_dependency_boundary_check_falls_back_to_native_when_script_is_missing() {
        let root = tempdir().expect("tempdir");
        let manifest_path = root
            .path()
            .join("client/runtime/config/dependency_boundary_manifest.json");
        write_json_fixture(
            &manifest_path,
            &json!({
                "layers": {
                    "core": ["client/runtime/lib"],
                    "systems": ["client/runtime/systems"]
                },
                "allow_imports": {
                    "systems": ["core", "systems"]
                },
                "enforce_layers": ["systems"],
                "scan": {
                    "include_dirs": ["client/runtime/lib", "client/runtime/systems"],
                    "include_ext": [".ts"],
                    "exclude_contains": []
                },
                "conduit_boundary": {
                    "include_dirs": ["client/runtime/systems"],
                    "include_ext": [".ts"],
                    "exclude_contains": [],
                    "allowlisted_files": [],
                    "forbidden_patterns": []
                }
            }),
        );
        let helper = root.path().join("client/runtime/lib/helper.ts");
        let system = root.path().join("client/runtime/systems/agent.ts");
        fs::create_dir_all(helper.parent().expect("helper parent")).expect("helper parent");
        fs::create_dir_all(system.parent().expect("system parent")).expect("system parent");
        fs::write(&helper, "export const helper = 1;\n").expect("write helper");
        fs::write(
            &system,
            "import { helper } from '../lib/helper';\nexport { helper };\n",
        )
        .expect("write system");

        let policy = OriginIntegrityPolicy {
            dependency_boundary_policy_path: manifest_path.display().to_string(),
            ..OriginIntegrityPolicy::default()
        };
        let payload = run_dependency_boundary_check(root.path(), &policy);
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("engine").and_then(Value::as_str),
            Some("native")
        );
        assert_eq!(
            payload
                .get("payload")
                .and_then(|v| v.get("scanned_files"))
                .and_then(Value::as_u64),
            Some(2)
        );
    }
}

