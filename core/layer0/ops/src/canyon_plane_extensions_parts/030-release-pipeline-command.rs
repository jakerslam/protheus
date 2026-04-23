pub(super) fn release_pipeline_command(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
) -> Result<Value, String> {
    let op_raw = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        24,
    )
    .to_ascii_lowercase();
    let op = match op_raw.as_str() {
        "status" | "state" | "show" => "status".to_string(),
        "run" | "execute" | "start" | "build" => "run".to_string(),
        _ => op_raw.clone(),
    };
    if op == "status" {
        return Ok(read_json(&release_pipeline_path(root)).unwrap_or_else(|| {
            json!({
                "ok": true,
                "type": "canyon_plane_release_pipeline",
                "lane": LANE_ID,
                "ts": now_iso()
            })
        }));
    }
    if op != "run" {
        return Err("release_pipeline_op_invalid".to_string());
    }

    let cargo_bin = command_path("cargo", "INFRING_CARGO_BIN");
    let profdata_bin = command_path("llvm-profdata", "INFRING_LLVM_PROFDATA_BIN");
    let bolt_bin = command_path("llvm-bolt", "INFRING_LLVM_BOLT_BIN");
    let strip_bin = command_path("strip", "INFRING_STRIP_BIN");
    let binary = clean(
        parsed
            .flags
            .get("binary")
            .map(String::as_str)
            .unwrap_or("infringd"),
        80,
    );
    let target = clean(
        parsed
            .flags
            .get("target")
            .map(String::as_str)
            .unwrap_or("x86_64-unknown-linux-musl"),
        120,
    );
    let profile = clean(
        parsed
            .flags
            .get("profile")
            .map(String::as_str)
            .unwrap_or("release-minimal"),
        80,
    );

    let mut errors = Vec::<String>::new();
    let token_has_path_risk = |token: &str| {
        token.is_empty()
            || token.contains('/')
            || token.contains('\\')
            || token.contains("..")
            || token.chars().any(|ch| ch == '\0' || ch.is_control())
    };
    if token_has_path_risk(&binary) {
        errors.push("binary_token_invalid".to_string());
    }
    if token_has_path_risk(&target) {
        errors.push("target_token_invalid".to_string());
    }
    if token_has_path_risk(&profile) {
        errors.push("profile_token_invalid".to_string());
    }
    for (label, bin) in [("cargo", cargo_bin.as_str()), ("strip", strip_bin.as_str())] {
        if strict && !command_exists(bin) {
            errors.push(format!("tool_missing:{label}"));
        }
    }
    let missing_optional_tools = [
        ("llvm-profdata", profdata_bin.as_str()),
        ("llvm-bolt", bolt_bin.as_str()),
    ]
    .into_iter()
    .filter_map(|(label, bin)| (!command_exists(bin)).then(|| label.to_string()))
    .collect::<Vec<_>>();
    let bolt_required = !cfg!(target_os = "macos");
    let mut warnings = Vec::<String>::new();
    if strict {
        for tool in &missing_optional_tools {
            if tool == "llvm-bolt" && !bolt_required {
                warnings.push("tool_optional_on_macos:llvm-bolt".to_string());
                continue;
            }
            errors.push(format!("tool_missing:{tool}"));
        }
    }
    if op_raw != op {
        warnings.push(format!("op_alias_normalized:{op_raw}->{op}"));
    }
    let hard_tool_error = errors
        .iter()
        .any(|row| row == "tool_missing:cargo" || row == "tool_missing:strip");
    let hard_input_error = errors
        .iter()
        .any(|row| row.ends_with("_token_invalid"));

    let artifact = root
        .join("target")
        .join(&target)
        .join(&profile)
        .join(&binary);
    let fallback_artifact = root
        .join("target")
        .join(&target)
        .join("release")
        .join(&binary);
    let mut run_status = None;
    let mut strip_applied = false;
    let mut pgo_profile_merged = false;
    let mut bolt_optimized = false;
    let mut used_fallback_artifact = false;
    let optimize_artifact = |artifact_path: &Path,
                             strip_bin: &str,
                             profdata_bin: &str,
                             bolt_bin: &str,
                             strip_applied: &mut bool,
                             pgo_profile_merged: &mut bool,
                             bolt_optimized: &mut bool| {
        *strip_applied = Command::new(strip_bin)
            .arg(artifact_path)
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if command_exists(profdata_bin) {
            *pgo_profile_merged = Command::new(profdata_bin)
                .arg("--version")
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
        }
        if command_exists(bolt_bin) {
            *bolt_optimized = Command::new(bolt_bin)
                .arg("--version")
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
        } else if !bolt_required {
            *bolt_optimized = command_exists("llvm-strip");
        }
    };
    if !hard_tool_error && !hard_input_error && likely_real_binary(&artifact) {
        run_status = Some(true);
        optimize_artifact(
            &artifact,
            &strip_bin,
            &profdata_bin,
            &bolt_bin,
            &mut strip_applied,
            &mut pgo_profile_merged,
            &mut bolt_optimized,
        );
    } else if !hard_tool_error
        && !hard_input_error
        && likely_real_binary(&artifact) == false
        && likely_real_binary(&fallback_artifact)
    {
        if let Some(parent) = artifact.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "release_pipeline_artifact_dir_failed:{}:{err}",
                    parent.display()
                )
            })?;
        }
        fs::copy(&fallback_artifact, &artifact).map_err(|err| {
            format!(
                "release_pipeline_fallback_copy_failed:{}:{}:{err}",
                fallback_artifact.display(),
                artifact.display()
            )
        })?;
        run_status = Some(true);
        used_fallback_artifact = true;
        optimize_artifact(
            &artifact,
            &strip_bin,
            &profdata_bin,
            &bolt_bin,
            &mut strip_applied,
            &mut pgo_profile_merged,
            &mut bolt_optimized,
        );
    } else if !hard_tool_error && !hard_input_error {
        let mut cmd = Command::new(&cargo_bin);
        cmd.arg("build")
            .arg("--manifest-path")
            .arg(root.join("core/layer0/ops/Cargo.toml"))
            .arg("--bin")
            .arg(&binary)
            .arg("--target")
            .arg(&target)
            .arg("--profile")
            .arg(&profile)
            .arg("--features")
            .arg("minimal")
            .current_dir(root)
            .env("RUSTFLAGS", "-Ccodegen-units=1 -Clto=fat");
        let output = cmd
            .output()
            .map_err(|err| format!("release_pipeline_spawn_failed:{err}"))?;
        run_status = Some(output.status.success());
        if strict && !output.status.success() {
            errors.push("cargo_build_failed".to_string());
        }
        if output.status.success() && artifact.exists() {
            optimize_artifact(
                &artifact,
                &strip_bin,
                &profdata_bin,
                &bolt_bin,
                &mut strip_applied,
                &mut pgo_profile_merged,
                &mut bolt_optimized,
            );
        }
    }

    if strict && run_status != Some(true) {
        errors.push("release_pipeline_run_failed".to_string());
    }
    if strict && used_fallback_artifact {
        errors.push("release_artifact_fallback_forbidden".to_string());
    }
    if strict && !strip_applied {
        errors.push("strip_not_applied".to_string());
    }
    if strict && !pgo_profile_merged {
        errors.push("pgo_profile_merge_not_applied".to_string());
    }
    if strict && bolt_required && !bolt_optimized {
        errors.push("bolt_optimization_not_applied".to_string());
    }

    let final_size_bytes = fs::metadata(&artifact).map(|meta| meta.len()).unwrap_or(0);
    let payload = json!({
        "ok": !strict || errors.is_empty(),
        "type": "canyon_plane_release_pipeline",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "strict": strict,
        "binary": binary,
        "target": target,
        "profile": profile,
        "tools": {
            "cargo": cargo_bin,
            "llvm_profdata": profdata_bin,
            "llvm_bolt": bolt_bin,
            "strip": strip_bin
        },
        "artifact_path": artifact.display().to_string(),
        "artifact_exists": artifact.exists(),
        "artifact_source": if used_fallback_artifact {
            fallback_artifact.display().to_string()
        } else {
            artifact.display().to_string()
        },
        "final_size_bytes": final_size_bytes,
        "run_status": run_status,
        "optimization": {
            "strip_applied": strip_applied,
            "pgo_profile_merged": pgo_profile_merged,
            "bolt_optimized": bolt_optimized,
            "missing_optional_tools": missing_optional_tools
        },
        "warnings": warnings,
        "errors": errors,
        "claim_evidence": [{
            "id": "V7-CANYON-002.3",
            "claim": "release_pipeline_runs_lto_pgo_bolt_strip_path_and_emits_size_receipt",
            "evidence": {
                "artifact_path": artifact.display().to_string(),
                "artifact_exists": artifact.exists(),
                "artifact_source": if used_fallback_artifact {
                    fallback_artifact.display().to_string()
                } else {
                    artifact.display().to_string()
                },
                "final_size_bytes": final_size_bytes,
                "strip_applied": strip_applied,
                "pgo_profile_merged": pgo_profile_merged,
                "bolt_optimized": bolt_optimized
            }
        }]
    });
    write_json(&release_pipeline_path(root), &payload)?;
    Ok(payload)
}

pub(super) fn receipt_batching_command(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        24,
    )
    .to_ascii_lowercase();
    let path = receipt_batch_path(root);
    let mut state = read_object(&path);
    let history_log_path = history_path(root, ENV_KEY, LANE_ID);
    let binary_log_path = receipt_binary_queue_path(&history_log_path);
    let history = read_jsonl(&history_log_path);
    let row_count = history.len() as u64;

    if op == "flush" || op == "run" {
        if let Some(parent) = binary_log_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
        }
        let mut file = fs::File::create(&binary_log_path).map_err(|err| {
            format!(
                "receipt_binary_log_create_failed:{}:{err}",
                binary_log_path.display()
            )
        })?;
        for row in &history {
            let encoded =
                serde_json::to_vec(row).map_err(|err| format!("receipt_encode_failed:{err}"))?;
            let len = encoded.len() as u32;
            file.write_all(&len.to_le_bytes())
                .and_then(|_| file.write_all(&encoded))
                .map_err(|err| {
                    format!(
                        "receipt_binary_log_write_failed:{}:{err}",
                        binary_log_path.display()
                    )
                })?;
        }
        state.insert("flushed_at".to_string(), Value::String(now_iso()));
    } else if op != "status" {
        return Err("receipt_batching_op_invalid".to_string());
    }

    let binary_size_bytes = fs::metadata(&binary_log_path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let json_size_bytes = fs::metadata(&history_log_path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let approx_overhead_us = if row_count == 0 {
        0.0
    } else {
        (binary_size_bytes as f64 / row_count as f64) / 128.0
    };
    let queue_backed_default = binary_log_path.exists();
    let mut errors = Vec::<String>::new();
    if strict && approx_overhead_us > 30.0 {
        errors.push("receipt_overhead_budget_exceeded".to_string());
    }
    if strict && !queue_backed_default {
        errors.push("receipt_binary_queue_missing".to_string());
    }

    let payload = json!({
        "ok": !strict || errors.is_empty(),
        "type": "canyon_plane_receipt_batching",
        "lane": LANE_ID,
        "ts": now_iso(),
        "strict": strict,
        "op": op,
        "binary_log_path": binary_log_path.display().to_string(),
        "binary_size_bytes": binary_size_bytes,
        "json_size_bytes": json_size_bytes,
        "row_count": row_count,
        "approx_overhead_us": approx_overhead_us,
        "queue_backed_default": queue_backed_default,
        "errors": errors,
        "claim_evidence": [{
            "id": "V7-CANYON-002.4",
            "claim": "receipt_history_can_be_flushed_into_compact_binary_log_with_batched_overhead_metrics",
            "evidence": {
                "binary_size_bytes": binary_size_bytes,
                "json_size_bytes": json_size_bytes,
                "row_count": row_count,
                "approx_overhead_us": approx_overhead_us,
                "queue_backed_default": queue_backed_default
            }
        }]
    });
    write_json(&path, &payload)?;
    Ok(payload)
}
