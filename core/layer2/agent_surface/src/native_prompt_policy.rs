// Layer ownership: Core Layer 2 (Scheduling + Execution) - native prompt policy bridge.
use crate::native_evidence::{
    native_tool_changed_paths, native_tool_evidence_target_brief,
    native_tool_failed_validation_receipt_details, native_tool_prompt_checkpoint_name,
    native_tool_prompt_expected_memory_row_id, native_tool_prompt_memory_cli_pattern,
    native_tool_successful_receipt_refs,
};
use crate::native_tools::NativeToolReceipt;
use crate::native_workflow_artifact::native_tool_workflow_artifact_memory_tags;
use serde_json::{json, Value};

pub(crate) fn native_tool_initial_prompt(original_prompt: &str, metadata: &Value) -> String {
    let criteria = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"));
    let evidence_target_brief = native_tool_evidence_target_brief(original_prompt);
    let requires_native_tool_use = criteria
        .and_then(|value| value.get("requires_native_tool_use"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let force_discovery_first = criteria
        .and_then(|value| value.get("force_discovery_first_turn"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let force_read_first = criteria
        .and_then(|value| value.get("force_read_first_turn"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if force_discovery_first {
        return format!(
            "{original_prompt}\n\nNative tool-use initiation rule: before planning, editing, or final answering, return only JSON with a tool_calls array that discovers the local project shape. Start with file_list on the local project root or directory implied by the task. Use file_stat before reading any target path that may not exist. After discovery observations, classify the work as create, edit, extend, debug, or refactor; then read only relevant existing context files before writing or patching. For create/new-file tasks, do not file_read the target file unless file_stat or file_list shows it exists. If the task requires mutation, do not repeat discovery/read-only turns after sufficient context; transition to file_write/file_patch or return a structured blocker. Do not produce prose, analysis, or a final answer until native discovery observations are returned.{evidence_target_brief}"
        );
    }
    if !force_read_first {
        if requires_native_tool_use {
            let rule = native_tool_orchestration_prompt_text(
                metadata,
                "initial_tool_use_rule",
                "Native tool-use rule: choose the shortest safe native file-tool path for the task. Use discovery for unclear existing-project work, mutate only after enough context is available, validate after edits when requested, and do not claim success until native receipts prove the required file mutation and validation outcomes.",
            );
            return format!(
                "{original_prompt}\n\n{rule}{evidence_target_brief}"
            );
        }
        return original_prompt.to_string();
    }
    format!(
        "{original_prompt}\n\nNative tool-use initiation rule: before planning or final answering, return only JSON with a tool_calls array that reads existing local context files relevant to the task. If a target may not exist yet, use file_stat first rather than file_read. Prefer file_read_many when multiple existing files are known. Do not produce prose, analysis, or a final answer until native file-read observations are returned.{evidence_target_brief}"
    )
}

pub(crate) fn native_tool_orchestration_prompt_text(
    metadata: &Value,
    key: &str,
    fallback: &str,
) -> String {
    metadata
        .get("native_runtime_prompt_policy")
        .or_else(|| metadata.pointer("/workflow/native_runtime_prompt_policy"))
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| fallback.to_string())
}

pub(crate) fn native_tool_completion_evidence_repair_prompt(
    metadata: &Value,
    original_prompt: &str,
    previous_output: &str,
    receipts: &[NativeToolReceipt],
    repair_reasons: &[String],
) -> String {
    let changed_paths = native_tool_changed_paths(receipts);
    let receipt_refs = native_tool_successful_receipt_refs(receipts);
    let evidence_target_brief = native_tool_evidence_target_brief(original_prompt);
    let failed_validation_details = native_tool_failed_validation_receipt_details(receipts);
    let repair_actions =
        native_tool_completion_repair_action_brief(metadata, original_prompt, repair_reasons);
    let repair_rule = native_tool_orchestration_prompt_text(
        metadata,
        "completion_evidence_repair_prompt_rule",
        "This is a bounded continuation of the same native tool task. Repair only the listed uncovered requirements. Return JSON tool calls while repairing, or a structured blocker only when local completion is genuinely blocked.",
    );
    format!(
        "{}\n\nOriginal task:\n{}\n{}\n\nReceipt-backed changed files so far:\n{}\n\nSuccessful receipt refs:\n{}\n\nFailed validation receipt details:\n{}\n\nUncovered requirements detected by the runtime:\n{}\n\nRequired repair actions:\n{}\n\nPrevious output preview:\n{}",
        repair_rule,
        original_prompt.chars().take(2600).collect::<String>(),
        evidence_target_brief,
        changed_paths.join("\n"),
        receipt_refs.join("\n"),
        failed_validation_details,
        repair_reasons.join("\n"),
        repair_actions,
        previous_output.chars().take(1400).collect::<String>()
    )
}

pub(crate) fn native_tool_completion_repair_action_brief(
    metadata: &Value,
    original_prompt: &str,
    repair_reasons: &[String],
) -> String {
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "completion_repair_action_rule",
        "Continue from the existing native tool work and produce the smallest missing final deliverable. Do not restart or rewrite unrelated work.",
    );
    let target_paths = repair_reasons
        .iter()
        .filter_map(|reason| reason.strip_prefix("missing_changed_path:"))
        .collect::<Vec<_>>();
    let expected_row = repair_reasons
        .iter()
        .find_map(|reason| {
            reason
                .strip_prefix("missing_memory_write_receipt:")
                .filter(|value| !value.is_empty())
        })
        .map(str::to_string)
        .or_else(|| native_tool_prompt_expected_memory_row_id(original_prompt));
    let cli_pattern = native_tool_prompt_memory_cli_pattern(original_prompt);
    let tags = native_tool_workflow_artifact_memory_tags(metadata).join(",");
    let completed_checkpoint = native_tool_prompt_checkpoint_name(original_prompt);
    format!(
        "{rule}\n\nUncovered items:\n{}\n\nPrompt-derived target paths:\n{}\n\nExpected memory row:\n{}\n\nMemory CLI pattern:\n{}\n\nMemory tags:\n{}\n\nPrompt-derived completed checkpoint:\n{}",
        repair_reasons.join("\n"),
        target_paths.join("\n"),
        expected_row.unwrap_or_else(|| "<none>".to_string()),
        cli_pattern.unwrap_or_else(|| "<none>".to_string()),
        tags,
        completed_checkpoint.unwrap_or_else(|| "<none>".to_string())
    )
}

pub(crate) fn native_tool_public_reasoning_metadata(metadata: &Value) -> Value {
    let mut out = metadata.clone();
    if let Some(object) = out.as_object_mut() {
        object.insert("provider_timeout_seconds".to_string(), json!(90));
        object.insert("native_public_reasoning_finalization".to_string(), json!(true));
    }
    out
}

pub(crate) fn native_tool_public_reasoning_finalization_prompt(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
    previous_output: &str,
) -> String {
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "public_reasoning_finalization_rule",
        "Produce a concise receipt-backed final user response for this native coding workflow attempt. Include status, changed files when known, validation status, blockers when present, and the next useful step. Do not expose hidden chain-of-thought.",
    );
    format!(
        "{rule}\n\nOriginal task summary:\n{}\n\nNative tool receipt count: {}\n\nPrevious non-final output preview:\n{}",
        original_prompt.chars().take(2400).collect::<String>(),
        receipts.len(),
        previous_output.chars().take(1200).collect::<String>()
    )
}

pub(crate) fn native_tool_recovery_prompt(
    metadata: &Value,
    original_prompt: &str,
    reason: &str,
    changed_paths: &[String],
    receipts: &[NativeToolReceipt],
) -> String {
    let receipt_refs = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .map(|receipt| {
            let path = receipt
                .result
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("");
            format!("{}:{}:{}", receipt.call_id, receipt.tool_name, path)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let recovery_rule = native_tool_orchestration_prompt_text(
        metadata,
        "partial_progress_recovery_rule",
        "Run a bounded recovery pass using receipt-backed changed files as context. Fix only obvious local errors inside those files, do not expand scope, and provide the final user response if no safe repair remains.",
    );
    format!(
        "{recovery_rule}\n\nRecovery reason:\n{reason}\n\nOriginal task:\n{}\n\nReceipt-backed changed files:\n{}\n\nSuccessful receipt refs:\n{}",
        original_prompt.chars().take(2400).collect::<String>(),
        changed_paths.join("\n"),
        receipt_refs
    )
}

pub(crate) fn native_tool_empty_retry_prompt(
    metadata: &Value,
    original_prompt: &str,
    previous_output: &str,
    retry: u64,
) -> String {
    let previous = previous_output.trim();
    let evidence_target_brief = native_tool_evidence_target_brief(original_prompt);
    let previous = if previous.is_empty() {
        "The previous response was empty.".to_string()
    } else {
        format!(
            "Previous response without native tool calls:\n{}",
            previous.chars().take(1200).collect::<String>()
        )
    };
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "empty_tool_retry_rule",
        "This run requires native tool receipts before completion. Return only JSON with a tool_calls array now, or return a structured blocker only if local files, permissions, or missing user information genuinely prevent mutation.",
    );
    format!(
        "{original_prompt}\n\nNative tool retry {retry}: {rule}\n\n{previous}{evidence_target_brief}"
    )
}

pub(crate) fn native_tool_context_to_mutation_retry_prompt(
    metadata: &Value,
    original_prompt: &str,
    previous_output: &str,
    observations: &str,
    retry: u64,
) -> String {
    let previous = previous_output.trim();
    let previous = if previous.is_empty() {
        "The previous response had no native tool calls.".to_string()
    } else {
        format!(
            "Previous response without mutation tool calls:\n{}",
            previous.chars().take(1200).collect::<String>()
        )
    };
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "context_to_mutation_transition_rule",
        "Native mutation transition retry: local context already exists, but no successful file_write or file_patch receipt exists yet. Return only JSON tool calls for the next safe mutation batch, then validate when requested. Return a structured blocker only when local context proves mutation is unsafe or impossible.",
    );
    format!(
        "{original_prompt}\n\n{rule}\n\nRetry: {retry}\n\n{previous}\n\nNative tool observations already available:\n{observations}"
    )
}
