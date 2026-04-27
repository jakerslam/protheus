# Workflow Quality Extension Boundary Policy

## Purpose

Preserve the split between generic orchestration runtime quality and workflow-family-specific doctrine. New workflow families (after ForgeCode) must extend `WorkflowQualitySignals` as new variants, not widen `RuntimeQualitySignals` with family-specific fields.

## Scope

Applies to any change that adds, modifies, or extends quality-signal fields surfaced through `OrchestrationResultPackage.runtime_quality` or `OrchestrationResultPackage.workflow_quality`.

## Two-Tier Schema

### Generic Tier — `RuntimeQualitySignals`

Anchor: `surface/orchestration/src/contracts.rs::RuntimeQualitySignals` (lines 623-645).

Allowed fields concern only workflow-agnostic orchestration quality:

- candidate counts, selected variant, executable / degraded / clarification breakdown
- probe state (`used_heuristic_probe`, `heuristic_probe_source_count`, `typed_probe_contract_gap_count`)
- precondition state (`blocked_precondition_count`)
- adapter / fallback state (`surface_adapter_fallback`, `fallback_action_count`)
- tool-failure-budget state (`tool_failure_budget_*`)
- aggregate signals about plans (`zero_executable_candidates`, `all_candidates_degraded`, `all_candidates_require_clarification`, `decision_rationale_count`)

Forbidden in `RuntimeQualitySignals`:

- workflow-family identifiers: `forgecode`, `openhands`, `codex_*`, `forge_*`, `sage_*`, `muse_*`, `assimilation_*` (when they refer to a specific workflow doctrine rather than generic ingest)
- workflow-specific doctrine: `mcp_*`, `subagent_*`, `step_checkpointing_*`, `completion_hygiene_*`, `parallel_independent_tool_calls_*`, etc.
- specific tool / route names

### Workflow Tier — `WorkflowQualitySignals`

Anchor: `surface/orchestration/src/contracts.rs::WorkflowQualitySignals` (lines 676-680).

Each workflow family lives behind its own enum variant:

```rust
#[serde(tag = "workflow", content = "signals", rename_all = "snake_case")]
pub enum WorkflowQualitySignals {
    ForgeCode(ForgeCodeWorkflowQualitySignals),
    // future: OpenHands(OpenHandsWorkflowQualitySignals),
    // future: ResearchSynthesizeVerify(ResearchSynthesizeVerifyWorkflowQualitySignals),
}
```

`OrchestrationResultPackage.workflow_quality: Option<WorkflowQualitySignals>` is emitted only when the selected workflow template matches the variant, so consumers must downcast on the workflow tag.

## Fail-Closed Rule

A pull request that:

- adds a workflow-family-specific field name to `RuntimeQualitySignals`,
- adds a workflow-family token (e.g., `forgecode_*`, `openhands_*`) to a generic-tier struct,
- or removes a `WorkflowQualitySignals` variant without retiring its consumers,

is not admissible. Workflow-specific doctrine MUST land as a new (or extended) `WorkflowQualitySignals` variant.

## Adding a New Workflow Family

1. Add a new variant to `WorkflowQualitySignals` with a snake_case tag (matches the `WorkflowTemplate` discriminant where reasonable).
2. Define a sibling struct `<Family>WorkflowQualitySignals` next to `ForgeCodeWorkflowQualitySignals`.
3. In `result_packaging.rs`, emit the new variant ONLY when the workflow template matches the family. Preserve the rule that `workflow_quality` is `None` for non-matching templates.
4. Add conformance tests in `surface/orchestration/tests/conformance/lifecycle_feedback.rs` mirroring the existing ForgeCode shape: at least one assertion per new variant verifying the variant is emitted and the generic tier remains family-free.
5. Mention the new family in this policy doc's "Variants" section below.

## Variants

| Family | Variant Name | Anchor Struct | Test |
|---|---|---|---|
| ForgeCode | `WorkflowQualitySignals::ForgeCode` | `ForgeCodeWorkflowQualitySignals` (`contracts.rs:648-674`) | `tests/conformance/lifecycle_feedback.rs:445-457` |

Add a row each time a new family lands.

## CI Guard Contract

Required guard (recommended location: extend `surface/orchestration/src/tool_routing_authority.rs` with a `runtime_quality_schema_workflow_clean()` check):

- Read the body of the `RuntimeQualitySignals` struct definition from `contracts.rs`.
- Assert no field name matches a workflow-family-specific token regex: `forgecode|openhands|forge_|sage_|muse_|mcp_|subagent_|codex_|assimilation_(?!ingest)|step_checkpoint|completion_hygiene|parallel_independent_tool_calls`.
- Fail closed when any forbidden token appears in `RuntimeQualitySignals`.
- Append the result row to the existing tool-routing-authority artifact at `core/local/artifacts/tool_routing_authority_guard_current.json`.

The guard is straightforward to add given the existing `token_check` pattern in `tool_routing_authority.rs` (line 1001) and the `RuntimeQualitySignals` struct can be matched with a regex over the source.

## Integration Notes

- This policy complements the existing diversity invariants in `lifecycle_feedback.rs` (see V11-EXT-CHATGPT-005) and the dedupe/threshold pipeline (see V11-EXT-CHATGPT-008).
- `ForgeCodeWorkflowQualitySignals` is the canonical shape: rich, family-specific, narrowly emitted only when the matching workflow template is selected. All future variants should follow the same shape.
- `WorkflowTemplate` enum in `contracts.rs:684-693` is the source of truth for which workflow families exist; new variants on `WorkflowQualitySignals` should typically match an entry there.
