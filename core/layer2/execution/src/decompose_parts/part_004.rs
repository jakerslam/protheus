pub fn evaluate_route(req: &RouteEvaluateRequest) -> RouteEvaluateResponse {
    let primitives = evaluate_route_primitives(&RoutePrimitivesRequest {
        task_text: req.task_text.clone(),
        tokens_est: req.tokens_est,
        repeats_14d: req.repeats_14d,
        errors_30d: req.errors_30d,
    });
    let habit_match = evaluate_route_match(&RouteMatchRequest {
        intent_key: primitives.intent_key.clone(),
        skip_habit_id: req.skip_habit_id.clone(),
        habits: req.habits.clone(),
    });
    let reflex_match = evaluate_route_reflex_match(&RouteReflexMatchRequest {
        intent_key: primitives.intent_key.clone(),
        task_text: req.task_text.clone(),
        routines: req.reflex_routines.clone(),
    });
    let complexity = evaluate_route_complexity(&RouteComplexityRequest {
        task_text: req.task_text.clone(),
        tokens_est: req.tokens_est,
        has_match: habit_match.matched_habit_id.is_some(),
        any_trigger: primitives.any_trigger,
    });

    RouteEvaluateResponse {
        ok: true,
        intent_key: primitives.intent_key,
        intent: primitives.intent,
        predicted_habit_id: primitives.predicted_habit_id,
        trigger_a: primitives.trigger_a,
        trigger_b: primitives.trigger_b,
        trigger_c: primitives.trigger_c,
        any_trigger: primitives.any_trigger,
        which_met: primitives.which_met,
        thresholds: primitives.thresholds,
        matched_habit_id: habit_match.matched_habit_id,
        matched_habit_strategy: habit_match.match_strategy,
        matched_reflex_id: reflex_match.matched_reflex_id,
        matched_reflex_strategy: reflex_match.match_strategy,
        complexity: complexity.complexity,
        complexity_reason: complexity.reason,
    }
}

fn normalize_route_state(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "active" => "active".to_string(),
        "candidate" => "candidate".to_string(),
        _ => "other".to_string(),
    }
}
pub fn evaluate_route_decision(req: &RouteDecisionRequest) -> RouteDecisionResponse {
    let matched_reflex_id = req.matched_reflex_id.trim();
    if req.reflex_eligible && !matched_reflex_id.is_empty() {
        return RouteDecisionResponse {
            ok: true,
            decision: "RUN_REFLEX".to_string(),
            reason_code: "reflex_match".to_string(),
            suggested_habit_id: None,
            auto_habit_flow: false,
        };
    }

    let matched_habit_id = req.matched_habit_id.trim();
    if !matched_habit_id.is_empty() {
        let state = normalize_route_state(req.matched_habit_state.as_str());
        if state == "active" || state == "candidate" {
            if req.has_required_inputs || req.required_input_count > 0 {
                return RouteDecisionResponse {
                    ok: true,
                    decision: "MANUAL".to_string(),
                    reason_code: "required_inputs".to_string(),
                    suggested_habit_id: Some(matched_habit_id.to_string()),
                    auto_habit_flow: false,
                };
            }
            if !req.trusted_entrypoint {
                return RouteDecisionResponse {
                    ok: true,
                    decision: "MANUAL".to_string(),
                    reason_code: "untrusted_entrypoint".to_string(),
                    suggested_habit_id: Some(matched_habit_id.to_string()),
                    auto_habit_flow: false,
                };
            }
            return RouteDecisionResponse {
                ok: true,
                decision: if state == "active" {
                    "RUN_HABIT".to_string()
                } else {
                    "RUN_CANDIDATE_FOR_VERIFICATION".to_string()
                },
                reason_code: if state == "active" {
                    "active_match".to_string()
                } else {
                    "candidate_match".to_string()
                },
                suggested_habit_id: Some(matched_habit_id.to_string()),
                auto_habit_flow: false,
            };
        }
        return RouteDecisionResponse {
            ok: true,
            decision: "MANUAL".to_string(),
            reason_code: "matched_state_not_runnable".to_string(),
            suggested_habit_id: Some(matched_habit_id.to_string()),
            auto_habit_flow: false,
        };
    }

    if req.any_trigger {
        let predicted = req.predicted_habit_id.trim();
        return RouteDecisionResponse {
            ok: true,
            decision: "RUN_CANDIDATE_FOR_VERIFICATION".to_string(),
            reason_code: "trigger_autocrystallize".to_string(),
            suggested_habit_id: if predicted.is_empty() {
                None
            } else {
                Some(predicted.to_string())
            },
            auto_habit_flow: true,
        };
    }

    RouteDecisionResponse {
        ok: true,
        decision: "MANUAL".to_string(),
        reason_code: "no_match_no_trigger".to_string(),
        suggested_habit_id: None,
        auto_habit_flow: false,
    }
}

pub fn evaluate_route_habit_readiness(
    req: &RouteHabitReadinessRequest,
) -> RouteHabitReadinessResponse {
    let state = normalize_route_state(req.habit_state.as_str());
    let required_inputs = req
        .required_inputs
        .iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect::<Vec<String>>();
    let entrypoint = req.entrypoint_resolved.trim();
    let trusted_entrypoint = if entrypoint.is_empty() {
        false
    } else {
        req.trusted_entrypoints
            .iter()
            .any(|candidate| candidate.trim() == entrypoint)
    };
    let runnable_state = state == "active" || state == "candidate";
    let runnable = runnable_state && required_inputs.is_empty() && trusted_entrypoint;
    let reason_code = if !runnable_state {
        "matched_state_not_runnable"
    } else if !required_inputs.is_empty() {
        "required_inputs"
    } else if !trusted_entrypoint {
        "untrusted_entrypoint"
    } else if state == "active" {
        "runnable_active"
    } else {
        "runnable_candidate"
    };

    RouteHabitReadinessResponse {
        ok: true,
        state,
        required_inputs,
        trusted_entrypoint,
        runnable,
        reason_code: reason_code.to_string(),
    }
}

pub fn evaluate_route_primitives_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<RoutePrimitivesRequest>(payload)
        .map_err(|err| format!("route_primitives_payload_parse_failed:{}", err))?;
    let resp = evaluate_route_primitives(&req);
    serde_json::to_string(&resp)
        .map_err(|err| format!("route_primitives_payload_serialize_failed:{}", err))
}

pub fn evaluate_route_match_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<RouteMatchRequest>(payload)
        .map_err(|err| format!("route_match_payload_parse_failed:{}", err))?;
    let resp = evaluate_route_match(&req);
    serde_json::to_string(&resp)
        .map_err(|err| format!("route_match_payload_serialize_failed:{}", err))
}

pub fn evaluate_route_reflex_match_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<RouteReflexMatchRequest>(payload)
        .map_err(|err| format!("route_reflex_match_payload_parse_failed:{}", err))?;
    let resp = evaluate_route_reflex_match(&req);
    serde_json::to_string(&resp)
        .map_err(|err| format!("route_reflex_match_payload_serialize_failed:{}", err))
}

pub fn evaluate_route_complexity_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<RouteComplexityRequest>(payload)
        .map_err(|err| format!("route_complexity_payload_parse_failed:{}", err))?;
    let resp = evaluate_route_complexity(&req);
    serde_json::to_string(&resp)
        .map_err(|err| format!("route_complexity_payload_serialize_failed:{}", err))
}

pub fn evaluate_route_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<RouteEvaluateRequest>(payload)
        .map_err(|err| format!("route_evaluate_payload_parse_failed:{}", err))?;
    let resp = evaluate_route(&req);
    serde_json::to_string(&resp)
        .map_err(|err| format!("route_evaluate_payload_serialize_failed:{}", err))
}

pub fn evaluate_route_decision_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<RouteDecisionRequest>(payload)
        .map_err(|err| format!("route_decision_payload_parse_failed:{}", err))?;
    let resp = evaluate_route_decision(&req);
    serde_json::to_string(&resp)
        .map_err(|err| format!("route_decision_payload_serialize_failed:{}", err))
}

pub fn evaluate_route_habit_readiness_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<RouteHabitReadinessRequest>(payload)
        .map_err(|err| format!("route_habit_readiness_payload_parse_failed:{}", err))?;
    let resp = evaluate_route_habit_readiness(&req);
    serde_json::to_string(&resp)
        .map_err(|err| format!("route_habit_readiness_payload_serialize_failed:{}", err))
}

fn is_trust_registry_modification(task_lower: &str) -> bool {
    let trust_targets = [
        "trust_registry",
        "trust registry",
        "trust_add",
        "trust_remove",
        "tests/tooling/scripts/memory/trust_add.ts",
        "client/memory/trust/registry.json",
        "registry.json",
        "registry hashes",
        "trust registry hashes",
    ];
    let mutation_verbs = [
        "edit", "modify", "update", "patch", "delete", "remove", "tamper", "change",
    ];
    contains_any(task_lower, &trust_targets) && contains_any(task_lower, &mutation_verbs)
}

fn set_manual(decision: &mut String) {
    if decision != "DENY" {
        *decision = "MANUAL".to_string();
    }
}

pub fn evaluate_directive_gate(task_text: &str) -> DirectiveGateResponse {
    let task = task_text.trim();
    if task.is_empty() {
        return DirectiveGateResponse {
            ok: true,
            decision: "DENY".to_string(),
            risk: "high".to_string(),
            reasons: vec!["Task must be a non-empty string".to_string()],
        };
    }

    let mut reasons: Vec<String> = Vec::new();
    let mut decision = "ALLOW".to_string();
    let mut risk = "low".to_string();
    let task_lower = task.to_ascii_lowercase();

    let deny_patterns = [
        (
            "bypass gate",
            r"(?i)\b(bypass.*gate|disable.*gate|skip.*gate)\b",
        ),
        (
            "disable log",
            r"(?i)\b(disable.*log|stop.*log|suppress.*event|remove.*audit)\b",
        ),
        (
            "tamper trust",
            r"(?i)\b(tamper|modify|edit|delete)\b.*\b(trust|hash|registry)\b",
        ),
        (
            "modify gate",
            r"(?i)\b(modify|edit|delete)\b.*\b(directive_gate)\b",
        ),
    ];
    for (label, pattern) in deny_patterns {
        if Regex::new(pattern)
            .ok()
            .map(|regex| regex.is_match(task))
            .unwrap_or(false)
        {
            reasons.push(format!("T0 violation: {} detected", label));
            decision = "DENY".to_string();
            risk = "high".to_string();
        }
    }
    if is_trust_registry_modification(&task_lower) {
        reasons.push("T0 violation: trust registry modification detected".to_string());
        decision = "DENY".to_string();
        risk = "high".to_string();
    }

    let high_risk_patterns = [
        (
            "High-risk: process execution detected",
            "high",
            r"(?i)\b(child[_\s]?process|exec|execsync|spawn|fork|execfile)\b",
        ),
        (
            "High-risk: shell execution detected",
            "high",
            r"(?i)\b(shell|bash|sh\s|cmd\.exe|powershell)\b",
        ),
        (
            "High-risk: credentials/token access detected",
            "high",
            r"(?i)\.openclaw[\/\\]credentials|\/credentials[\/\\]|token|api[_-]?key|secret|password",
        ),
        (
            "High-risk: network/API call detected",
            "medium",
            r"(?i)\b(http|https|fetch|axios|request|curl|wget|net\.|tls\.|socket)\b",
        ),
        (
            "High-risk: git remote operation detected",
            "high",
            r"(?i)\b(git\s+(push|force|reset|rebase|merge)|push\s+to|push\s+--|origin|publish|deploy)\b",
        ),
        (
            "High-risk: cron/system config modification detected",
            "high",
            r"(?i)\b(cron|crontab|systemd|service|daemon)\b",
        ),
        (
            "High-risk: revenue/financial action detected",
            "high",
            r"(?i)\b(payment|billing|subscription|charge|refund|account.*money|revenue)\b",
        ),
        (
            "High-risk: governance/security tooling modification detected",
            "high",
            r"(?i)\b(trust[_-]?|verify[_-]?hash|tamper|bypass|disable.*log|registry.*hash)\b",
        ),
        (
            "High-risk: governance/security tooling modification detected",
            "high",
            r"(?i)\b(trust_add|trust_remove|trust_registry|registry\.json)\b",
        ),
    ];
    for (message, severity, pattern) in high_risk_patterns {
        if Regex::new(pattern)
            .ok()
            .map(|regex| regex.is_match(task))
            .unwrap_or(false)
        {
            reasons.push(message.to_string());
            risk = severity.to_string();
            set_manual(&mut decision);
        }
    }

    let path_regex = Regex::new(r"[/~][a-zA-Z0-9_/.\-]+").ok();
    if let Some(path_regex) = path_regex {
        for path_match in path_regex.find_iter(task) {
            let found = path_match.as_str().to_ascii_lowercase();
            if contains_any(&found, &["credentials", "secret", "token"]) {
                reasons.push(format!("Path validation: sensitive path \"{}\"", found));
                risk = "high".to_string();
                set_manual(&mut decision);
            }
        }
    }

    if reasons.is_empty() {
        reasons.push("No high-risk patterns detected; standard routing applies".to_string());
    }
    DirectiveGateResponse {
        ok: true,
        decision,
        risk,
        reasons,
    }
}

pub fn evaluate_directive_gate_json(payload: &str) -> Result<String, String> {
    let req = serde_json::from_str::<DirectiveGateRequest>(payload)
        .map_err(|err| format!("directive_gate_payload_parse_failed:{}", err))?;
    let resp = evaluate_directive_gate(req.task_text.as_str());
    serde_json::to_string(&resp)
        .map_err(|err| format!("directive_gate_payload_serialize_failed:{}", err))
}
