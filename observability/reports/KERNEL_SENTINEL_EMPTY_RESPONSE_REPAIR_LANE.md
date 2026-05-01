# Kernel Sentinel Empty-Response Repair Lane

Owner: Observability with Validation proof support.

This lane turns repeated empty-assistant-response findings into repairable issue candidates only when the failure has enough finalization evidence to avoid symptom patching.

## Release policy

- Human review is required before filing or promoting work.
- Auto-apply is forbidden.
- System-authored visible fallback text is forbidden.
- Findings must pass the Sentinel quality filter before promotion.

## Symptom signatures

- `empty_assistant_response`
- `final_llm_empty`
- `tool_pending_without_final_synthesis`
- `system_fallback_suppressed_without_operator_diagnostic`

## Acceptance criteria

- Every promoted empty-response finding carries a finalization phase, failed condition, evidence refs, recurrence count, freshness tier, owner guess, root-cause hypothesis, and concrete next action.
- Pending tool-menu progress is not counted as tool execution or final answer synthesis.
- Suppressed system fallback text remains diagnostics-only and never becomes visible chat content.
- Repeated empty-response symptoms cluster by finalization phase and failure signature before TODO or issue promotion.

## Validation commands

```bash
cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_visibility_diagnostics_classify_empty_llm_reply_without_system_injection -- --nocapture
cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_self_play_empty_reply_keeps_trace_diagnostic_from_beginning_to_end -- --nocapture
npm run -s ops:ksent:empty-response-repair:guard
```
