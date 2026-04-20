
fn op_dispatch(root: &Path, op: &str, args: Option<&Value>) -> Result<Value, String> {
    let args_obj = args.and_then(Value::as_object).cloned().unwrap_or_default();
    let policy_path = as_str(args_obj.get("policy_path"));
    let policy = load_policy(
        root,
        if policy_path.is_empty() {
            None
        } else {
            Some(policy_path.as_str())
        },
    );

    match op {
        "loadDualityPolicy" => Ok(policy),
        "parseDualityCodexText" => {
            let text = as_str(args_obj.get("text"));
            Ok(parse_codex_text(&text))
        }
        "loadDualityCodex" => Ok(load_codex(&policy)),
        "loadDualityState" => Ok(load_state(&policy)),
        "evaluateDualitySignal" | "duality_evaluate" => {
            let state = load_state(&policy);
            let opts = args_obj.get("opts").cloned().unwrap_or_else(|| json!({}));
            let skip_validation = as_bool(opts.get("skip_validation"), false);
            let state_after_validation = if skip_validation {
                state.clone()
            } else {
                maybe_run_self_validation(
                    &policy,
                    &state,
                    if policy_path.is_empty() {
                        None
                    } else {
                        Some(policy_path.as_str())
                    },
                )?
            };
            let context = args_obj
                .get("context")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let out = evaluate_signal(
                &policy,
                &load_codex(&policy),
                &state_after_validation,
                &context,
                &opts,
            );

            if as_bool(opts.get("persist"), false)
                && as_bool(
                    policy
                        .get("outputs")
                        .and_then(Value::as_object)
                        .and_then(|obj| obj.get("persist_shadow_receipts")),
                    true,
                )
            {
                let history_path = policy
                    .get("state")
                    .and_then(Value::as_object)
                    .and_then(|obj| obj.get("history_path"))
                    .map(|v| PathBuf::from(as_str(Some(v))))
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_HISTORY_REL));
                append_jsonl(
                    &history_path,
                    &json!({
                        "ts": now_iso(),
                        "type": "duality_evaluation",
                        "lane": out.get("lane").cloned().unwrap_or(Value::Null),
                        "run_id": out.get("run_id").cloned().unwrap_or(Value::Null),
                        "source": out
                            .get("diagnostics")
                            .and_then(Value::as_object)
                            .and_then(|obj| obj.get("source"))
                            .cloned()
                            .unwrap_or(Value::Null),
                        "score_trit": out.get("score_trit").cloned().unwrap_or(Value::Null),
                        "balance_score": out.get("balance_score").cloned().unwrap_or(Value::Null),
                        "zero_point_harmony_potential": out
                            .get("zero_point_harmony_potential")
                            .cloned()
                            .unwrap_or(Value::Null),
                        "confidence": out.get("confidence").cloned().unwrap_or(Value::Null),
                        "effective_weight": out.get("effective_weight").cloned().unwrap_or(Value::Null),
                        "recommended_adjustment": out
                            .get("recommended_adjustment")
                            .cloned()
                            .unwrap_or(Value::Null)
                    }),
                )?;
            }

            Ok(out)
        }
        "dualVoiceEvaluate" | "dual_voice_evaluate" => {
            let state = load_state(&policy);
            let opts = args_obj.get("opts").cloned().unwrap_or_else(|| json!({}));
            let skip_validation = as_bool(opts.get("skip_validation"), false);
            let state_after_validation = if skip_validation {
                state.clone()
            } else {
                maybe_run_self_validation(
                    &policy,
                    &state,
                    if policy_path.is_empty() {
                        None
                    } else {
                        Some(policy_path.as_str())
                    },
                )?
            };
            let context = args_obj
                .get("context")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let left = args_obj
                .get("left")
                .or_else(|| args_obj.get("left_context"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let right = args_obj
                .get("right")
                .or_else(|| args_obj.get("right_context"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let out = evaluate_dual_voice_signal(
                &policy,
                &load_codex(&policy),
                &state_after_validation,
                &context,
                &left,
                &right,
                &opts,
            );

            if as_bool(opts.get("persist"), false)
                && as_bool(
                    policy
                        .get("outputs")
                        .and_then(Value::as_object)
                        .and_then(|obj| obj.get("persist_shadow_receipts")),
                    true,
                )
            {
                let history_path = policy
                    .get("state")
                    .and_then(Value::as_object)
                    .and_then(|obj| obj.get("history_path"))
                    .map(|v| PathBuf::from(as_str(Some(v))))
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_HISTORY_REL));
                append_jsonl(
                    &history_path,
                    &json!({
                        "ts": now_iso(),
                        "type": "duality_dual_voice_evaluation",
                        "run_id": out.get("run_id").cloned().unwrap_or(Value::Null),
                        "source": out.get("source").cloned().unwrap_or(Value::Null),
                        "score_trit": out.get("score_trit").cloned().unwrap_or(Value::Null),
                        "harmony": out.get("harmony").cloned().unwrap_or(Value::Null),
                        "pass": out.get("pass").cloned().unwrap_or(Value::Bool(false))
                    }),
                )?;
            }

