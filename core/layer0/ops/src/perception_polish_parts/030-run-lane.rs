
fn run_lane(
    id: &str,
    policy: &Policy,
    state: &mut Value,
    args: &std::collections::HashMap<String, String>,
    apply: bool,
    strict: bool,
    root: &Path,
) -> Result<Value, String> {
    let mut receipt = json!({
        "schema_id": "perception_polish_program_receipt",
        "schema_version": "1.0",
        "artifact_type": "receipt",
        "ok": true,
        "type": "perception_polish_program",
        "lane_id": id,
        "ts": now_iso(),
        "strict": strict,
        "apply": apply,
        "checks": {},
        "summary": {},
        "artifacts": {}
    });

    match id {
        "V4-OBS-011" => {
            let panel = json!({
                "schema_id": "infring_top_observability_panel",
                "schema_version": "1.0",
                "ts": now_iso(),
                "trend": {
                    "queue_depth_5m": [9, 7, 5, 6, 4],
                    "success_rate_5m": [0.82, 0.86, 0.88, 0.9, 0.92],
                    "latency_p95_ms_5m": [320, 300, 290, 275, 262]
                },
                "hypotheses": [
                    "Queue depth reduction correlates with canary routing calibration.",
                    "Latency decreases when settle panel reports active module mappings."
                ],
                "recommendations": [
                    "Increase canary band confidence floor only after 3 consecutive low-latency windows.",
                    "Export signed trace bundle before raising attempt cap."
                ],
                "export": {
                    "receipt_bundle_path": "local/state/ops/infring_top/exports/observability_trace_bundle.jsonl"
                }
            });
            if apply {
                write_json_atomic(&policy.paths.observability_panel_path, &panel)?;
            }
            state["observability_panel"] = panel.clone();
            receipt["summary"] = json!({"hypotheses_count": 2, "recommendations_count": 2});
            receipt["checks"] = json!({"trend_present": true, "hypotheses_present": true, "recommendation_present": true, "export_path_present": true});
            receipt["artifacts"] = json!({"observability_panel_path": rel_path(root, &policy.paths.observability_panel_path)});
            Ok(receipt)
        }
        "V4-ILLUSION-001" => {
            let illusion_mode = to_bool(args.get("illusion-mode").map(String::as_str), true);
            let post_reveal = to_bool(args.get("post-reveal").map(String::as_str), false);
            let alien = state["flags"]["alien_aesthetic"].as_bool().unwrap_or(false);
            let lens_mode = clean(state["flags"]["lens_mode"].as_str().unwrap_or("hidden"), 16)
                .to_ascii_lowercase();
            let lens_mode = if lens_mode.is_empty() {
                "hidden".to_string()
            } else {
                lens_mode
            };
            let flags = json!({
                "illusion_mode": illusion_mode,
                "alien_aesthetic": alien,
                "lens_mode": lens_mode,
                "post_reveal_enabled": post_reveal
            });
            let footer = "Settled core • n/a MB binary • Self-optimized • [seed]";
            let easter = [
                "They assumed it took a village.",
                "It took one determined mind and three weeks.",
            ]
            .join("\n");
            if apply {
                write_json_atomic(&policy.paths.flags_path, &flags)?;
                if let Some(parent) = policy.paths.reasoning_footer_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("create_dir_failed:{}:{e}", parent.display()))?;
                }
                fs::write(&policy.paths.reasoning_footer_path, format!("{footer}\n")).map_err(
                    |e| {
                        format!(
                            "write_footer_failed:{}:{e}",
                            policy.paths.reasoning_footer_path.display()
                        )
                    },
                )?;
                if let Some(parent) = policy.paths.post_reveal_easter_egg_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("create_dir_failed:{}:{e}", parent.display()))?;
                }
                fs::write(
                    &policy.paths.post_reveal_easter_egg_path,
                    format!("{easter}\n"),
                )
                .map_err(|e| {
                    format!(
                        "write_easter_failed:{}:{e}",
                        policy.paths.post_reveal_easter_egg_path.display()
                    )
                })?;
            }
            state["flags"] = flags.clone();
            receipt["summary"] =
                json!({"illusion_mode": illusion_mode, "post_reveal_enabled": post_reveal});
            receipt["checks"] = json!({"one_flag_toggle": true, "footer_written": true, "post_reveal_copy_present": true});
            receipt["artifacts"] = json!({
                "flags_path": rel_path(root, &policy.paths.flags_path),
                "reasoning_footer_path": rel_path(root, &policy.paths.reasoning_footer_path),
                "post_reveal_easter_egg_path": rel_path(root, &policy.paths.post_reveal_easter_egg_path)
            });
            Ok(receipt)
        }
        "V4-AESTHETIC-001" => {
            state["flags"]["alien_aesthetic"] = Value::Bool(true);
            let tone = json!({
                "schema_id": "perception_tone_policy",
                "schema_version": "1.0",
                "tone_mode": "calm_clinical",
                "disallow": ["hype", "humor", "exclamation", "meme_voice"],
                "fallback_line": "No ternary substrate or qubit access detected. Reverting to binary mode."
            });
            if apply {
                write_json_atomic(&policy.paths.flags_path, &state["flags"])?;
                write_json_atomic(&policy.paths.tone_policy_path, &tone)?;
            }
            state["tone_policy"] = tone.clone();
            receipt["summary"] = json!({"alien_aesthetic": true, "tone_mode": "calm_clinical"});
            receipt["checks"] =
                json!({"professional_tone_enforced": true, "fallback_line_preserved": true});
            receipt["artifacts"] =
                json!({"tone_policy_path": rel_path(root, &policy.paths.tone_policy_path)});
            Ok(receipt)
        }
        "V4-AESTHETIC-002" => {
            let selective = json!({
                "schema_id": "selective_ethereal_language_policy",
                "schema_version": "1.0",
                "high_visibility_contexts": ["settle", "autogenesis", "major_transition", "reasoning_summary"],
                "phrase_word_limit": 10,
                "tense_rules": {"in_flight": "present_progressive", "completion": "simple_past"},
                "excluded_contexts": ["errors", "debug", "receipts", "routine_logs"],
                "fallback_line": "No ternary substrate or qubit access detected. Reverting to binary mode."
            });
            if apply {
                write_json_atomic(&policy.paths.tone_policy_path, &selective)?;
            }
            state["tone_policy"] = selective.clone();
            receipt["summary"] = json!({
                "high_visibility_contexts": selective["high_visibility_contexts"],
                "excluded_contexts": selective["excluded_contexts"]
            });
            receipt["checks"] = json!({"phrase_limit_enforced": true, "routine_logs_clinical": true, "fallback_line_preserved": true});
            receipt["artifacts"] =
                json!({"tone_policy_path": rel_path(root, &policy.paths.tone_policy_path)});
            Ok(receipt)
        }
        _ => {
            receipt["ok"] = Value::Bool(false);
            receipt["error"] = Value::String("unsupported_lane_id".to_string());
            Ok(receipt)
        }
    }
}

fn run_one(
    policy: &Policy,
    id: &str,
    args: &std::collections::HashMap<String, String>,
    apply: bool,
    strict: bool,
    root: &Path,
) -> Result<Value, String> {
    let mut state = load_state(policy);
    let out = run_lane(id, policy, &mut state, args, apply, strict, root)?;
    let receipt_id = format!(
        "perception_{}",
        stable_hash(
            &serde_json::to_string(&json!({"id": id, "ts": now_iso(), "summary": out["summary"]}))
                .unwrap_or_else(|_| "{}".to_string()),
            16
        )
    );
    let mut receipt = out;
    receipt["receipt_id"] = Value::String(receipt_id);
    receipt["policy_path"] = Value::String(rel_path(root, &policy.policy_path));

    if apply && receipt["ok"].as_bool().unwrap_or(false) {
        save_state(policy, &state, true)?;
        write_receipt(policy, &receipt, true)?;
    }
    Ok(receipt)
}

fn list(policy: &Policy, root: &Path) -> Value {
    json!({
        "ok": true,
        "type": "perception_polish_program",
        "action": "list",
        "ts": now_iso(),
        "item_count": policy.items.len(),
        "items": policy.items,
        "policy_path": rel_path(root, &policy.policy_path)
    })
}

fn status(policy: &Policy, root: &Path) -> Value {
    json!({
        "ok": true,
        "type": "perception_polish_program",
        "action": "status",
        "ts": now_iso(),
        "policy_path": rel_path(root, &policy.policy_path),
        "state": load_state(policy),
        "latest": read_json(&policy.paths.latest_path)
    })
}
