pub fn build_queue_rows(req: &QueueRowsRequest) -> (Vec<Value>, Vec<Value>) {
    let mut weaver: Vec<Value> = Vec::new();
    let mut storm: Vec<Value> = Vec::new();

    for task in &req.tasks {
        let route = task.get("route").and_then(|v| v.as_object());
        let lane = route
            .and_then(|row| row.get("lane"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let blocked = route
            .and_then(|row| row.get("blocked"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let manual = route
            .and_then(|row| row.get("requires_manual_review"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let parallel_group = route
            .and_then(|row| row.get("parallel_group"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let parallel_priority = route
            .and_then(|row| row.get("parallel_priority"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let weaver_row = json!({
            "type": "task_micro_route_candidate",
            "run_id": req.run_id,
            "goal_id": req.goal_id,
            "objective_id": req.objective_id,
            "micro_task_id": task.get("micro_task_id").cloned().unwrap_or(Value::Null),
            "profile_id": task.get("profile_id").cloned().unwrap_or(Value::Null),
            "lane": lane,
            "parallel_group": parallel_group,
            "parallel_priority": parallel_priority,
            "blocked": blocked,
            "requires_manual_review": manual,
            "shadow_only": req.shadow_only,
            "passport_id": req.passport_id,
            "duality_indicator": duality_indicator_for_task(task),
            "attribution": attribution_for_task(task)
        });
        weaver.push(weaver_row);

        if lane == req.storm_lane && !blocked {
            let storm_row = json!({
                "type": "storm_micro_task_offer",
                "run_id": req.run_id,
                "goal_id": req.goal_id,
                "objective_id": req.objective_id,
                "micro_task_id": task.get("micro_task_id").cloned().unwrap_or(Value::Null),
                "title": task.get("title").cloned().unwrap_or(Value::Null),
                "task_text": task.get("task_text").cloned().unwrap_or(Value::Null),
                "estimated_minutes": task.get("estimated_minutes").cloned().unwrap_or(Value::Null),
                "success_criteria": task.get("success_criteria").cloned().unwrap_or_else(|| json!([])),
                "profile_id": task.get("profile_id").cloned().unwrap_or(Value::Null),
                "shadow_only": req.shadow_only,
                "passport_id": req.passport_id,
                "duality_indicator": duality_indicator_for_task(task)
            });
            storm.push(storm_row);
        }
    }

    (weaver, storm)
}

pub fn queue_rows_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<QueueRowsRequest>(payload)
        .map_err(|err| format!("queue_rows_payload_parse_failed:{}", err))?;
    let (weaver, storm) = build_queue_rows(&req);
    let resp = QueueRowsResponse {
        ok: true,
        weaver,
        storm,
    };
    serde_json::to_string(&resp)
        .map_err(|err| format!("queue_rows_payload_serialize_failed:{}", err))
}

pub fn build_dispatch_rows(req: &DispatchRowsRequest) -> Vec<Value> {
    let storm_lane = {
        let lane = normalize_token(req.storm_lane.as_str(), 80);
        if lane.is_empty() {
            default_storm_lane()
        } else {
            lane
        }
    };
    let autonomous_executor = {
        let executor = normalize_token(req.autonomous_executor.as_str(), 80);
        if executor.is_empty() {
            default_autonomous_executor()
        } else {
            executor
        }
    };
    let storm_executor = {
        let executor = normalize_token(req.storm_executor.as_str(), 80);
        if executor.is_empty() {
            default_storm_executor()
        } else {
            executor
        }
    };

    req.tasks
        .iter()
        .map(|task| {
            let route = task.get("route").and_then(|v| v.as_object());
            let governance = task.get("governance").and_then(|v| v.as_object());
            let lane = {
                let normalized = normalize_token(
                    route
                        .and_then(|row| row.get("lane"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown"),
                    80,
                );
                if normalized.is_empty() {
                    "unknown".to_string()
                } else {
                    normalized
                }
            };
            let blocked = governance
                .and_then(|row| row.get("blocked"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let executor = if lane == storm_lane {
                storm_executor.clone()
            } else {
                autonomous_executor.clone()
            };

            json!({
                "type": "task_micro_execution_dispatch",
                "run_id": req.run_id,
                "goal_id": req.goal_id,
                "objective_id": req.objective_id,
                "micro_task_id": task.get("micro_task_id").cloned().unwrap_or(Value::Null),
                "profile_id": task.get("profile_id").cloned().unwrap_or(Value::Null),
                "lane": lane,
                "executor": executor,
                "blocked": blocked,
                "shadow_only": req.shadow_only,
                "apply_executed": req.apply_executed,
                "status": if blocked { "blocked" } else { "queued" },
                "passport_id": req.passport_id
            })
        })
        .collect()
}

pub fn dispatch_rows_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<DispatchRowsRequest>(payload)
        .map_err(|err| format!("dispatch_rows_payload_parse_failed:{}", err))?;
    let resp = DispatchRowsResponse {
        ok: true,
        rows: build_dispatch_rows(&req),
    };
    serde_json::to_string(&resp)
        .map_err(|err| format!("dispatch_rows_payload_serialize_failed:{}", err))
}

fn contains_any(source: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| source.contains(needle))
}

fn normalize_route_intent(task_text: &str) -> String {
    if task_text.trim().is_empty() {
        return String::new();
    }

    let mut out = task_text.to_ascii_lowercase();
    let strip_patterns = [
        r"\b\d{4}-\d{2}-\d{2}\b",
        r"\b[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}\b",
        r"\d{4}-\d{2}-\d{2}t\d{2}:\d{2}:\d{2}(.\d+)?(z|[+-]\d{2}:\d{2})?",
        r#"["'][^"']*["']"#,
    ];
    for pattern in strip_patterns {
        if let Ok(re) = Regex::new(pattern) {
            out = re.replace_all(&out, "").to_string();
        }
    }
    if let Ok(re) = Regex::new(r"\s+") {
        out = re.replace_all(&out, " ").trim().to_string();
    } else {
        out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    if out.is_empty() {
        return String::new();
    }
    out.split_whitespace()
        .take(12)
        .collect::<Vec<_>>()
        .join("_")
}

fn predict_route_habit_id(intent_key: &str, task_text: &str) -> String {
    let fallback_intent = normalize_route_intent(task_text);
    let candidate = if intent_key.trim().is_empty() {
        fallback_intent
    } else {
        intent_key.to_ascii_lowercase()
    };
    let mut base = if let Ok(re) = Regex::new(r"[^a-z0-9_]+") {
        re.replace_all(&candidate, "_").to_string()
    } else {
        candidate
    };
    base = base.trim_matches('_').to_string();
    if base.len() > 48 {
        base.truncate(48);
    }
    if base.is_empty() {
        "habit".to_string()
    } else {
        base
    }
}

fn summarize_route_intent(task_text: &str) -> String {
    let parts = task_text
        .split_whitespace()
        .take(6)
        .map(|row| row.to_ascii_lowercase())
        .collect::<Vec<String>>();
    if parts.is_empty() {
        "task".to_string()
    } else {
        parts.join("_")
    }
}

pub fn evaluate_route_primitives(req: &RoutePrimitivesRequest) -> RoutePrimitivesResponse {
    let intent_key = normalize_route_intent(req.task_text.as_str());
    let trigger_a = req.repeats_14d >= 3 && req.tokens_est >= 500;
    let trigger_b = req.tokens_est >= 2000;
    let trigger_c = req.errors_30d >= 2;
    let mut which_met: Vec<String> = Vec::new();
    if trigger_a {
        which_met.push("A".to_string());
    }
    if trigger_b {
        which_met.push("B".to_string());
    }
    if trigger_c {
        which_met.push("C".to_string());
    }

    RoutePrimitivesResponse {
        ok: true,
        intent_key: intent_key.clone(),
        intent: summarize_route_intent(req.task_text.as_str()),
        predicted_habit_id: predict_route_habit_id(intent_key.as_str(), req.task_text.as_str()),
        trigger_a,
        trigger_b,
        trigger_c,
        any_trigger: trigger_a || trigger_b || trigger_c,
        which_met,
        thresholds: RouteThresholds {
            a: RouteThresholdA {
                repeats_14d_min: 3,
                tokens_min: 500,
                met: trigger_a,
            },
            b: RouteThresholdB {
                tokens_min: 2000,
                met: trigger_b,
            },
            c: RouteThresholdC {
                errors_30d_min: 2,
                met: trigger_c,
            },
        },
    }
}

pub fn evaluate_route_match(req: &RouteMatchRequest) -> RouteMatchResponse {
    let intent_key = normalize_token(req.intent_key.as_str(), 120);
    let skip_habit_id = normalize_token(req.skip_habit_id.as_str(), 120);
    if intent_key.is_empty() {
        return RouteMatchResponse {
            ok: true,
            matched_habit_id: None,
            match_strategy: "none".to_string(),
        };
    }

    let exact = req
        .habits
        .iter()
        .find(|habit| {
            let id = normalize_token(habit.id.as_str(), 120);
            !id.is_empty() && id == intent_key && id != skip_habit_id
        })
        .map(|habit| clean_text(habit.id.as_str(), 160));
    if let Some(matched_habit_id) = exact {
        return RouteMatchResponse {
            ok: true,
            matched_habit_id: Some(matched_habit_id),
            match_strategy: "exact".to_string(),
        };
    }

    let token_match = req
        .habits
        .iter()
        .find(|habit| {
            let id = normalize_token(habit.id.as_str(), 120);
            !id.is_empty() && id != skip_habit_id && intent_key.contains(id.as_str())
        })
        .map(|habit| clean_text(habit.id.as_str(), 160));
    if let Some(matched_habit_id) = token_match {
        return RouteMatchResponse {
            ok: true,
            matched_habit_id: Some(matched_habit_id),
            match_strategy: "token".to_string(),
        };
    }

    RouteMatchResponse {
        ok: true,
        matched_habit_id: None,
        match_strategy: "none".to_string(),
    }
}

pub fn evaluate_route_reflex_match(req: &RouteReflexMatchRequest) -> RouteReflexMatchResponse {
    let intent_key = normalize_token(req.intent_key.as_str(), 200);
    let task_text = clean_text(req.task_text.as_str(), 2000).to_ascii_lowercase();
    let intent_key_lower = intent_key.to_ascii_lowercase();

    for routine in &req.routines {
        if normalize_token(routine.status.as_str(), 32) != "enabled" {
            continue;
        }
        let id = normalize_token(routine.id.as_str(), 120);
        if id.is_empty() {
            continue;
        }
        if id == intent_key_lower || intent_key_lower.contains(id.as_str()) {
            return RouteReflexMatchResponse {
                ok: true,
                matched_reflex_id: Some(clean_text(routine.id.as_str(), 160)),
                match_strategy: "direct_id".to_string(),
            };
        }
    }

    for routine in &req.routines {
        if normalize_token(routine.status.as_str(), 32) != "enabled" {
            continue;
        }
        let tags = routine
            .tags
            .iter()
            .map(|tag| normalize_token(tag.as_str(), 120))
            .filter(|tag| !tag.is_empty())
            .collect::<Vec<String>>();
        if tags.is_empty() {
            continue;
        }
        if tags.iter().any(|tag| task_text.contains(tag.as_str())) {
            return RouteReflexMatchResponse {
                ok: true,
                matched_reflex_id: Some(clean_text(routine.id.as_str(), 160)),
                match_strategy: "tag".to_string(),
            };
        }
    }

    RouteReflexMatchResponse {
        ok: true,
        matched_reflex_id: None,
        match_strategy: "none".to_string(),
    }
}

pub fn evaluate_route_complexity(req: &RouteComplexityRequest) -> RouteComplexityResponse {
    if req.tokens_est >= 2500 {
        return RouteComplexityResponse {
            ok: true,
            complexity: "high".to_string(),
            reason: "tokens_est_high".to_string(),
        };
    }
    if req.tokens_est >= 800 {
        return RouteComplexityResponse {
            ok: true,
            complexity: "medium".to_string(),
            reason: "tokens_est_medium".to_string(),
        };
    }
    if clean_text(req.task_text.as_str(), 5000).chars().count() >= 240 {
        return RouteComplexityResponse {
            ok: true,
            complexity: "medium".to_string(),
            reason: "task_text_length".to_string(),
        };
    }
    if req.has_match || req.any_trigger {
        return RouteComplexityResponse {
            ok: true,
            complexity: "medium".to_string(),
            reason: if req.has_match {
                "has_match".to_string()
            } else {
                "any_trigger".to_string()
            },
        };
    }
    RouteComplexityResponse {
        ok: true,
        complexity: "low".to_string(),
        reason: "default_low".to_string(),
    }
}
