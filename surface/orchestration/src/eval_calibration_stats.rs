use infring_orchestration_surface_v1::eval::{EvalJudgeHumanComparableRow, EvalVerdict};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
pub struct EvalCalibrationStatsPolicy {
    pub minimum_samples: u64,
    pub max_ci_half_width: f64,
    pub sensitivity_min: f64,
    pub specificity_min: f64,
}

#[derive(Debug, Clone, Copy)]
struct RateStats {
    success: u64,
    total: u64,
    value: f64,
    low: f64,
    high: f64,
    half_width: f64,
}

pub fn judge_calibration_stats(
    rows: &[EvalJudgeHumanComparableRow],
    policy: EvalCalibrationStatsPolicy,
) -> Value {
    let mut tp = 0_u64;
    let mut tn = 0_u64;
    let mut fp = 0_u64;
    let mut fn_count = 0_u64;
    let mut exact_matches = 0_u64;

    for row in rows {
        let human_positive = is_positive(&row.human_verdict);
        let judge_positive = is_positive(&row.judge_verdict);
        match (human_positive, judge_positive) {
            (true, true) => tp = tp.saturating_add(1),
            (false, false) => tn = tn.saturating_add(1),
            (false, true) => fp = fp.saturating_add(1),
            (true, false) => fn_count = fn_count.saturating_add(1),
        }
        if row.human_verdict == row.judge_verdict {
            exact_matches = exact_matches.saturating_add(1);
        }
    }

    let total = rows.len() as u64;
    let sensitivity = rate_stats(tp, tp.saturating_add(fn_count));
    let specificity = rate_stats(tn, tn.saturating_add(fp));
    let precision = rate_stats(tp, tp.saturating_add(fp));
    let recall = sensitivity;
    let accuracy = rate_stats(tp.saturating_add(tn), total);
    let agreement = rate_stats(exact_matches, total);
    let widest_half_width = [
        sensitivity.half_width,
        specificity.half_width,
        precision.half_width,
        recall.half_width,
        accuracy.half_width,
        agreement.half_width,
    ]
    .into_iter()
    .fold(0.0_f64, f64::max);

    let samples_ready = total >= policy.minimum_samples;
    let intervals_ready = total > 0 && widest_half_width <= policy.max_ci_half_width;
    let sensitivity_ready = sensitivity.total > 0 && sensitivity.value >= policy.sensitivity_min;
    let specificity_ready = specificity.total > 0 && specificity.value >= policy.specificity_min;
    let promotion_ready =
        samples_ready && intervals_ready && sensitivity_ready && specificity_ready;

    json!({
        "type": "eval_judge_calibration_statistics",
        "schema_version": 1,
        "sample_count": total,
        "confusion": {
            "true_positive": tp,
            "true_negative": tn,
            "false_positive": fp,
            "false_negative": fn_count,
            "positive_definition": "human_or_judge_verdict_in_correct_or_partial",
            "negative_definition": "human_or_judge_verdict_in_incorrect"
        },
        "rates": {
            "sensitivity": sensitivity.value,
            "specificity": specificity.value,
            "precision": precision.value,
            "recall": recall.value,
            "accuracy": accuracy.value,
            "agreement": agreement.value
        },
        "confidence_intervals": {
            "method": "wilson_score",
            "confidence": 0.95,
            "widest_wilson_95_half_width": widest_half_width,
            "sensitivity": rate_json(sensitivity),
            "specificity": rate_json(specificity),
            "precision": rate_json(precision),
            "recall": rate_json(recall),
            "accuracy": rate_json(accuracy),
            "agreement": rate_json(agreement)
        },
        "adaptive_sample_policy": {
            "minimum_samples": policy.minimum_samples,
            "samples_ready": samples_ready,
            "additional_samples_needed": policy.minimum_samples.saturating_sub(total),
            "max_ci_half_width": policy.max_ci_half_width,
            "intervals_ready": intervals_ready,
            "sensitivity_min": policy.sensitivity_min,
            "sensitivity_ready": sensitivity_ready,
            "specificity_min": policy.specificity_min,
            "specificity_ready": specificity_ready,
            "promotion_ready": promotion_ready,
            "promotion_blocked": !promotion_ready,
            "block_reasons": block_reasons(
                samples_ready,
                intervals_ready,
                sensitivity_ready,
                specificity_ready,
            )
        }
    })
}

fn is_positive(verdict: &EvalVerdict) -> bool {
    matches!(verdict, EvalVerdict::Correct | EvalVerdict::Partial)
}

fn rate_stats(success: u64, total: u64) -> RateStats {
    let value = ratio(success, total);
    let (low, high) = wilson_interval(success, total);
    RateStats {
        success,
        total,
        value,
        low,
        high,
        half_width: ((high - low) / 2.0).max(0.0),
    }
}

fn rate_json(stats: RateStats) -> Value {
    json!({
        "success": stats.success,
        "total": stats.total,
        "value": stats.value,
        "wilson_95_low": stats.low,
        "wilson_95_high": stats.high,
        "wilson_95_half_width": stats.half_width
    })
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn wilson_interval(success: u64, total: u64) -> (f64, f64) {
    if total == 0 {
        return (0.0, 1.0);
    }
    let z = 1.959_963_984_540_054_f64;
    let n = total as f64;
    let p = success as f64 / n;
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denom;
    let margin = (z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt()) / denom;
    (
        (center - margin).clamp(0.0, 1.0),
        (center + margin).clamp(0.0, 1.0),
    )
}

fn block_reasons(
    samples_ready: bool,
    intervals_ready: bool,
    sensitivity_ready: bool,
    specificity_ready: bool,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if !samples_ready {
        reasons.push("insufficient_samples");
    }
    if !intervals_ready {
        reasons.push("confidence_interval_too_wide");
    }
    if !sensitivity_ready {
        reasons.push("sensitivity_below_minimum");
    }
    if !specificity_ready {
        reasons.push("specificity_below_minimum");
    }
    reasons
}
