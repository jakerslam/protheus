// SRS: V13-AGENT-CAPABILITY-LADDER-001
use super::eval_synthetic_user_chat_harness_utils::{
    bool_at, clean_text, f64_at, ratio, str_opt, string_array_at, u64_at,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Default)]
struct DomainStats {
    total_turns: u64,
    passed_turns: u64,
    case_passes: BTreeMap<String, bool>,
    failure_count: u64,
    failure_reasons: BTreeMap<String, u64>,
    failed_turns: Vec<Value>,
}

pub(super) fn agent_work_success_report(rows: &[Value], thresholds: &Value) -> Value {
    let min_domain_success_rate = f64_at(
        thresholds,
        &["agent_work_success", "min_domain_success_rate"],
        f64_at(thresholds, &["min_pass_rate"], 1.0),
    )
    .clamp(0.0, 1.0);
    let required_domains = string_array_at(thresholds, &["agent_work_success", "required_domains"]);
    let mut stats_by_domain = BTreeMap::<String, DomainStats>::new();

    for row in rows {
        let domain = str_opt(row, &["capability_domain"])
            .unwrap_or("uncategorized")
            .to_string();
        let case_id = str_opt(row, &["case_id"]).unwrap_or("unknown_case");
        let turn_id = str_opt(row, &["turn_id"]).unwrap_or("unknown_turn");
        let passed = bool_at(row, &["pass"], false);
        let failures = row
            .get("failures")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(|raw| clean_text(raw, 240))
            .collect::<Vec<_>>();

        let stats = stats_by_domain.entry(domain).or_default();
        stats.total_turns = stats.total_turns.saturating_add(1);
        if passed {
            stats.passed_turns = stats.passed_turns.saturating_add(1);
        }
        stats
            .case_passes
            .entry(case_id.to_string())
            .and_modify(|case_ok| *case_ok = *case_ok && passed)
            .or_insert(passed);
        stats.failure_count = stats.failure_count.saturating_add(failures.len() as u64);
        for reason in failures.iter() {
            *stats.failure_reasons.entry(reason.clone()).or_insert(0) += 1;
        }
        if !passed {
            stats.failed_turns.push(json!({
                "case_id": case_id,
                "turn_id": turn_id,
                "failures": failures,
            }));
        }
    }

    for domain in required_domains.iter() {
        stats_by_domain.entry(domain.clone()).or_default();
    }

    let mut total_turns = 0_u64;
    let mut passed_turns = 0_u64;
    let mut passed_domains = 0_u64;
    let by_domain = stats_by_domain
        .into_iter()
        .map(|(domain, stats)| {
            total_turns = total_turns.saturating_add(stats.total_turns);
            passed_turns = passed_turns.saturating_add(stats.passed_turns);
            let total_cases = stats.case_passes.len() as u64;
            let passed_cases = stats.case_passes.values().filter(|passed| **passed).count() as u64;
            let turn_success_rate = ratio(stats.passed_turns, stats.total_turns);
            let case_success_rate = ratio(passed_cases, total_cases);
            let ok = stats.total_turns > 0
                && turn_success_rate >= min_domain_success_rate
                && case_success_rate >= min_domain_success_rate;
            if ok {
                passed_domains = passed_domains.saturating_add(1);
            }
            let failure_reasons = stats
                .failure_reasons
                .into_iter()
                .map(|(reason, count)| json!({"reason": reason, "count": count}))
                .collect::<Vec<_>>();
            json!({
                "domain": domain,
                "ok": ok,
                "total_cases": total_cases,
                "passed_cases": passed_cases,
                "case_success_rate": case_success_rate,
                "total_turns": stats.total_turns,
                "passed_turns": stats.passed_turns,
                "turn_success_rate": turn_success_rate,
                "failure_count": stats.failure_count,
                "failure_reasons": failure_reasons,
                "failed_turns": stats.failed_turns,
            })
        })
        .collect::<Vec<_>>();
    let total_domains = by_domain.len() as u64;
    let overall_ok = total_domains == 0 || passed_domains == total_domains;

    json!({
        "metric": "agent_work_success",
        "definition": "End-to-end user work success grouped by capability domain; a domain passes only when its turns satisfy the visible-output rubric plus required tool/evidence/synthesis checks.",
        "thresholds": {
            "min_domain_success_rate": min_domain_success_rate,
            "required_domains": required_domains,
        },
        "overall": {
            "ok": overall_ok,
            "total_domains": total_domains,
            "passed_domains": passed_domains,
            "total_turns": total_turns,
            "passed_turns": passed_turns,
            "turn_success_rate": ratio(passed_turns, total_turns),
        },
        "by_domain": by_domain,
    })
}

pub(super) fn markdown_report(report: &Value) -> String {
    let mut output = format!(
        "# Synthetic User Chat Harness\n\n- generated_at: {}\n- ok: {}\n- mode: {}\n- cases: {}\n- total_turns: {}\n- pass_rate: {:.3}\n- failure_count: {}\n- route_stage_deltas: {}\n",
        str_opt(report, &["generated_at"]).unwrap_or(""),
        bool_at(report, &["ok"], false),
        str_opt(report, &["mode"]).unwrap_or("unknown"),
        u64_at(report, &["summary", "cases"], 0),
        u64_at(report, &["summary", "total_turns"], 0),
        f64_at(report, &["summary", "pass_rate"], 0.0),
        u64_at(report, &["summary", "failure_count"], 0),
        report
            .get("route_stage_deltas")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0),
    );

    if let Some(domains) = report
        .pointer("/summary/agent_work_success/by_domain")
        .and_then(Value::as_array)
    {
        output.push_str("\n## Agent Work Success By Category\n\n");
        for domain in domains {
            output.push_str(&format!(
                "- {}: {:.3} turns, {:.3} cases, ok={}\n",
                str_opt(domain, &["domain"]).unwrap_or("unknown"),
                f64_at(domain, &["turn_success_rate"], 0.0),
                f64_at(domain, &["case_success_rate"], 0.0),
                bool_at(domain, &["ok"], false),
            ));
        }
    }
    output
}
