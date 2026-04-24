// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    EvalCalibrationSnapshot, EvalQualityGateHistory, EvalQualityGatePolicy, EvalQualityGateState,
    EvalQualitySignalMode, EvalQualitySignalSnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalVerdict {
    Correct,
    Partial,
    Incorrect,
}

impl EvalVerdict {
    pub fn from_any(raw: &str) -> Option<Self> {
        let value = raw.trim().to_lowercase();
        match value.as_str() {
            "correct" | "pass" | "accurate" => Some(Self::Correct),
            "partial" | "mixed" | "partially_correct" => Some(Self::Partial),
            "incorrect" | "fail" | "wrong" | "inaccurate" | "false" => Some(Self::Incorrect),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalJudgeHumanComparableRow {
    pub ts: Option<String>,
    pub issue_id: Option<String>,
    pub human_verdict: EvalVerdict,
    pub judge_verdict: EvalVerdict,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalJudgeHumanAgreementPolicy {
    pub minimum_samples: u64,
    pub agreement_min: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalJudgeHumanAgreementSummary {
    pub comparable_samples: u64,
    pub minimum_samples: u64,
    pub agreement_rate: f64,
    pub agreement_min: f64,
    pub calibration_ready: bool,
    pub status: String,
    pub pair_counts: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalJudgeHumanAgreementResult {
    pub ok: bool,
    pub summary: EvalJudgeHumanAgreementSummary,
}

pub fn evaluate_quality_gate(
    signals: &EvalQualitySignalSnapshot,
    calibration: &EvalCalibrationSnapshot,
    history: &EvalQualityGateHistory,
    policy: &EvalQualityGatePolicy,
) -> EvalQualityGateState {
    let required = policy.required_consecutive_passes.max(1);
    let quality_signal_sufficient = !matches!(
        signals.evaluation_mode,
        EvalQualitySignalMode::InsufficientSignal
    ) && (signals.minimum_eval_samples == 0
        || signals.predicted_non_info_samples >= signals.minimum_eval_samples);
    let calibration_ready = calibration.calibration_ready;
    let hard_pass =
        signals.quality_ok && signals.monitor_ok && quality_signal_sufficient && calibration_ready;
    let soft_blocked = signals.quality_ok
        && signals.monitor_ok
        && (!quality_signal_sufficient || !calibration_ready);
    let consecutive_passes = if hard_pass {
        history.consecutive_passes.saturating_add(1)
    } else if soft_blocked {
        history.consecutive_passes
    } else {
        0
    };
    let autonomous_escalation_allowed = hard_pass && consecutive_passes >= required;
    let remaining_to_unlock = required.saturating_sub(consecutive_passes);
    EvalQualityGateState {
        quality_signal_sufficient,
        calibration_ready,
        current_pass: hard_pass,
        soft_blocked,
        consecutive_passes,
        required_consecutive_passes: required,
        autonomous_escalation_allowed,
        remaining_to_unlock,
    }
}

pub fn evaluate_judge_human_agreement(
    rows: &[EvalJudgeHumanComparableRow],
    policy: &EvalJudgeHumanAgreementPolicy,
) -> EvalJudgeHumanAgreementResult {
    let mut pair_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut matches = 0_u64;
    for row in rows {
        let pair_key = format!("{:?}->{:?}", row.judge_verdict, row.human_verdict).to_lowercase();
        *pair_counts.entry(pair_key).or_insert(0) += 1;
        if row.judge_verdict == row.human_verdict {
            matches = matches.saturating_add(1);
        }
    }

    let comparable_samples = rows.len() as u64;
    let agreement_rate = if comparable_samples == 0 {
        0.0
    } else {
        matches as f64 / comparable_samples as f64
    };
    let insufficient_signal = comparable_samples < policy.minimum_samples;
    let calibration_ready = !insufficient_signal && agreement_rate >= policy.agreement_min;
    let status = if insufficient_signal {
        "insufficient_signal".to_string()
    } else if calibration_ready {
        "calibrated".to_string()
    } else {
        "agreement_below_threshold".to_string()
    };

    let summary = EvalJudgeHumanAgreementSummary {
        comparable_samples,
        minimum_samples: policy.minimum_samples,
        agreement_rate,
        agreement_min: policy.agreement_min,
        calibration_ready,
        status,
        pair_counts,
    };

    let ok = insufficient_signal || calibration_ready;
    EvalJudgeHumanAgreementResult { ok, summary }
}
