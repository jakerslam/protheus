fn dashboard_response_gate_score_from_flags(
    final_response_contract_ok: bool,
    answer_contract_ok: bool,
    llm_reliability_not_low: bool,
    watchdog_triggered: bool,
) -> f64 {
    let mut score = 1.0_f64;
    if !final_response_contract_ok {
        score -= 0.35;
    }
    if !answer_contract_ok {
        score -= 0.35;
    }
    if !llm_reliability_not_low {
        score -= 0.20;
    }
    if watchdog_triggered {
        score -= 0.10;
    }
    (score.clamp(0.0, 1.0) * 10_000.0).round() / 10_000.0
}

fn dashboard_response_gate_severity_from_state(ready: bool, score: f64) -> &'static str {
    if ready {
        "ready"
    } else if score >= 0.6 {
        "degraded"
    } else {
        "blocked"
    }
}

fn dashboard_response_gate_score_band_from_state(ready: bool, score: f64) -> &'static str {
    if ready {
        "ready"
    } else if score >= 0.75 {
        "strong"
    } else if score >= 0.5 {
        "watch"
    } else if score >= 0.25 {
        "weak"
    } else {
        "critical"
    }
}

fn dashboard_response_gate_blockers_from_flags(
    final_response_contract_ok: bool,
    answer_contract_ok: bool,
    llm_reliability_not_low: bool,
    watchdog_triggered: bool,
) -> Vec<String> {
    [
        (!final_response_contract_ok, "final_response_contract"),
        (!answer_contract_ok, "answer_contract"),
        (!llm_reliability_not_low, "llm_reliability"),
        (watchdog_triggered, "watchdog"),
    ]
    .iter()
    .filter_map(|(failed, label)| if *failed { Some((*label).to_string()) } else { None })
    .collect::<Vec<_>>()
}
