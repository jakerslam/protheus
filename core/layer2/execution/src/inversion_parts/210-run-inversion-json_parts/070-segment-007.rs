        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "read_json",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_read_json_failed:{e}"));
    }
    if mode == "read_jsonl" {
        let input: ReadJsonlInput = decode_input(&payload, "read_jsonl_input")?;
        let out = compute_read_jsonl(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "read_jsonl",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_read_jsonl_failed:{e}"));
    }
    if mode == "write_json_atomic" {
        let input: WriteJsonAtomicInput = decode_input(&payload, "write_json_atomic_input")?;
        let out = compute_write_json_atomic(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "write_json_atomic",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_write_json_atomic_failed:{e}"));
    }
    if mode == "append_jsonl" {
        let input: AppendJsonlInput = decode_input(&payload, "append_jsonl_input")?;
        let out = compute_append_jsonl(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "append_jsonl",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_append_jsonl_failed:{e}"));
    }
    if mode == "read_text" {
        let input: ReadTextInput = decode_input(&payload, "read_text_input")?;
        let out = compute_read_text(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "read_text",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_read_text_failed:{e}"));
    }
    if mode == "latest_json_file_in_dir" {
        let input: LatestJsonFileInDirInput =
            decode_input(&payload, "latest_json_file_in_dir_input")?;
        let out = compute_latest_json_file_in_dir(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "latest_json_file_in_dir",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_latest_json_file_in_dir_failed:{e}"));
    }
    if mode == "normalize_output_channel" {
        let input: NormalizeOutputChannelInput =
            decode_input(&payload, "normalize_output_channel_input")?;
        let out = compute_normalize_output_channel(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_output_channel",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_output_channel_failed:{e}"));
    }
    if mode == "normalize_repo_path" {
        let input: NormalizeRepoPathInput = decode_input(&payload, "normalize_repo_path_input")?;
        let out = compute_normalize_repo_path(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_repo_path",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_repo_path_failed:{e}"));
    }
    if mode == "runtime_paths" {
        let input: RuntimePathsInput = decode_input(&payload, "runtime_paths_input")?;
        let out = compute_runtime_paths(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "runtime_paths",
            "payload": out.paths
        }))
        .map_err(|e| format!("inversion_encode_runtime_paths_failed:{e}"));
    }
    if mode == "normalize_axiom_list" {
        let input: NormalizeAxiomListInput = decode_input(&payload, "normalize_axiom_list_input")?;
        let out = compute_normalize_axiom_list(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_axiom_list",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_axiom_list_failed:{e}"));
    }
    if mode == "normalize_harness_suite" {
        let input: NormalizeHarnessSuiteInput =
            decode_input(&payload, "normalize_harness_suite_input")?;
        let out = compute_normalize_harness_suite(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_harness_suite",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_harness_suite_failed:{e}"));
    }
    if mode == "load_harness_state" {
        let input: LoadHarnessStateInput = decode_input(&payload, "load_harness_state_input")?;
        let out = compute_load_harness_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "load_harness_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_load_harness_state_failed:{e}"));
    }
    if mode == "save_harness_state" {
        let input: SaveHarnessStateInput = decode_input(&payload, "save_harness_state_input")?;
        let out = compute_save_harness_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "save_harness_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_save_harness_state_failed:{e}"));
    }
    if mode == "load_first_principle_lock_state" {
        let input: LoadFirstPrincipleLockStateInput =
            decode_input(&payload, "load_first_principle_lock_state_input")?;
        let out = compute_load_first_principle_lock_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "load_first_principle_lock_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_load_first_principle_lock_state_failed:{e}"));
    }
    if mode == "save_first_principle_lock_state" {
        let input: SaveFirstPrincipleLockStateInput =
            decode_input(&payload, "save_first_principle_lock_state_input")?;
        let out = compute_save_first_principle_lock_state(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "save_first_principle_lock_state",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_save_first_principle_lock_state_failed:{e}"));
    }
    if mode == "check_first_principle_downgrade" {
        let input: CheckFirstPrincipleDowngradeInput =
            decode_input(&payload, "check_first_principle_downgrade_input")?;
        let out = compute_check_first_principle_downgrade(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "check_first_principle_downgrade",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_check_first_principle_downgrade_failed:{e}"));
    }
    if mode == "upsert_first_principle_lock" {
        let input: UpsertFirstPrincipleLockInput =
            decode_input(&payload, "upsert_first_principle_lock_input")?;
        let out = compute_upsert_first_principle_lock(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "upsert_first_principle_lock",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_upsert_first_principle_lock_failed:{e}"));
    }
    if mode == "load_observer_approvals" {
        let input: LoadObserverApprovalsInput =
            decode_input(&payload, "load_observer_approvals_input")?;
        let out = compute_load_observer_approvals(&input);
        return serde_json::to_string(&json!({
            "ok": true,
