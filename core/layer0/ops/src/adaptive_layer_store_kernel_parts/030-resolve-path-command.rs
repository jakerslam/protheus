fn resolve_path_command(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let target = clean_text(payload.get("target_path"), 520);
    let (abs, rel) = resolve_adaptive_path(root, payload, &target)?;
    Ok(json!({
        "ok": true,
        "abs": abs.to_string_lossy(),
        "rel": rel,
    }))
}

fn resolve_provider_artifact_path_command(
    root: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let plugin_id = clean_text(
        payload
            .get("plugin_id")
            .or_else(|| payload.get("provider_plugin_id")),
        96,
    )
    .to_ascii_lowercase()
    .chars()
    .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    .collect::<String>();
    if plugin_id.is_empty() {
        return Err("adaptive_layer_store_kernel_provider_plugin_id_required".to_string());
    }
    let contract = clean_text(payload.get("contract"), 64);
    let contract_dir = match contract.as_str() {
        "webSearchProviders" => "web_search",
        "memoryEmbeddingProviders" => "memory_embedding",
        _ => "web_fetch",
    };
    let target = format!("tooling/public_artifacts/{contract_dir}/{plugin_id}.json");
    let (abs, rel) = resolve_adaptive_path(root, payload, &target)?;
    Ok(json!({
        "ok": true,
        "explicit_fast_path": true,
        "contract": if contract.is_empty() { "webFetchProviders".to_string() } else { contract },
        "plugin_id": plugin_id,
        "abs": abs.to_string_lossy(),
        "rel": rel,
        "exists": abs.exists()
    }))
}

pub(crate) fn read_json_command(
    root: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let fallback = payload.get("fallback").cloned().unwrap_or(Value::Null);
    let target = clean_text(payload.get("target_path"), 520);
    let (abs, rel) = resolve_adaptive_path(root, payload, &target)?;
    let (exists, value, current_hash) = read_json_with_hash(&abs);
    Ok(json!({
        "ok": true,
        "exists": exists,
        "path": abs.to_string_lossy(),
        "rel": rel,
        "value": if exists { value } else { fallback },
        "current_hash": current_hash,
    }))
}

pub(crate) fn ensure_json_command(
    root: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let target = clean_text(payload.get("target_path"), 520);
    let default_value = payload
        .get("default_value")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let meta = payload
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let (abs, rel) = resolve_adaptive_path(root, payload, &target)?;
    let lock = acquire_write_lock(&abs)?;
    let result = if let Some(existing) = read_json_value(&abs) {
        let current_hash = canonical_hash(&existing);
        json!({
            "ok": true,
            "created": false,
            "value": existing,
            "path": abs.to_string_lossy(),
            "rel": rel,
            "current_hash": current_hash,
            "lock_wait_ms": lock.waited_ms,
        })
    } else {
        write_json_atomic(&abs, &default_value)?;
        append_mutation_log(
            root,
            payload,
            &json!({
                "ts": now_iso(),
                "op": "ensure",
                "rel_path": rel,
                "actor": meta_actor(&meta),
                "source": meta_source(&meta),
                "reason": meta_reason(&meta, "ensure_default"),
                "lock_wait_ms": lock.waited_ms,
                "value_hash": canonical_hash(&default_value),
            }),
        );
        let pointer_stats =
            emit_adaptive_pointers(root, payload, &rel, &default_value, "ensure", &meta);
        json!({
            "ok": true,
            "created": true,
            "value": default_value,
            "path": abs.to_string_lossy(),
            "rel": rel,
            "current_hash": canonical_hash(&default_value),
            "lock_wait_ms": lock.waited_ms,
            "pointer_stats": pointer_stats,
        })
    };
    release_write_lock(lock);
    Ok(result)
}

pub(crate) fn set_json_command(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let target = clean_text(payload.get("target_path"), 520);
    let value = payload.get("value").cloned().unwrap_or(Value::Null);
    let meta = payload
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let expected_hash = clean_text(payload.get("expected_hash"), 160);
    let (abs, rel) = resolve_adaptive_path(root, payload, &target)?;
    let lock = acquire_write_lock(&abs)?;
    let (exists, current_value, current_hash) = read_json_with_hash(&abs);

    let expected_missing = expected_hash == MISSING_HASH_SENTINEL;
    if !expected_hash.is_empty() {
        let conflict = if expected_missing {
            exists
        } else {
            current_hash.as_deref() != Some(expected_hash.as_str())
        };
        if conflict {
            let result = json!({
                "ok": true,
                "applied": false,
                "conflict": true,
                "path": abs.to_string_lossy(),
                "rel": rel,
                "current_hash": current_hash,
                "value": if exists { current_value } else { Value::Null },
                "lock_wait_ms": lock.waited_ms,
            });
            release_write_lock(lock);
            return Ok(result);
        }
    }

    write_json_atomic(&abs, &value)?;
    append_mutation_log(
        root,
        payload,
        &json!({
            "ts": now_iso(),
            "op": "set",
            "rel_path": rel,
            "actor": meta_actor(&meta),
            "source": meta_source(&meta),
            "reason": meta_reason(&meta, "mutate"),
            "lock_wait_ms": lock.waited_ms,
            "value_hash": canonical_hash(&value),
        }),
    );
    let pointer_stats = emit_adaptive_pointers(root, payload, &rel, &value, "set", &meta);
    let result = json!({
        "ok": true,
        "applied": true,
        "conflict": false,
        "value": value,
        "path": abs.to_string_lossy(),
        "rel": rel,
        "current_hash": canonical_hash(&read_json_value(&abs).unwrap_or(Value::Null)),
        "lock_wait_ms": lock.waited_ms,
        "pointer_stats": pointer_stats,
    });
    release_write_lock(lock);
    Ok(result)
}

fn delete_path_command(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let target = clean_text(payload.get("target_path"), 520);
    let meta = payload
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let (abs, rel) = resolve_adaptive_path(root, payload, &target)?;
    let lock = acquire_write_lock(&abs)?;
    let existed = abs.exists();
    if existed {
        fs::remove_file(&abs)
            .map_err(|err| format!("adaptive_layer_store_kernel_delete_failed:{err}"))?;
    }
    append_mutation_log(
        root,
        payload,
        &json!({
            "ts": now_iso(),
            "op": "delete",
            "rel_path": rel,
            "actor": meta_actor(&meta),
            "source": meta_source(&meta),
            "reason": meta_reason(&meta, "delete"),
            "lock_wait_ms": lock.waited_ms,
        }),
    );
    let tombstone = json!({
        "uid": stable_uid(&format!("adaptive_blob|{rel}|v1"), "a", 24),
    });
    let pointer_stats = emit_adaptive_pointers(root, payload, &rel, &tombstone, "delete", &meta);
    let result = json!({
        "ok": true,
        "deleted": existed,
        "path": abs.to_string_lossy(),
        "rel": rel,
        "lock_wait_ms": lock.waited_ms,
        "pointer_stats": pointer_stats,
    });
    release_write_lock(lock);
    Ok(result)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
    {
        usage();
        return 0;
    }

    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "paths".to_string());
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error(
                "adaptive_layer_store_kernel_error",
                err.as_str(),
            ));
            return 1;
        }
    };
    let payload = payload_obj(&payload);

    let receipt = match command.as_str() {
        "paths" => cli_receipt(
            "adaptive_layer_store_kernel_paths",
            paths_command(root, payload),
        ),
        "is-within-root" => cli_receipt(
            "adaptive_layer_store_kernel_is_within_root",
            is_within_root_command(root, payload),
        ),
        "resolve-path" => match resolve_path_command(root, payload) {
            Ok(value) => cli_receipt("adaptive_layer_store_kernel_resolve_path", value),
            Err(err) => cli_error("adaptive_layer_store_kernel_error", err.as_str()),
        },
        "resolve-provider-artifact-path" => match resolve_provider_artifact_path_command(root, payload)
        {
            Ok(value) => cli_receipt(
                "adaptive_layer_store_kernel_resolve_provider_artifact_path",
                value,
            ),
            Err(err) => cli_error("adaptive_layer_store_kernel_error", err.as_str()),
        },
        "read-json" => match read_json_command(root, payload) {
            Ok(value) => cli_receipt("adaptive_layer_store_kernel_read_json", value),
            Err(err) => cli_error("adaptive_layer_store_kernel_error", err.as_str()),
        },
        "ensure-json" => match ensure_json_command(root, payload) {
            Ok(value) => cli_receipt("adaptive_layer_store_kernel_ensure_json", value),
            Err(err) => cli_error("adaptive_layer_store_kernel_error", err.as_str()),
        },
        "set-json" => match set_json_command(root, payload) {
            Ok(value) => cli_receipt("adaptive_layer_store_kernel_set_json", value),
            Err(err) => cli_error("adaptive_layer_store_kernel_error", err.as_str()),
        },
        "delete-path" => match delete_path_command(root, payload) {
            Ok(value) => cli_receipt("adaptive_layer_store_kernel_delete_path", value),
            Err(err) => cli_error("adaptive_layer_store_kernel_error", err.as_str()),
        },
        _ => {
            usage();
            cli_error(
                "adaptive_layer_store_kernel_error",
                "adaptive_layer_store_kernel_unknown_command",
            )
        }
    };

    let exit_code = if receipt.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    };
    print_json_line(&receipt);
    exit_code
}
