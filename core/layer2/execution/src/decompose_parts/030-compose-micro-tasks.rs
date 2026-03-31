pub fn compose_micro_tasks(req: &ComposeRequest) -> Vec<Value> {
    let run_id = if req.run_id.trim().is_empty() {
        format!(
            "tdp_compose_{}",
            sha16(format!("{}|{}", req.goal_id, req.goal_text).as_str())
        )
    } else {
        req.run_id.trim().to_string()
    };
    let max_groups = req.policy.max_groups.max(1);
    let default_lane = {
        let lane = normalize_token(req.policy.default_lane.as_str(), 80);
        if lane.is_empty() {
            "autonomous_micro_agent".to_string()
        } else {
            lane
        }
    };

    req.tasks
        .iter()
        .enumerate()
        .filter_map(|(i, base)| {
            let task_text = clean_text(base.task_text.as_str(), 1000);
            if task_text.is_empty() {
                return None;
            }

            let micro_task_id = {
                let normalized = normalize_token(base.micro_task_id.as_str(), 120);
                if normalized.is_empty() {
                    format!(
                        "mt_{}",
                        sha16(format!("{}|{}|{}", run_id, i, task_text).as_str())
                    )
                } else {
                    normalized
                }
            };
            let profile_id = {
                let normalized = normalize_token(base.profile_id.as_str(), 120);
                if normalized.is_empty() {
                    format!(
                        "task_micro_{}",
                        sha16(format!("{}|{}", req.goal_id, micro_task_id).as_str())
                    )
                } else {
                    normalized
                }
            };
            let capability = normalize_capability(&base.capability);
            let suggested_lane = {
                let lane = normalize_token(base.suggested_lane.as_str(), 80);
                if lane.is_empty() {
                    default_lane.clone()
                } else {
                    lane
                }
            };
            let minutes = base
                .estimated_minutes
                .clamp(req.policy.min_minutes.max(1), req.policy.max_minutes.max(1));
            let success_criteria = if base.success_criteria.is_empty() {
                success_criteria(task_text.as_str())
            } else {
                base.success_criteria
                    .iter()
                    .map(|row| clean_text(row.as_str(), 220))
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<String>>()
            };
            let parallel_priority = if base.parallel_priority.is_finite() {
                (base.parallel_priority * 10_000f64).round() / 10_000f64
            } else {
                (1f64 / minutes.max(1) as f64 * 10_000f64).round() / 10_000f64
            };
            let title = {
                let normalized = clean_text(base.title.as_str(), 220);
                if normalized.is_empty() {
                    title_for_task(task_text.as_str())
                } else {
                    normalized
                }
            };
            let objective_id = req.objective_id.clone();
            let creator_id = req.creator_id.clone();
            Some(json!({
                "micro_task_id": micro_task_id,
                "goal_id": req.goal_id,
                "objective_id": objective_id,
                "parent_id": base.parent_id,
                "depth": base.depth,
                "index": base.index,
                "title": title,
                "task_text": task_text,
                "estimated_minutes": minutes,
                "success_criteria": success_criteria,
                "required_capability": capability.capability_id,
                "profile_id": profile_id,
                "capability": capability,
                "route": {
                    "lane": suggested_lane,
                    "parallel_group": base.parallel_group.min(max_groups.saturating_sub(1)),
                    "parallel_priority": parallel_priority,
                    "blocked": false,
                    "requires_manual_review": false
                },
                "profile": {
                    "schema_id": "task_micro_profile",
                    "schema_version": "1.0",
                    "profile_id": profile_id,
                    "source": {
                        "source_type": capability.source_type,
                        "capability_id": capability.capability_id,
                        "objective_id": objective_id,
                        "origin_lane": "task_decomposition_primitive"
                    },
                    "intent": {
                        "id": "micro_task_execute",
                        "description": task_text,
                        "success_criteria": success_criteria
                    },
                    "execution": {
                        "adapter_kind": capability.adapter_kind,
                        "estimated_minutes": minutes,
                        "dry_run_default": true
                    },
                    "routing": {
                        "preferred_lane": suggested_lane,
                        "requires_manual_review": false
                    },
                    "provenance": {
                        "confidence": 0.92,
                        "evidence": {
                            "decomposition_depth": base.depth,
                            "heroic_echo_decision": "allow",
                            "constitution_decision": "ALLOW"
                        }
                    },
                    "governance": {
                        "heroic_echo": {
                            "classification": "normal",
                            "decision": "allow",
                            "reason_codes": []
                        },
                        "constitution": {
                            "decision": "ALLOW",
                            "risk": "low",
                            "reasons": []
                        }
                    },
                    "attribution": {
                        "source_goal_id": req.goal_id,
                        "source_goal_hash": sha16(req.goal_text.as_str()),
                        "creator_id": creator_id,
                        "influence_score": 1,
                        "lineage": [req.goal_id, micro_task_id]
                    },
                    "duality": {
                        "enabled": false,
                        "score_trit": 0,
                        "score_label": "unknown",
                        "zero_point_harmony_potential": 0,
                        "recommended_adjustment": Value::Null,
                        "indicator": {
                            "subtle_hint": "duality_signal_pending"
                        }
                    }
                },
                "governance": {
                    "blocked": false,
                    "block_reasons": [],
                    "heroic_echo": {
                        "classification": "normal",
                        "decision": "allow",
                        "blocked": false,
                        "reason_codes": []
                    },
                    "constitution": {
                        "decision": "ALLOW",
                        "risk": "low",
                        "reasons": []
                    }
                },
                "duality": {
                    "enabled": false,
                    "score_trit": 0,
                    "score_label": "unknown",
                    "zero_point_harmony_potential": 0,
                    "recommended_adjustment": Value::Null,
                    "indicator": {
                        "subtle_hint": "duality_signal_pending"
                    }
                }
            }))
        })
        .collect()
}

pub fn decompose_goal(req: &DecomposeRequest) -> Vec<BaseTask> {
    let run_id = if req.run_id.trim().is_empty() {
        format!(
            "tdp_{}",
            sha16(format!("{}|{}", req.goal_id, req.goal_text).as_str())
        )
    } else {
        req.run_id.trim().to_string()
    };
    let max_items = req.policy.max_micro_tasks.max(1);
    let segments = dedupe_segments(
        recursive_decompose(req.goal_text.as_str(), 0, &req.policy, None),
        max_items,
    );

    let mut tasks: Vec<BaseTask> = Vec::new();
    for (i, seg) in segments.into_iter().enumerate() {
        let task_text = clean_text(seg.text.as_str(), 1000);
        if task_text.is_empty() {
            continue;
        }
        let micro_task_id = format!(
            "mt_{}",
            sha16(format!("{}|{}|{}", run_id, i, task_text).as_str())
        );
        let capability = infer_capability(task_text.as_str());
        let minutes = estimate_minutes(task_text.as_str(), &req.policy);
        let profile_id = format!(
            "task_micro_{}",
            sha16(format!("{}|{}", req.goal_id, micro_task_id).as_str())
        );
        let lane = lane_for_task(task_text.as_str(), &req.policy);
        tasks.push(BaseTask {
            micro_task_id,
            goal_id: req.goal_id.clone(),
            objective_id: req.objective_id.clone(),
            parent_id: seg.parent_id,
            depth: seg.depth,
            index: i,
            title: title_for_task(task_text.as_str()),
            task_text: task_text.clone(),
            estimated_minutes: minutes,
            success_criteria: success_criteria(task_text.as_str()),
            required_capability: capability.capability_id.clone(),
            profile_id,
            capability,
            suggested_lane: lane,
            parallel_group: i % req.policy.max_groups.max(1),
            parallel_priority: 1f64 / (minutes.max(1) as f64),
        });
    }

    let human_count = tasks
        .iter()
        .filter(|task| task.suggested_lane == req.policy.storm_lane)
        .count();
    let human_share = if tasks.is_empty() {
        0f64
    } else {
        human_count as f64 / tasks.len() as f64
    };
    if tasks.len() > 2 && human_share < req.policy.min_storm_share {
        if let Some(first) = tasks.first_mut() {
            first.suggested_lane = req.policy.storm_lane.clone();
        }
    }

    tasks
}

pub fn decompose_goal_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<DecomposeRequest>(payload)
        .map_err(|err| format!("decompose_payload_parse_failed:{}", err))?;
    let resp = DecomposeResponse {
        ok: true,
        tasks: decompose_goal(&req),
    };
    serde_json::to_string(&resp)
        .map_err(|err| format!("decompose_payload_serialize_failed:{}", err))
}

pub fn compose_micro_tasks_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<ComposeRequest>(payload)
        .map_err(|err| format!("compose_payload_parse_failed:{}", err))?;
    let resp = ComposeResponse {
        ok: true,
        tasks: compose_micro_tasks(&req),
    };
    serde_json::to_string(&resp).map_err(|err| format!("compose_payload_serialize_failed:{}", err))
}

pub fn summarize_tasks(tasks: &[Value], shadow_only: bool, apply_executed: bool) -> Value {
    let mut lane_breakdown: BTreeMap<String, u64> = BTreeMap::new();
    let mut ready = 0u64;
    let mut blocked = 0u64;
    let mut manual_review = 0u64;
    let mut autonomous_lane = 0u64;
    let mut storm_lane = 0u64;

    for task in tasks {
        let route = task.get("route").and_then(|v| v.as_object());
        let governance = task.get("governance").and_then(|v| v.as_object());

        let lane = route
            .and_then(|row| row.get("lane"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let is_blocked = governance
            .and_then(|row| row.get("blocked"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let is_manual = route
            .and_then(|row| row.get("requires_manual_review"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        *lane_breakdown.entry(lane.clone()).or_insert(0) += 1;
        if is_blocked {
            blocked += 1;
        } else {
            ready += 1;
        }
        if is_manual {
            manual_review += 1;
        }
        if lane == "autonomous_micro_agent" {
            autonomous_lane += 1;
        }
        if lane == "storm_human_lane" {
            storm_lane += 1;
        }
    }

    json!({
        "total_micro_tasks": tasks.len(),
        "ready": ready,
        "blocked": blocked,
        "manual_review": manual_review,
        "autonomous_lane": autonomous_lane,
        "storm_lane": storm_lane,
        "lane_breakdown": lane_breakdown,
        "shadow_only": shadow_only,
        "apply_executed": apply_executed
    })
}

pub fn summarize_tasks_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<TaskSummaryRequest>(payload)
        .map_err(|err| format!("task_summary_payload_parse_failed:{}", err))?;
    let resp = TaskSummaryResponse {
        ok: true,
        summary: summarize_tasks(&req.tasks, req.shadow_only, req.apply_executed),
    };
    serde_json::to_string(&resp)
        .map_err(|err| format!("task_summary_payload_serialize_failed:{}", err))
}

pub fn summarize_dispatch(rows: &[Value], enabled: bool) -> Value {
    let mut queued = 0u64;
    let mut executed = 0u64;
    let mut failed = 0u64;
    let mut blocked = 0u64;

    for row in rows {
        let status = row
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        match status {
            "queued" => queued += 1,
            "executed" => executed += 1,
            "failed" => failed += 1,
            "blocked" => blocked += 1,
            _ => {}
        }
    }

    json!({
        "enabled": enabled,
        "total": rows.len(),
        "queued": queued,
        "executed": executed,
        "failed": failed,
        "blocked": blocked
    })
}

pub fn summarize_dispatch_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<DispatchSummaryRequest>(payload)
        .map_err(|err| format!("dispatch_summary_payload_parse_failed:{}", err))?;
    let resp = DispatchSummaryResponse {
        ok: true,
        summary: summarize_dispatch(&req.rows, req.enabled),
    };
    serde_json::to_string(&resp)
        .map_err(|err| format!("dispatch_summary_payload_serialize_failed:{}", err))
}

fn duality_indicator_for_task(task: &Value) -> Value {
    task.get("duality")
        .and_then(|row| row.get("indicator"))
        .cloned()
        .unwrap_or_else(|| json!({ "subtle_hint": "duality_signal_absent" }))
}

fn attribution_for_task(task: &Value) -> Value {
    task.get("profile")
        .and_then(|row| row.get("attribution"))
        .cloned()
        .unwrap_or_else(|| json!({}))
}
