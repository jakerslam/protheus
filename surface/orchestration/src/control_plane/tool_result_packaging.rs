// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolReceiptStatus {
    Success,
    Error,
    Blocked,
    LowSignal,
    NoOutput,
    Running,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolReceiptProjection {
    pub tool_name: String,
    pub status: ToolReceiptStatus,
    pub attempt_id: Option<String>,
    pub output_present: bool,
    pub normalized_summary_present: bool,
    pub backend_receipt_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolResultPackageAction {
    RenderToolReceipt {
        status: ToolReceiptStatus,
        display_state: String,
    },
    HoldForCompletion {
        reason: String,
    },
    PackageEmptyAssistantWithReceipts {
        receipt_count: usize,
    },
    EscalateLowSignal {
        tool_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultPackage {
    pub action: ToolResultPackageAction,
    pub authoritative_receipt_count: usize,
    pub visible_chat_text_allowed: bool,
    pub telemetry_note: String,
}

pub struct ToolResultPackagingContract;

impl SubdomainContract for ToolResultPackagingContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "tool_result_packaging",
        legacy_module_bindings: &[
            "chat_assistant_text_signal_helpers",
            "chat_response_tool_payload_helpers",
            "chat_ws_tool_event_helpers",
            "chat_tool_summary_helpers",
            "chat_tool_card_helpers",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "tool_receipt_projection",
            "tool_result_package_projection",
            "tool_low_signal_recovery_recommendation",
        ],
        message_boundaries: &[
            "tool_result_to_shell_projection_boundary",
            "tool_result_to_synthesis_boundary",
            "tool_result_to_recovery_boundary",
        ],
    }
}

pub fn package_tool_receipts(
    assistant_text_present: bool,
    receipts: &[ToolReceiptProjection],
) -> ToolResultPackage {
    let authoritative = receipts
        .iter()
        .filter(|receipt| receipt.backend_receipt_present)
        .count();
    if let Some(running) = receipts
        .iter()
        .find(|receipt| receipt.status == ToolReceiptStatus::Running)
    {
        return ToolResultPackage {
            action: ToolResultPackageAction::HoldForCompletion {
                reason: format!("tool still running: {}", normalized_tool_name(running)),
            },
            authoritative_receipt_count: authoritative,
            visible_chat_text_allowed: assistant_text_present,
            telemetry_note: "hold result packaging until running tool completes".to_string(),
        };
    }

    if let Some(low_signal) = receipts
        .iter()
        .find(|receipt| receipt.status == ToolReceiptStatus::LowSignal)
    {
        return ToolResultPackage {
            action: ToolResultPackageAction::EscalateLowSignal {
                tool_name: normalized_tool_name(low_signal),
            },
            authoritative_receipt_count: authoritative,
            visible_chat_text_allowed: assistant_text_present,
            telemetry_note: "low-signal tool output should route through recovery telemetry"
                .to_string(),
        };
    }

    if !assistant_text_present && authoritative > 0 {
        return ToolResultPackage {
            action: ToolResultPackageAction::PackageEmptyAssistantWithReceipts {
                receipt_count: authoritative,
            },
            authoritative_receipt_count: authoritative,
            visible_chat_text_allowed: false,
            telemetry_note: "empty assistant text is represented by structured tool receipts"
                .to_string(),
        };
    }

    if let Some(receipt) = receipts.iter().find(|receipt| {
        matches!(
            receipt.status,
            ToolReceiptStatus::Blocked | ToolReceiptStatus::Error | ToolReceiptStatus::NoOutput
        )
    }) {
        return ToolResultPackage {
            action: ToolResultPackageAction::RenderToolReceipt {
                status: receipt.status.clone(),
                display_state: display_state(receipt),
            },
            authoritative_receipt_count: authoritative,
            visible_chat_text_allowed: assistant_text_present,
            telemetry_note: "render normalized tool receipt projection".to_string(),
        };
    }

    let status = receipts
        .first()
        .map(|receipt| receipt.status.clone())
        .unwrap_or(ToolReceiptStatus::NoOutput);
    ToolResultPackage {
        action: ToolResultPackageAction::RenderToolReceipt {
            status,
            display_state: "ready".to_string(),
        },
        authoritative_receipt_count: authoritative,
        visible_chat_text_allowed: assistant_text_present,
        telemetry_note: "render successful tool receipt projection".to_string(),
    }
}

fn display_state(receipt: &ToolReceiptProjection) -> String {
    match receipt.status {
        ToolReceiptStatus::Blocked => "blocked".to_string(),
        ToolReceiptStatus::Error => "error".to_string(),
        ToolReceiptStatus::NoOutput => "no_output".to_string(),
        ToolReceiptStatus::LowSignal => "low_signal".to_string(),
        ToolReceiptStatus::Running => "running".to_string(),
        ToolReceiptStatus::Success => {
            if receipt.output_present || receipt.normalized_summary_present {
                "ready".to_string()
            } else {
                "no_output".to_string()
            }
        }
    }
}

fn normalized_tool_name(receipt: &ToolReceiptProjection) -> String {
    let trimmed = receipt.tool_name.trim();
    if trimmed.is_empty() {
        "tool".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(status: ToolReceiptStatus) -> ToolReceiptProjection {
        ToolReceiptProjection {
            tool_name: "web_search".to_string(),
            status,
            attempt_id: Some("attempt-1".to_string()),
            output_present: true,
            normalized_summary_present: true,
            backend_receipt_present: true,
        }
    }

    #[test]
    fn success_receipt_renders_normalized_projection() {
        let package = package_tool_receipts(true, &[receipt(ToolReceiptStatus::Success)]);

        assert_eq!(
            package.action,
            ToolResultPackageAction::RenderToolReceipt {
                status: ToolReceiptStatus::Success,
                display_state: "ready".to_string()
            }
        );
        assert!(package.visible_chat_text_allowed);
        assert_eq!(package.authoritative_receipt_count, 1);
    }

    #[test]
    fn error_receipt_is_structured_not_shell_inferred() {
        let package = package_tool_receipts(true, &[receipt(ToolReceiptStatus::Error)]);

        assert_eq!(
            package.action,
            ToolResultPackageAction::RenderToolReceipt {
                status: ToolReceiptStatus::Error,
                display_state: "error".to_string()
            }
        );
    }

    #[test]
    fn blocked_receipt_stays_receipt_state() {
        let package = package_tool_receipts(true, &[receipt(ToolReceiptStatus::Blocked)]);

        assert_eq!(
            package.action,
            ToolResultPackageAction::RenderToolReceipt {
                status: ToolReceiptStatus::Blocked,
                display_state: "blocked".to_string()
            }
        );
    }

    #[test]
    fn low_signal_receipt_escalates_to_recovery_telemetry() {
        let package = package_tool_receipts(true, &[receipt(ToolReceiptStatus::LowSignal)]);

        assert_eq!(
            package.action,
            ToolResultPackageAction::EscalateLowSignal {
                tool_name: "web_search".to_string()
            }
        );
        assert!(package.telemetry_note.contains("low-signal"));
    }

    #[test]
    fn no_output_receipt_is_not_visible_chat_text() {
        let mut no_output = receipt(ToolReceiptStatus::NoOutput);
        no_output.output_present = false;
        no_output.normalized_summary_present = false;

        let package = package_tool_receipts(true, &[no_output]);

        assert_eq!(
            package.action,
            ToolResultPackageAction::RenderToolReceipt {
                status: ToolReceiptStatus::NoOutput,
                display_state: "no_output".to_string()
            }
        );
    }

    #[test]
    fn empty_assistant_with_receipts_uses_structured_receipts() {
        let package = package_tool_receipts(false, &[receipt(ToolReceiptStatus::Success)]);

        assert_eq!(
            package.action,
            ToolResultPackageAction::PackageEmptyAssistantWithReceipts { receipt_count: 1 }
        );
        assert!(!package.visible_chat_text_allowed);
    }
}
