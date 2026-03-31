pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());

    match cmd.as_str() {
        "help" | "--help" | "-h" => {
            usage();
            0
        }
        "default-policy" => match payload_json(argv) {
            Ok(payload) => {
                let root_dir =
                    root_dir_from_payload(root, payload.as_object().unwrap_or(&Map::new()));
                print_json_line(&cli_receipt(
                    "training_conduit_schema_kernel_default_policy",
                    json!({ "policy": default_policy(&root_dir) }),
                ));
                0
            }
            Err(err) => {
                print_json_line(&cli_error(
                    "training_conduit_schema_kernel_default_policy",
                    &err,
                ));
                1
            }
        },
        "normalize-policy" => match payload_json(argv) {
            Ok(payload) => {
                let obj = payload.as_object().cloned().unwrap_or_default();
                let root_dir = root_dir_from_payload(root, &obj);
                print_json_line(&cli_receipt(
                    "training_conduit_schema_kernel_normalize_policy",
                    json!({ "policy": normalize_policy(obj.get("policy"), &root_dir) }),
                ));
                0
            }
            Err(err) => {
                print_json_line(&cli_error(
                    "training_conduit_schema_kernel_normalize_policy",
                    &err,
                ));
                1
            }
        },
        "load-policy" => match payload_json(argv) {
            Ok(payload) => {
                let obj = payload.as_object().cloned().unwrap_or_default();
                let root_dir = root_dir_from_payload(root, &obj);
                print_json_line(&cli_receipt(
                    "training_conduit_schema_kernel_load_policy",
                    json!({ "policy": load_policy(&root_dir, &obj) }),
                ));
                0
            }
            Err(err) => {
                print_json_line(&cli_error(
                    "training_conduit_schema_kernel_load_policy",
                    &err,
                ));
                1
            }
        },
        "build-metadata" => match payload_json(argv) {
            Ok(payload) => {
                let obj = payload.as_object().cloned().unwrap_or_default();
                let root_dir = root_dir_from_payload(root, &obj);
                print_json_line(&cli_receipt(
                    "training_conduit_schema_kernel_build_metadata",
                    json!({
                        "metadata": build_training_conduit_metadata(obj.get("input"), obj.get("policy"), &root_dir)
                    }),
                ));
                0
            }
            Err(err) => {
                print_json_line(&cli_error(
                    "training_conduit_schema_kernel_build_metadata",
                    &err,
                ));
                1
            }
        },
        "validate-metadata" => match payload_json(argv) {
            Ok(payload) => {
                let obj = payload.as_object().cloned().unwrap_or_default();
                let root_dir = root_dir_from_payload(root, &obj);
                let empty = json!({});
                let metadata = obj.get("metadata").unwrap_or(&empty);
                print_json_line(&cli_receipt(
                    "training_conduit_schema_kernel_validate_metadata",
                    json!({
                        "validation": validate_training_conduit_metadata(
                            metadata,
                            obj.get("policy"),
                            &root_dir
                        )
                    }),
                ));
                0
            }
            Err(err) => {
                print_json_line(&cli_error(
                    "training_conduit_schema_kernel_validate_metadata",
                    &err,
                ));
                1
            }
        },
        _ => {
            usage();
            print_json_line(&cli_error(
                "training_conduit_schema_kernel",
                "unknown_command",
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_policy_clamps_and_normalizes_defaults() {
        let root = PathBuf::from("/tmp/repo/client");
        let out = normalize_policy(
            Some(&json!({
                "defaults": {
                    "owner_id": " Team Lead ",
                    "retention_days": 99999,
                    "consent_status": "Granted"
                },
                "constraints": {
                    "min_retention_days": 5,
                    "max_retention_days": 90
                }
            })),
            &root,
        );
        assert_eq!(
            out.pointer("/defaults/owner_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "team_lead"
        );
        assert_eq!(
            out.pointer("/defaults/retention_days")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            3650
        );
        assert_eq!(
            out.pointer("/defaults/consent_status")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "granted"
        );
    }

    #[test]
    fn build_metadata_embeds_validation() {
        let root = PathBuf::from("/tmp/repo/client");
        let out = build_training_conduit_metadata(
            Some(&json!({
                "source_system": "discord",
                "source_channel": "ops",
                "datum_id": "abc-123"
            })),
            Some(&default_policy(&root)),
            &root,
        );
        assert_eq!(
            out.pointer("/source/system")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "discord"
        );
        assert_eq!(
            out.pointer("/validation/ok")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            true
        );
    }
}

