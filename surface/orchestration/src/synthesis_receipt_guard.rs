// Layer ownership: surface/orchestration (non-canonical synthesis gating metadata only).
use infring_tooling_core_v1::{ToolExecutionReceipt, ToolExecutionReceiptStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynthesisToolEvidenceLine {
    pub tool_id: String,
    pub status: ToolExecutionReceiptStatus,
    pub error_code: Option<String>,
    pub data_ref: Option<String>,
    pub evidence_count: usize,
    pub claimable_success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynthesisEvidenceSummary {
    pub guard_status: String,
    pub total_receipts: usize,
    pub claimable_success_receipts: usize,
    pub blocked_receipts: usize,
    pub error_receipts: usize,
    pub zero_evidence_success_receipts: usize,
    pub usable_evidence_count: usize,
    pub issue_codes: Vec<String>,
    pub assistant_visibility_contract: String,
    pub tools: Vec<SynthesisToolEvidenceLine>,
}

pub fn summarize_tool_receipts_for_synthesis(
    receipts: &[ToolExecutionReceipt],
) -> SynthesisEvidenceSummary {
    let tools = receipts.iter().map(tool_evidence_line).collect::<Vec<_>>();
    let claimable_success_receipts = tools.iter().filter(|row| row.claimable_success).count();
    let blocked_receipts = tools
        .iter()
        .filter(|row| row.status == ToolExecutionReceiptStatus::Blocked)
        .count();
    let error_receipts = tools
        .iter()
        .filter(|row| row.status == ToolExecutionReceiptStatus::Error)
        .count();
    let zero_evidence_success_receipts = tools
        .iter()
        .filter(|row| row.status == ToolExecutionReceiptStatus::Success && row.evidence_count == 0)
        .count();
    let usable_evidence_count = tools
        .iter()
        .filter(|row| row.claimable_success)
        .map(|row| row.evidence_count)
        .sum::<usize>();
    let mut issue_codes = Vec::<String>::new();
    if receipts.is_empty() {
        issue_codes.push("tool_receipt_missing".to_string());
    }
    if blocked_receipts > 0 {
        issue_codes.push("tool_blocked".to_string());
    }
    if error_receipts > 0 {
        issue_codes.push("tool_error".to_string());
    }
    if zero_evidence_success_receipts > 0 {
        issue_codes.push("zero_evidence_success_receipt".to_string());
    }
    issue_codes.extend(
        tools
            .iter()
            .filter_map(|row| row.error_code.as_ref())
            .map(|code| format!("tool_error_code:{code}")),
    );
    issue_codes.sort();
    issue_codes.dedup();
    let guard_status = if claimable_success_receipts > 0 {
        "ready".to_string()
    } else {
        "blocked_until_success_receipt_with_evidence".to_string()
    };
    SynthesisEvidenceSummary {
        guard_status,
        total_receipts: receipts.len(),
        claimable_success_receipts,
        blocked_receipts,
        error_receipts,
        zero_evidence_success_receipts,
        usable_evidence_count,
        issue_codes,
        assistant_visibility_contract:
            "LLM final output may claim tool success only for claimable_success receipts; all other receipts are no usable evidence if mentioned."
                .to_string(),
        tools,
    }
}

pub fn synthesis_may_claim_tool_success(summary: &SynthesisEvidenceSummary) -> bool {
    summary.claimable_success_receipts > 0 && summary.usable_evidence_count > 0
}

fn tool_evidence_line(receipt: &ToolExecutionReceipt) -> SynthesisToolEvidenceLine {
    let claimable_success = receipt.status == ToolExecutionReceiptStatus::Success
        && receipt.evidence_count > 0
        && receipt.data_ref.is_some();
    SynthesisToolEvidenceLine {
        tool_id: receipt.tool_id.clone(),
        status: receipt.status.clone(),
        error_code: receipt.error_code.clone(),
        data_ref: receipt.data_ref.clone(),
        evidence_count: receipt.evidence_count,
        claimable_success,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(
        status: ToolExecutionReceiptStatus,
        evidence_count: usize,
        error_code: Option<&str>,
    ) -> ToolExecutionReceipt {
        ToolExecutionReceipt {
            attempt_id: "attempt".to_string(),
            trace_id: "trace".to_string(),
            task_id: "task".to_string(),
            status,
            tool_id: "web_search".to_string(),
            input_hash: "input".to_string(),
            started_at: 1,
            ended_at: 2,
            latency_ms: 1,
            error_code: error_code.map(str::to_string),
            data_ref: (evidence_count > 0).then(|| "raw://ok".to_string()),
            evidence_count,
            receipt_hash: "hash".to_string(),
        }
    }

    #[test]
    fn missing_or_failed_receipts_block_synthesis_success_claims() {
        let empty = summarize_tool_receipts_for_synthesis(&[]);
        assert!(!synthesis_may_claim_tool_success(&empty));
        assert!(empty
            .issue_codes
            .contains(&"tool_receipt_missing".to_string()));

        let errored = summarize_tool_receipts_for_synthesis(&[receipt(
            ToolExecutionReceiptStatus::Error,
            0,
            Some("anti_bot_challenge"),
        )]);
        assert!(!synthesis_may_claim_tool_success(&errored));
        assert!(errored
            .issue_codes
            .contains(&"tool_error_code:anti_bot_challenge".to_string()));
    }

    #[test]
    fn success_receipt_needs_usable_evidence_to_be_claimable() {
        let zero = summarize_tool_receipts_for_synthesis(&[receipt(
            ToolExecutionReceiptStatus::Success,
            0,
            None,
        )]);
        assert!(!synthesis_may_claim_tool_success(&zero));
        assert_eq!(zero.zero_evidence_success_receipts, 1);

        let ready = summarize_tool_receipts_for_synthesis(&[receipt(
            ToolExecutionReceiptStatus::Success,
            2,
            None,
        )]);
        assert!(synthesis_may_claim_tool_success(&ready));
        assert_eq!(ready.guard_status, "ready");
        assert_eq!(ready.usable_evidence_count, 2);
    }
}
