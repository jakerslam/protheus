pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "--help" | "-h" | "help") {
        usage();
        return 0;
    }

    let policy = load_policy(root, parsed.flags.get("policy"));
    let strict = parsed
        .flags
        .get("strict")
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(policy.strict_default);

    let payload = match cmd.as_str() {
        "authorize" => run_authorize(&policy, &parsed.flags),
        "scim-lifecycle" => run_scim(&policy, &parsed.flags),
        "status" => {
            let out = status(&policy);
            println!(
                "{}",
                serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string())
            );
            return 0;
        }
        _ => {
            usage();
            let out = cli_error(argv, "unknown_command", 2);
            println!(
                "{}",
                serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string())
            );
            return 2;
        }
    };

    let mut out = payload;
    out["policy_path"] = Value::String(policy.policy_path.to_string_lossy().to_string());
    out["strict"] = Value::Bool(strict);
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));

    if let Err(err) = persist(&policy, &out) {
        let fail = cli_error(argv, &format!("persist_failed:{err}"), 1);
        println!(
            "{}",
            serde_json::to_string(&fail).unwrap_or_else(|_| "{}".to_string())
        );
        return 1;
    }

    println!(
        "{}",
        serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string())
    );
    if strict && !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_text(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(path, text).expect("write");
    }

    fn write_policy(root: &Path) {
        write_text(
            &root.join("client/runtime/config/identity_federation_policy.json"),
            &json!({
                "strict_default": true,
                "providers": {
                    "okta": {
                        "issuer_prefix": "https://okta.example.com",
                        "allowed_scopes": ["openid", "profile", "protheus.read"],
                        "allowed_roles": ["operator", "security"],
                        "scim_enabled": true
                    }
                },
                "outputs": {
                    "latest_path": "local/state/ops/identity_federation/latest.json",
                    "history_path": "local/state/ops/identity_federation/history.jsonl"
                }
            })
            .to_string(),
        );
    }

    #[test]
    fn authorize_fails_closed_on_unknown_scope() {
        let tmp = tempdir().expect("tmp");
        write_policy(tmp.path());

        let code = run(
            tmp.path(),
            &[
                "authorize".to_string(),
                "--provider=okta".to_string(),
                "--subject=user-1".to_string(),
                "--token-issuer=https://okta.example.com/oauth2/default".to_string(),
                "--scopes=openid,unknown.scope".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(code, 1);
    }

    #[test]
    fn scim_delete_requires_empty_entitlements() {
        let tmp = tempdir().expect("tmp");
        write_policy(tmp.path());

        let code = run(
            tmp.path(),
            &[
                "scim-lifecycle".to_string(),
                "--provider=okta".to_string(),
                "--operation=delete".to_string(),
                "--user-id=user-1".to_string(),
                "--entitlements=team.alpha".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(code, 1);
    }
}
