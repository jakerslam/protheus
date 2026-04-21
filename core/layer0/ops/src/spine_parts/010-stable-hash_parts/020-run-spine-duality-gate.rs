
fn run_spine_duality_gate(
    root: &Path,
    run_id: &str,
    mode: &str,
    date: &str,
    run_context: &str,
) -> Value {
    // V4-DUAL-SPI-004: spine orchestration 0-point enforcer before major lane execution.
    let context = json!({
        "lane": "weaver_arbitration",
        "source": "spine_orchestration",
        "run_id": run_id,
        "mode": mode,
        "date": date,
        "run_context": run_context
    });

    let evaluation = match crate::duality_seed::invoke(
        root,
        "duality_evaluate",
        Some(&json!({
            "context": context,
            "opts": {
                "persist": true,
                "lane": "weaver_arbitration",
                "source": "spine_orchestration",
                "run_id": run_id
            }
        })),
    ) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "spine_duality_gate",
                "error": format!("duality_evaluate_failed:{err}")
            });
        }
    };

    let dual_voice = crate::duality_seed::invoke(
        root,
        "dual_voice_evaluate",
        Some(&json!({
            "context": {
                "lane": "weaver_arbitration",
                "source": "spine_orchestration",
                "run_id": run_id,
                "mode": mode,
                "date": date
            },
            "left": {
                "policy_lens": "guardian",
                "focus": "safety_and_order"
            },
            "right": {
                "policy_lens": "strategist",
                "focus": "adaptation_and_inversion"
            },
            "opts": {
                "persist": true,
                "source": "spine_orchestration",
                "run_id": run_id
            }
        })),
    )
    .unwrap_or_else(|_| json!({"ok": false, "type": "duality_dual_voice_evaluation"}));

    let toll_update = match crate::duality_seed::invoke(
        root,
        "duality_toll_update",
        Some(&json!({
            "context": {
                "lane": "weaver_arbitration",
                "source": "spine_orchestration",
                "run_id": run_id,
                "mode": mode,
                "date": date
            },
            "signal": evaluation.clone(),
            "opts": {
                "persist": true,
                "source": "spine_orchestration",
                "run_id": run_id
            }
        })),
    ) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "spine_duality_gate",
                "evaluation": evaluation,
                "dual_voice": dual_voice,
                "error": format!("duality_toll_update_failed:{err}")
            });
        }
    };

    let toll = toll_update.get("toll").cloned().unwrap_or_else(|| json!({}));
    let debt_after = value_f64(toll.get("debt_after"), 0.0);
    let hard_block = toll
        .get("hard_block")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let harmony = value_f64(
        dual_voice.get("harmony"),
        value_f64(evaluation.get("zero_point_harmony_potential"), 0.0),
    )
    .clamp(0.0, 1.0);
    let clearance_before = parse_clearance_level(std::env::var("CLEARANCE").ok(), 3);
    let (clearance_after, clearance_reason) =
        derive_duality_clearance(clearance_before, debt_after, harmony, hard_block);
    std::env::set_var("CLEARANCE", clearance_after.to_string());

    json!({
        "ok": true,
        "type": "spine_duality_gate",
        "run_id": run_id,
        "evaluation": evaluation,
        "dual_voice": dual_voice,
        "toll": toll,
        "state": toll_update.get("state").cloned().unwrap_or(Value::Null),
        "hard_block": hard_block,
        "clearance": {
            "before": clearance_before,
            "after": clearance_after,
            "reason": clearance_reason
        },
        "fractal_balance_score": ((harmony * (1.0 - debt_after.min(1.0))) * 1_000_000.0).round() / 1_000_000.0
    })
}

fn receipt_ledger_io_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn to_base36(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while n > 0 {
        let digit = (n % 36) as u8;
        let ch = if digit < 10 {
            (b'0' + digit) as char
        } else {
            (b'a' + (digit - 10)) as char
        };
        out.push(ch);
        n /= 36;
    }
    out.into_iter().rev().collect()
}

fn parse_cli(argv: &[String]) -> Option<CliArgs> {
    if argv.is_empty() {
        return None;
    }

    let mut idx = 0usize;
    let mut command = "run".to_string();
    let mut mode = argv[idx].to_ascii_lowercase();
    if mode == "status" {
        command = "status".to_string();
        mode = "daily".to_string();
    } else if mode == "run" {
        idx += 1;
        mode = argv.get(idx)?.to_ascii_lowercase();
    }

    if command != "status" && mode != "eyes" && mode != "daily" {
        return None;
    }

    if command != "status" {
        idx += 1;
    }
    let mut date = argv
        .get(idx)
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() == 10 && s.chars().nth(4) == Some('-') && s.chars().nth(7) == Some('-'))
        .unwrap_or_else(|| now_iso()[..10].to_string());

    let mut max_eyes = None::<i64>;
    let mut i = 0usize;
    while i < argv.len() {
        let token = argv[i].trim();
        if let Some((k, v)) = token.split_once('=') {
            if k == "--max-eyes" {
                if let Ok(n) = v.parse::<i64>() {
                    max_eyes = Some(n.clamp(1, 500));
                }
            } else if k == "--mode" {
                let candidate = v.trim().to_ascii_lowercase();
                if candidate == "eyes" || candidate == "daily" {
                    mode = candidate;
                }
            } else if k == "--date" {
                let candidate = v.trim();
                if candidate.len() == 10
                    && candidate.chars().nth(4) == Some('-')
                    && candidate.chars().nth(7) == Some('-')
                {
                    date = candidate.to_string();
                }
            }
            i += 1;
            continue;
        }
        if token == "--max-eyes" {
            if let Some(next) = argv.get(i + 1) {
                if !next.starts_with("--") {
                    if let Ok(n) = next.trim().parse::<i64>() {
                        max_eyes = Some(n.clamp(1, 500));
                    }
                    i += 2;
                    continue;
                }
            }
        } else if token == "--mode" {
            if let Some(next) = argv.get(i + 1) {
                let candidate = next.trim().to_ascii_lowercase();
                if !next.starts_with("--") && (candidate == "eyes" || candidate == "daily") {
                    mode = candidate;
                    i += 2;
                    continue;
                }
            }
        } else if token == "--date" {
            if let Some(next) = argv.get(i + 1) {
                let candidate = next.trim();
                if !next.starts_with("--")
                    && candidate.len() == 10
                    && candidate.chars().nth(4) == Some('-')
                    && candidate.chars().nth(7) == Some('-')
                {
                    date = candidate.to_string();
                    i += 2;
                    continue;
                }
            }
        }
        i += 1;
    }

    Some(CliArgs {
        command,
        mode,
        date,
        max_eyes,
    })
}
